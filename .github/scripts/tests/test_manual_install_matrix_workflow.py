"""Structure checks for the manual install-matrix producer in go-native-preflight.yml
and its route from the kit-registered release-preflight.yml.

Deterministic: parses the workflow and canonical contracts only; nothing runs.
"""

import sys
import tomllib
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import check_install_matrix as matrix  # noqa: E402

WORKFLOW = ROOT / ".github" / "workflows" / "go-native-preflight.yml"
PREFLIGHT = ROOT / ".github" / "workflows" / "release-preflight.yml"
MATRIX_USES = "./.github/workflows/go-native-preflight.yml"
CALL_JOB = "consumer_workflow_install-matrix"
BEFORE_PREFLIGHT = ROOT / ".github" / "scripts" / "tests" / "fixtures" / "release-preflight-before-matrix.yml"


def route_gaps(preflight: dict, matrix: dict) -> list[str]:
    """Why the registered Release Preflight does not run the full matrix with the candidate identity."""
    gaps = []
    jobs = preflight.get("jobs", {})
    call = jobs.get(CALL_JOB)
    if call is None:
        return ["release-preflight.yml has no job calling the install matrix"]
    if call.get("uses") != MATRIX_USES:
        gaps.append(f"call job uses {call.get('uses')!r}, not {MATRIX_USES}")
    if call.get("with") != {"version": "${{ inputs.version }}", "candidate_sha": "${{ github.sha }}"}:
        gaps.append("call job does not pass the dispatched version and github.sha")
    if "secrets" in call:
        gaps.append("call job passes secrets to the matrix")
    outcome = jobs.get("consumer_workflows", {})
    if CALL_JOB not in outcome.get("needs", []) or outcome.get("if") != "${{ always() }}":
        gaps.append("consumer_workflows does not record and deny on the call result")
    inputs = matrix.get(True, {}).get("workflow_call", {}).get("inputs", {})
    for name in ("version", "candidate_sha"):
        if inputs.get(name, {}).get("type") != "string":
            gaps.append(f"matrix workflow_call does not declare string input {name}")
    return gaps


