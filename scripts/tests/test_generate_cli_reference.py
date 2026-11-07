"""Wrapper tests for scripts/generate_cli_reference.py (real docli, fixture model)."""

import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "generate_cli_reference.py"
MODEL = ROOT / "tests" / "fixtures" / "cli-reference" / "reference.json"


def run(out: Path, mode: str, model: Path = MODEL, docli: str = "docli"):
    return subprocess.run(
        [sys.executable, str(SCRIPT), "--output-dir", str(out), mode, "--model-json", str(model), "--docli", docli],
        capture_output=True,
        text=True,
        check=False,
    )


class GenerateCliReferenceTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.out = Path(self._tmp.name) / "out"

    def test_write_produces_html_and_markdown_then_check_passes(self):
        done = run(self.out, "--write")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertTrue((self.out / "html" / "index.html").is_file())
        self.assertIn("sc-compose", (self.out / "cli-reference.md").read_text(encoding="utf-8"))
        self.assertEqual(run(self.out, "--check").returncode, 0)

    def test_check_refuses_stale_markdown_and_write_recovers(self):
        self.assertEqual(run(self.out, "--write").returncode, 0)
        md = self.out / "cli-reference.md"
        md.write_text(md.read_text(encoding="utf-8") + "stale\n", encoding="utf-8")
        stale = run(self.out, "--check")
        self.assertEqual(stale.returncode, 1)
        self.assertIn("cli-reference.md", stale.stderr)
        self.assertEqual(run(self.out, "--write").returncode, 0)
        self.assertEqual(run(self.out, "--check").returncode, 0)

    def test_check_refuses_stale_html_missing_and_extra_files(self):
        self.assertEqual(run(self.out, "--write").returncode, 0)
        index = self.out / "html" / "index.html"
        original = index.read_text(encoding="utf-8")
        index.write_text(original + "<!-- stale -->", encoding="utf-8")
        self.assertEqual(run(self.out, "--check").returncode, 1)
        index.write_text(original, encoding="utf-8")
        (self.out / "html" / "extra.txt").write_text("x", encoding="utf-8")
        self.assertEqual(run(self.out, "--check").returncode, 1)
        (self.out / "html" / "extra.txt").unlink()
        index.unlink()
        self.assertEqual(run(self.out, "--check").returncode, 1)

    def test_check_never_rewrites_expected_files(self):
        self.assertEqual(run(self.out, "--write").returncode, 0)
        md = self.out / "cli-reference.md"
        md.write_text("tampered\n", encoding="utf-8")
        self.assertEqual(run(self.out, "--check").returncode, 1)
        self.assertEqual(md.read_text(encoding="utf-8"), "tampered\n")

    def test_missing_outputs_fail_check(self):
        self.assertEqual(run(self.out, "--check").returncode, 1)

    def test_model_without_payload_model_is_rejected(self):
        bad = Path(self._tmp.name) / "bad.json"
        bad.write_text('{"payload": {}}', encoding="utf-8")
        done = run(self.out, "--write", model=bad)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("payload.model", done.stderr)

    def test_missing_docli_names_pinned_install(self):
        done = run(self.out, "--write", docli="docli-does-not-exist")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("sc-docli --version 0.2.0", done.stderr)


def fake_tool(directory: Path, name: str, body: str) -> Path:
    path = directory / name
    path.symlink_to(ROOT / "crates/test-support/fake-exec")
    path.with_name(path.name + ".sh").write_text(body, encoding="utf-8")
    return path


class DocliProvisioningTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.bin = Path(self._tmp.name) / "bin"
        self.bin.mkdir()
        self.out = Path(self._tmp.name) / "out"

    def invoke(self, *extra, path=None):
        env = {"PATH": path if path is not None else f"{self.bin}:/usr/bin:/bin"}
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--check", "--output-dir", str(self.out), "--model-json", str(MODEL), *extra],
            capture_output=True,
            text=True,
            check=False,
            env=env,
        )

    def test_fake_tool_executes_committed_launcher_and_reads_open_data(self):
        path = fake_tool(self.bin, "probe", 'printf "%s\\n" "$1"\n')
        self.assertEqual(path.resolve(), (ROOT / "crates/test-support/fake-exec").resolve())
        data = path.with_name(path.name + ".sh")
        with data.open("a", encoding="utf-8"):
            result = subprocess.run([str(path), "controlled"], capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "controlled\n")

    def test_wrong_docli_version_fails_closed_before_generating(self):
        fake_tool(self.bin, "docli", 'echo "docli 0.1.9"\n')
        done = self.invoke()
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("docli 0.2.0 required, found docli 0.1.9", done.stderr)

    def test_missing_docli_fails_closed_without_install_flag(self):
        done = self.invoke()
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("found none", done.stderr)

    def test_install_flag_runs_pinned_locked_install_then_checks_version(self):
        log = Path(self._tmp.name) / "cargo.log"
        real = shutil.which("docli")
        self.assertIsNotNone(real, "real docli 0.2.0 must be installed for this test")
        # Fake cargo "installs" by exposing the real pinned docli on the fake bin dir.
        fake_tool(self.bin, "cargo", f'echo "$@" > {log}\nln -s {real} {self.bin}/docli\n')
        done = self.invoke("--install-docli")
        self.assertEqual(log.read_text(encoding="utf-8").strip(), "install sc-docli --version 0.2.0 --locked")
        self.assertEqual(done.returncode, 1, done.stderr)  # provisioned; outputs absent -> stale, not a docli error
        self.assertIn("out of date", done.stderr)

    def test_failed_install_fails_closed(self):
        fake_tool(self.bin, "cargo", "exit 7\n")
        done = self.invoke("--install-docli")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("failed (7)", done.stderr)


