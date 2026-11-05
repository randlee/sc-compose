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
