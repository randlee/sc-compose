"""Verify runtime setup delegates to the installed kit and preserves failures."""
from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).parents[1] / "materialize_sc_lint_runtime.py"


class MaterializeRuntimeTests(unittest.TestCase):
    def test_calls_consumer_bootstrap_with_its_config(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bootstrap = root / ".sc-lint" / "bootstrap.py"
            bootstrap.parent.mkdir()
            bootstrap.write_text(
                "import json, pathlib, sys\n"
                "pathlib.Path('called.json').write_text(json.dumps(sys.argv[1:]))\n",
                encoding="utf-8",
            )
            result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(root)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads((root / "called.json").read_text()), ["setup", "--config", "sc-lint.toml"])

    def test_missing_bootstrap_and_setup_failure_are_not_success(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            command = [sys.executable, str(SCRIPT), "--root", str(root)]
            missing = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn("bootstrap is missing", missing.stderr)
            bootstrap = root / ".sc-lint" / "bootstrap.py"
            bootstrap.parent.mkdir()
            bootstrap.write_text("raise SystemExit(7)\n", encoding="utf-8")
            failed = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(failed.returncode, 7)


if __name__ == "__main__":
    unittest.main()


class LockedWheelSetupTests(unittest.TestCase):
    def test_clean_environment_installs_hashed_requirements_before_bootstrap(self) -> None:
        import importlib.util
        from unittest import mock

        script = SCRIPT.parent / "sc_lint_python.py"
        spec = importlib.util.spec_from_file_location("sc_lint_python", script)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        prepare = module.prepare
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with mock.patch.object(module, "ROOT", root), \
                 mock.patch.object(module, "REQUIREMENTS", root / "requirements.txt"), \
                 mock.patch.object(module.os, "environ", {"PIP_CONSTRAINT": "old"}), \
                 mock.patch.object(module.sys, "argv", [str(script), ".sc-lint/bootstrap.py", "setup", "--config", "sc-lint.toml"]), \
                 mock.patch.object(module.venv, "create") as create, \
                 mock.patch.object(module.subprocess, "run") as run, \
                 mock.patch.object(module.subprocess, "call", return_value=0) as call, \
                 mock.patch.object(module, "prepare", side_effect=lambda: prepare(run, create)):
                self.assertEqual(module.main(), 0)
            create.assert_called_once_with(root / ".sc-lint" / "venv", with_pip=True)
            args = run.call_args.args[0]
            self.assertEqual(args[1:], ["-m", "pip", "install", "--disable-pip-version-check", "--force-reinstall", "--require-hashes", "-r", str(root / "requirements.txt")])
            self.assertNotIn("PIP_CONSTRAINT", run.call_args.kwargs["env"])
            self.assertEqual(call.call_args.args[0][1:], [".sc-lint/bootstrap.py", "setup", "--config", "sc-lint.toml"])
