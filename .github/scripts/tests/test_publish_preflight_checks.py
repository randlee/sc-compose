"""The restored release preflight checks are declared through the supported consumer hook.

Deterministic: reads the install input, its rendered manifest and the CI workflow; the
cheap Go lockstep check is executed against the repository's own contract files.
"""

import json
import subprocess
import sys
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RESTORED = {
    "workspace-fmt": "rustup component add rustfmt && cargo fmt --all --check",
    "workspace-clippy": "rustup component add clippy && cargo clippy --workspace --all-targets --all-features -- -D warnings",
    "workspace-tests": "cargo test --workspace",
    "go-native-version-lockstep": (
        "python3 .github/scripts/go_native_module.py verify-version-lockstep "
        "--config release/go-native-module.toml --workspace-toml Cargo.toml"
    ),
}


def checks(entries):
    return {entry["id"]: entry["run"] for entry in entries}


class RestoredPreflightChecks(unittest.TestCase):
    def test_install_input_declares_each_restored_check(self):
        install = json.loads((ROOT / "release/sc-publish-install.json").read_text(encoding="utf-8"))
        declared = checks(install["preflight_checks"])
        for check_id, command in RESTORED.items():
            self.assertEqual(declared.get(check_id), command)
        self.assertIn("cli-reference-fresh", declared)

    def test_rendered_manifest_matches_the_install_input(self):
        install = json.loads((ROOT / "release/sc-publish-install.json").read_text(encoding="utf-8"))
        rendered = tomllib.loads((ROOT / "release/publish-preflight.toml").read_text(encoding="utf-8"))
        self.assertEqual(checks(rendered["checks"]), checks(install["preflight_checks"]))

    def test_no_managed_workflow_carries_the_restored_checks(self):
        text = (ROOT / ".github/workflows/release-preflight.yml").read_text(encoding="utf-8")
        self.assertNotIn("go_native_module.py", text)

    def test_ci_still_runs_the_same_go_lockstep_check(self):
        ci = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertIn("go_native_module.py verify-version-lockstep", ci)

    def test_go_lockstep_check_passes_on_the_repository_contract(self):
        result = subprocess.run(RESTORED["go-native-version-lockstep"].split(), cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    @unittest.skipIf(sys.platform == "win32", "the consumer preflight runs these commands with POSIX sh on Linux")
    def test_component_checks_provision_before_running_and_fail_closed(self):
        # The preflight toolchain uses the minimal profile, so rustfmt/clippy
        # must be added before their checks. Fake rustup/cargo record their
        # argv; the declared command runs through the same shell=True path.
        import os
        import tempfile

        cases = {
            "workspace-fmt": ("rustfmt", ["fmt", "--all", "--check"]),
            "workspace-clippy": (
                "clippy",
                ["clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"],
            ),
        }
        for check_id, (component, cargo_args) in cases.items():
            for rustup_exit, cargo_exit in ((0, 0), (0, 1), (1, 0)):
                with self.subTest(check=check_id, rustup=rustup_exit, cargo=cargo_exit), tempfile.TemporaryDirectory() as tmp:
                    bin_dir = Path(tmp)
                    calls = bin_dir / "calls"
                    for tool, code in (("rustup", rustup_exit), ("cargo", cargo_exit)):
                        fake = bin_dir / tool
                        fake.symlink_to(ROOT / "crates/test-support/fake-exec")
                        fake.with_name(fake.name + ".sh").write_text(
                            f'echo "{tool} $*" >> "{calls}"\nexit {code}\n', encoding="utf-8"
                        )
                        self.assertEqual(fake.resolve(), (ROOT / "crates/test-support/fake-exec").resolve())
                    env = dict(os.environ, PATH=f"{bin_dir}{os.pathsep}{os.environ['PATH']}")
                    result = subprocess.run(RESTORED[check_id], shell=True, cwd=ROOT, env=env)
                    recorded = calls.read_text(encoding="utf-8").splitlines()
                    expected = [f"rustup component add {component}"]
                    if rustup_exit == 0:
                        expected.append("cargo " + " ".join(cargo_args))
                    self.assertEqual(recorded, expected)
                    self.assertEqual(result.returncode != 0, bool(rustup_exit or cargo_exit))

    def test_go_lockstep_check_rejects_a_workspace_without_a_release_version(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            drifted = Path(tmp) / "Cargo.toml"
            drifted.write_text('[workspace.package]\nedition = "2021"\n', encoding="utf-8")
            command = RESTORED["go-native-version-lockstep"].replace("--workspace-toml Cargo.toml", f"--workspace-toml {drifted}")
            result = subprocess.run(command.split(), cwd=ROOT, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