class ManualInstallMatrixWorkflow(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        cls.jobs = cls.workflow["jobs"]

    def test_runs_only_by_manual_dispatch_or_the_registered_preflight_call(self):
        self.assertEqual(set(self.workflow[True]), {"workflow_dispatch", "workflow_call"})

    def test_registered_release_preflight_runs_the_matrix_with_candidate_identity(self):
        preflight = yaml.safe_load(PREFLIGHT.read_text(encoding="utf-8"))
        self.assertEqual(route_gaps(preflight, self.workflow), [])
        # The entrypoint itself stays manual: the matrix is never reachable from push/PR/schedule.
        self.assertEqual(set(preflight[True]), {"workflow_dispatch"})

    def test_before_route_is_disconnected_and_after_route_is_connected(self):
        after = yaml.safe_load(PREFLIGHT.read_text(encoding="utf-8"))
        before = yaml.safe_load(BEFORE_PREFLIGHT.read_text(encoding="utf-8"))
        self.assertEqual(sorted(before["jobs"]), ["preflight"])
        self.assertEqual(route_gaps(before, self.workflow), ["release-preflight.yml has no job calling the install matrix"])
        self.assertEqual(route_gaps(after, self.workflow), [])
        # Each identity/route property is individually load-bearing.
        wrong_ref = yaml.safe_load(PREFLIGHT.read_text(encoding="utf-8"))
        wrong_ref["jobs"][CALL_JOB]["with"]["candidate_sha"] = "${{ github.sha }}-x"
        self.assertEqual(route_gaps(wrong_ref, self.workflow), ["call job does not pass the dispatched version and github.sha"])
        no_inputs = {**self.workflow, True: {"workflow_dispatch": None}}
        self.assertEqual(len(route_gaps(after, no_inputs)), 2)

    def test_called_matrix_verifies_the_requested_candidate_before_building(self):
        plan = self.jobs["matrix-plan"]["steps"]
        step = next(s for s in plan if s.get("name") == "Verify the called candidate identity")
        self.assertLess(plan.index(step), next(i for i, s in enumerate(plan) if s.get("id") == "plan"))
        self.assertEqual(step["env"], {"INPUT_VERSION": "${{ inputs.version }}", "INPUT_CANDIDATE_SHA": "${{ inputs.candidate_sha }}"})
        for needle in ("git rev-parse HEAD", "INPUT_VERSION#v", "exit 1"):
            self.assertIn(needle, step["run"])

    def test_called_matrix_publishes_nothing(self):
        text = WORKFLOW.read_text(encoding="utf-8")
        for marker in ("gh release", "cargo publish", "twine upload", "maturin publish", "git push", "git tag"):
            self.assertNotIn(marker, text)
        self.assertEqual(self.workflow["permissions"], {"contents": "read"})

    def test_install_input_registers_the_matrix_workflow(self):
        import json

        values = json.loads((ROOT / "release" / "sc-publish-install.json").read_text(encoding="utf-8"))
        self.assertEqual([w["uses"] for w in values["preflight_workflows"]], [MATRIX_USES])

    def test_wheel_matrix_covers_each_distribution_on_six_coordinates(self):
        manifest = tomllib.loads((ROOT / "release" / "publish-artifacts.toml").read_text(encoding="utf-8"))
        declared = {d["name"] for d in manifest["python_distributions"]}
        strategy = self.jobs["matrix-wheels"]["strategy"]["matrix"]
        self.assertEqual({d["name"] for d in strategy["dist"]}, declared)
        for dist in strategy["dist"]:
            self.assertTrue((ROOT / dist["cargo_manifest"]).is_file())
        runners = {c["id"]: c["os"] for c in strategy["coordinate"]}
        self.assertEqual(
            set(runners),
            {"linux-x64", "linux-arm64", "macos-arm64", "macos-x64", "windows-x64", "windows-arm64"},
        )
        self.assertEqual(len(declared) * len(runners), 18)

    def test_collect_publishes_the_artifact_names_the_verifier_expects(self):
        steps = self.jobs["matrix-collect"]["steps"]
        uploaded = {s["with"]["name"] for s in steps if str(s.get("uses", "")).startswith("actions/upload-artifact")}
        self.assertEqual(uploaded, {matrix.FILES_ARTIFACT, matrix.CHECKSUMS_ARTIFACT})
        for step in steps:
            if str(step.get("uses", "")).startswith("actions/upload-artifact"):
                self.assertEqual(step["with"]["if-no-files-found"], "error")

    def test_each_wheel_is_installed_and_tested_before_the_upload(self):
        steps = self.jobs["matrix-wheels"]["steps"]
        names = [s.get("name", "") for s in steps]
        install = next(i for i, n in enumerate(names) if n.startswith("Install the exact wheel"))
        upload = next(i for i, s in enumerate(steps) if str(s.get("uses", "")).startswith("actions/upload-artifact"))
        builds = [i for i, n in enumerate(names) if n.startswith("Build")]
        self.assertTrue(builds and max(builds) < install < upload)
        for dist in self.jobs["matrix-wheels"]["strategy"]["matrix"]["dist"]:
            self.assertTrue((ROOT / dist["source"] / dist["regression"]).is_file())

    def test_each_cli_archive_is_extracted_and_run_before_the_upload(self):
        steps = self.jobs["matrix-rust"]["steps"]
        names = [s.get("name", "") for s in steps]
        smoke = names.index("Extract the exact archive and run the native CLI")
        upload = next(i for i, s in enumerate(steps) if str(s.get("uses", "")).startswith("actions/upload-artifact"))
        self.assertLess(names.index("Package manifest-declared release archive"), smoke)
        self.assertLess(smoke, upload)

    def test_collect_records_the_post_run_verifier_invocation(self):
        step = next(s for s in self.jobs["matrix-collect"]["steps"] if s.get("name", "").startswith("Record the post-run V7"))
        run = step["run"]
        for needle in ("producer-side only", "scripts/check_install_matrix.py", "--run", "--artifacts", "--checksums", "--staging", "--candidate-sha", "--version", "--workflow ${workflow_path}", "GITHUB_WORKFLOW_REF"):
            self.assertIn(needle, run)
        # No in-run identity assertion: the collect job never calls the verifier itself.
        for other in self.jobs["matrix-collect"]["steps"]:
            if other is not step:
                self.assertNotIn("check_install_matrix.py", other.get("run", ""))

    def test_cli_smoke_runs_version_help_and_a_render(self):
        step = next(s for s in self.jobs["matrix-rust"]["steps"] if s.get("name", "").startswith("Extract the exact archive"))
        for needle in ("--version", "--help", '"render"'):
            self.assertIn(needle, step["run"])

    def test_collect_waits_for_every_cell_producer(self):
        self.assertEqual(
            set(self.jobs["matrix-collect"]["needs"]),
            {"matrix-plan", "matrix-rust", "matrix-wheels", "go-native-matrix"},
        )

    def test_go_cells_upload_the_name_the_verifier_classifies(self):
        upload = [s for s in self.jobs["go-native-matrix"]["steps"] if str(s.get("uses", "")).startswith("actions/upload-artifact")]
        self.assertEqual(len(upload), 1)
        self.assertTrue(upload[0]["with"]["name"].startswith("matrix-cell-"))
        self.assertTrue(upload[0]["with"]["path"].endswith("${{ matrix.artifact_prefix }}-${{ matrix.target }}.zip"))

    def test_release_requires_the_reused_go_matrix_and_each_target(self):
        import copy
        import re

        release = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())
        jobs = release["jobs"]
        producer = jobs["build-go-bundles"]
        self.assertEqual(producer["uses"], MATRIX_USES)
        self.assertEqual(producer["needs"], "gate-and-tag")
        self.assertEqual(producer["with"]["candidate_sha"], "${{ needs.gate-and-tag.outputs['build_ref'] }}")
        self.assertIn("build-go-bundles", jobs["build-npm"]["needs"])
        self.assertIn("needs.build-go-bundles.result == 'success'", jobs["build-npm"]["if"])
        self.assertIn("build-npm", jobs["release"]["needs"])
        self.assertIn("needs.build-npm.result == 'success'", jobs["release"]["if"])
        broken = copy.deepcopy(release)
        del broken["jobs"]["build-go-bundles"]
        self.assertNotIn("build-go-bundles", broken["jobs"])
        steps = jobs["release"]["steps"]
        probe = next(step for step in steps if step.get("id") == "expected_assets")
        require = next(step for step in steps if step.get("name") == "Collect archives")
        upload = next(i for i, step in enumerate(steps) if step.get("name") == "Create GitHub Release")
        self.assertLess(steps.index(require), upload)
        self.assertEqual(require["env"]["REQUIRE_GO_BUNDLES"], "true")
        contract = tomllib.loads((ROOT / "bindings/sc-sha-go/native/targets.toml").read_text())
        targets = {entry["rust_target"] for entry in contract["targets"]}
        self.assertEqual(len(targets), 6)
        for target in targets:
            self.assertIn(f"'{target}'", require["run"])
            self.assertIn(f"sc-sha-go-{target}\\.zip", probe["run"])
        self.assertEqual(len(re.findall(r"echo '\^sc-sha-go-", probe["run"])), 6)

    def test_release_go_collection_refuses_missing_target_before_copy(self):
        import os
        import subprocess
        import tempfile

        workflow = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())
        steps = workflow["jobs"]["release"]["steps"]
        script = next(step["run"] for step in steps if step.get("name") == "Collect archives")
        script = script.split("python3 - <<'PYCODE'\n", 1)[1].rsplit("\nPYCODE", 1)[0]
        contract = tomllib.loads((ROOT / "bindings/sc-sha-go/native/targets.toml").read_text())
        names = [f"sc-sha-go-{entry['rust_target']}.zip" for entry in contract["targets"]]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "release-artifacts"
            destination = root / "release-assets"
            source.mkdir()
            destination.mkdir()
            for name in names[:-1]:
                cell = source / f"matrix-cell-go-{name.removeprefix('sc-sha-go-').removesuffix('.zip')}"
                cell.mkdir()
                (cell / name).write_bytes(b"bundle")
            env = {**os.environ, "RELEASE_DOWNLOAD_DIR": str(source), "RELEASE_ASSET_DIR": str(destination)}
            missing = subprocess.run([sys.executable, "-c", script], env=env, capture_output=True, text=True, check=False)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn(names[-1], missing.stderr)
            self.assertEqual(list(destination.iterdir()), [])
            cell = source / f"matrix-cell-go-{names[-1].removeprefix('sc-sha-go-').removesuffix('.zip')}"
            cell.mkdir()
            (cell / names[-1]).write_bytes(b"bundle")
            complete = subprocess.run([sys.executable, "-c", script], env=env, capture_output=True, text=True, check=False)
            self.assertEqual(complete.returncode, 0, complete.stderr)
            self.assertEqual({p.name for p in destination.iterdir()}, set(names))

    def test_release_go_tag_is_created_reused_and_conflicts_fail(self):
        import os
        import subprocess
        import tempfile

        workflow = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())
        steps = workflow["jobs"]["gate-and-tag"]["steps"]
        step = next(step for step in steps if step.get("id") == "release-ref")
        self.assertEqual(step["env"]["SC_GO_MODULE_TAG_PREFIX"], "bindings/sc-sha-go")
        self.assertGreater(step["run"].index("module_tag="), step["run"].index('git push origin "$tag"'))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            remote = root / "origin.git"
            repo = root / "repo"
            subprocess.run(["git", "init", "--bare", "-q", str(remote)], check=True)
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            def git(*args):
                return subprocess.run(["git", *args], cwd=repo, check=True, text=True, capture_output=True).stdout.strip()
            git("config", "user.name", "Release Test")
            git("config", "user.email", "test@example.invalid")
            git("config", "commit.gpgsign", "false")
            git("remote", "add", "origin", str(remote))
            git("commit", "--allow-empty", "-qm", "first")
            first = git("rev-parse", "HEAD")
            git("commit", "--allow-empty", "-qm", "release")
            release = git("rev-parse", "HEAD")
            git("tag", "release-candidate-v1.9.0")
            git("push", "-q", "origin", "HEAD:refs/heads/main", "refs/tags/release-candidate-v1.9.0")
            env = {**os.environ, "SC_GO_MODULE_TAG_PREFIX": "bindings/sc-sha-go", "GITHUB_OUTPUT": str(root / "output")}
            script = (step["run"]
                .replace("'${{ steps.meta.outputs.release_tag }}'", "'v1.9.0'")
                .replace("'${{ steps.meta.outputs.release_target }}'", "'production'")
                .replace("'${{ steps.release_gate.outputs.release_sha }}'", repr(release)))
            def execute():
                return subprocess.run(["bash", "-c", script], cwd=repo, env=env, capture_output=True, text=True, check=False)
            created = execute()
            self.assertEqual(created.returncode, 0, created.stderr)
            self.assertEqual(git("rev-parse", "bindings/sc-sha-go/v1.9.0^{commit}"), release)
            self.assertEqual(execute().returncode, 0)
            git("tag", "-f", "bindings/sc-sha-go/v1.9.0", first)
            git("push", "-q", "--force", "origin", "refs/tags/bindings/sc-sha-go/v1.9.0")
            conflict = execute()
            self.assertNotEqual(conflict.returncode, 0)
            self.assertIn("expected", conflict.stderr)
            self.assertEqual(git("rev-parse", "bindings/sc-sha-go/v1.9.0^{commit}"), first)

    def test_gnullvm_toolchain_is_pinned_by_digest_in_the_go_job_only(self):
        step = next(s for s in self.jobs["go-native-matrix"]["steps"] if "LLVM-MinGW" in s.get("name", ""))
        self.assertEqual(len(step["env"]["LLVM_MINGW_SHA256"]), 64)
        self.assertIn("gnullvm", step["if"])
        # The Rust CLI gnullvm coupling is tracked separately (sc-publish#164), not worked around here.
        self.assertFalse([s for s in self.jobs["matrix-rust"]["steps"] if "LLVM-MinGW" in s.get("name", "")])


if __name__ == "__main__":
    unittest.main()