class ConsumerPreflightTests(unittest.TestCase):
    """The installed kit's consumer preflight runs the wrapper check."""

    RUNNER = ROOT / ".github" / "scripts" / "run-consumer-preflight.sh"
    MANIFEST = ROOT / "release" / "publish-preflight.toml"
    WRAPPER_CHECK = "python3 scripts/generate_cli_reference.py --check --install-docli"
    # Every configured consumer preflight check, in declaration order: the docli
    # wrapper first, then the four checks PR941 restored from the pre-PR866
    # release preflight (also pinned by test_publish_preflight_checks.py).
    EXPECTED_CHECKS = [
        ("cli-reference-fresh", WRAPPER_CHECK),
        ("workspace-fmt", "rustup component add rustfmt && cargo fmt --all --check"),
        ("workspace-clippy", "rustup component add clippy && cargo clippy --workspace --all-targets --all-features -- -D warnings"),
        ("workspace-tests", "cargo test --workspace"),
        (
            "go-native-version-lockstep",
            "python3 .github/scripts/go_native_module.py verify-version-lockstep "
            "--config release/go-native-module.toml --workspace-toml Cargo.toml",
        ),
    ]

    def test_publish_families_and_consumer_preflight_entrypoint(self):
        scripts = ROOT / ".github" / "scripts"
        manifest_path = ROOT / "release" / "publish-artifacts.toml"
        plan = json.loads(
            subprocess.run(
                [sys.executable, str(scripts / "release_artifacts.py"), "preflight-secret-plan", "--manifest", str(manifest_path)],
                capture_output=True,
                text=True,
                check=True,
            ).stdout
        )
        # Publish paths as the kit's own preflight plan sees them (root + post-release).
        paths = {c["name"] for c in plan["root_channels"] + plan["post_release_channels"]}
        contracts = tomllib.loads((ROOT / "release" / "publish-channel-contracts.toml").read_text(encoding="utf-8"))["channels"]
        active = {n for n in contracts if n != "npm"}  # npm: no npm_packages in this manifest
        self.assertEqual(paths, {"crates_io", "github_release", "pypi", "homebrew", "winget", "scoop"})
        self.assertEqual(paths, active)
        # Configured publication families and the raw workflow_dispatch workflow that
        # publishes each. Existence of these files proves nothing about gating, so the
        # assertions below read what each workflow actually does.
        publish_yml = {
            "crates_io": "release.yml",
            "github_release": "release.yml",
            **{n: f"{n}-publish.yml" for n in ("pypi", "homebrew", "winget", "scoop")},
        }
        workflows = ROOT / ".github" / "workflows"
        runner = "run-consumer-preflight.sh"
        for name in sorted(paths):
            with self.subTest(publish_path=name):
                text = (workflows / publish_yml[name]).read_text(encoding="utf-8")
                self.assertRegex(text, r"(?m)^on:\n  workflow_dispatch:", "publication is a raw workflow_dispatch entrypoint")
                # Known limitation of the existing sc-publish seam, recorded as a
                # phase-end decision (not a pass): no publication workflow runs the
                # consumer runner or needs a job that does. Adding such a gate would
                # change shared-kit code, which the user rejected (sc-publish PR 159).
                self.assertNotIn(runner, text, f"{publish_yml[name]} now references the runner: update this test and the phase-end decision")
        # The one real consumer entrypoint is the dispatched Release Preflight
        # workflow: it is the only workflow that runs the runner, and it is
        # workflow_dispatch-only, so no publication workflow can `needs` it. It
        # runs the installed consumer runner and records the outcome in the final
        # deny summary, so a failing check blocks that gate's verdict.
        referencing = sorted(f.name for f in workflows.glob("*.yml") if runner in f.read_text(encoding="utf-8"))
        self.assertEqual(referencing, ["release-preflight.yml"])
        preflight = (workflows / "release-preflight.yml").read_text(encoding="utf-8")
        self.assertRegex(preflight, r"(?m)^on:\n  workflow_dispatch:\n    inputs:")
        self.assertNotRegex(preflight, r"(?m)^  (push|pull_request|workflow_call|schedule):")
        self.assertIn("id: consumer_preflight", preflight)
        self.assertIn("run: bash .github/scripts/run-consumer-preflight.sh", preflight)
        self.assertIn('record consumer-preflight "${CONSUMER_PREFLIGHT}"', preflight)
        self.assertIn("CONSUMER_PREFLIGHT: ${{ steps.consumer_preflight.outcome }}", preflight)
        # Each check is declared once, through preflight_checks, and the rendered
        # manifest carries exactly the same ordered checks as the install input.
        manifest = tomllib.loads(self.MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual([(c["id"], c["run"]) for c in manifest["checks"]], self.EXPECTED_CHECKS)
        install = json.loads((ROOT / "release" / "sc-publish-install.json").read_text(encoding="utf-8"))
        self.assertEqual([(c["id"], c["run"]) for c in install["preflight_checks"]], self.EXPECTED_CHECKS)
        # The kit's optional docli object is intentionally unused: docli runs
        # through preflight_checks so the wrapper owns pin enforcement.
        self.assertNotIn("docli", manifest)
        self.assertNotIn("docli", install)

    def test_installer_package_is_pinned_by_immutable_revision(self):
        install = json.loads((ROOT / "release" / "sc-publish-install.json").read_text(encoding="utf-8"))
        pin = install["sc_publish_pin"]
        self.assertRegex(pin["revision"], r"^[0-9a-f]{40}$")
        self.assertEqual(pin["repository"], "https://github.com/randlee/sc-publish")
        recorded = tomllib.loads((ROOT / "release" / "sc-publish-pin.toml").read_text(encoding="utf-8"))
        self.assertEqual(recorded["sc-publish"], pin)

    def test_consumer_preflight_rejects_stale_reference(self):
        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp)
            out = stage / "out"
            (stage / ".github" / "scripts").mkdir(parents=True)
            (stage / "release").mkdir()
            shutil.copy(self.RUNNER, stage / ".github" / "scripts" / "run-consumer-preflight.sh")
            declared = tomllib.loads(self.MANIFEST.read_text(encoding="utf-8"))["checks"]
            self.assertEqual([(c["id"], c["run"]) for c in declared], self.EXPECTED_CHECKS)
            # Only the declared docli check, pointed at controlled model and
            # outputs: the stale-reference refusal is what this test exercises,
            # and the restored cargo/Go checks are covered by
            # test_publish_preflight_checks.py.
            command = (
                f"{sys.executable} {SCRIPT} --check --output-dir {out} --model-json {MODEL}"
            )
            (stage / "release" / "publish-preflight.toml").write_text(
                'schema_version = 1\n\n[[checks]]\nid = "cli-reference-fresh"\n'
                f'description = "controlled"\nrun = {json.dumps(command)}\n',
                encoding="utf-8",
            )

            def preflight():
                return subprocess.run(
                    ["bash", str(stage / ".github" / "scripts" / "run-consumer-preflight.sh")],
                    capture_output=True,
                    text=True,
                    check=False,
                )

            self.assertEqual(run(out, "--write").returncode, 0)
            self.assertEqual(preflight().returncode, 0)
            md = out / "cli-reference.md"
            md.write_text(md.read_text(encoding="utf-8") + "stale\n", encoding="utf-8")
            stale_text = md.read_text(encoding="utf-8")
            failed = preflight()
            self.assertEqual(failed.returncode, 1, failed.stdout + failed.stderr)
            self.assertIn("FAIL cli-reference-fresh", failed.stderr)
            self.assertEqual(md.read_text(encoding="utf-8"), stale_text)


if __name__ == "__main__":
    unittest.main()
