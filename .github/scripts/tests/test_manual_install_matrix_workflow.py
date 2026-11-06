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
        before = {**after, "jobs": {k: v for k, v in after["jobs"].items() if not k.startswith("consumer_workflow")}}
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
        self.assertEqual(values["sc_publish_pin"]["revision"], "c619f225ebd1bfb3e20a59b91b1fc6e0e59fd07c")

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

    def test_gnullvm_toolchain_is_pinned_by_digest_in_the_go_job_only(self):
        step = next(s for s in self.jobs["go-native-matrix"]["steps"] if "LLVM-MinGW" in s.get("name", ""))
        self.assertEqual(len(step["env"]["LLVM_MINGW_SHA256"]), 64)
        self.assertIn("gnullvm", step["if"])
        # The Rust CLI gnullvm coupling is tracked separately (sc-publish#164), not worked around here.
        self.assertFalse([s for s in self.jobs["matrix-rust"]["steps"] if "LLVM-MinGW" in s.get("name", "")])


if __name__ == "__main__":
    unittest.main()
