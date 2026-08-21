import importlib.util
import platform
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("install_uniffi_bindgen_go.py")
SPEC = importlib.util.spec_from_file_location("install_uniffi", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class InstallerTests(unittest.TestCase):
    def test_linux_timeout_remains_1800(self):
        with patch.object(platform, "system", return_value="Linux"):
            self.assertEqual(MODULE.timeout_seconds(), 1800)

    def test_darwin_timeout_is_extended(self):
        with patch.object(platform, "system", return_value="Darwin"):
            self.assertEqual(MODULE.timeout_seconds(), 2700)

    def test_retry_reuses_target_dir_and_timeout(self):
        with patch.object(MODULE.shutil, "which", return_value=None), patch.object(
            MODULE.platform, "system", return_value="Darwin"
        ), patch.object(MODULE.time, "sleep"), patch.object(
            MODULE.subprocess, "run", side_effect=[subprocess.TimeoutExpired(MODULE.COMMAND, 2700), None]
        ) as run:
            self.assertEqual(MODULE.main(), 0)
            self.assertEqual(run.call_count, 2)
            for call in run.call_args_list:
                self.assertEqual(call.kwargs["timeout"], 2700)
                self.assertEqual(call.kwargs["env"]["CARGO_TARGET_DIR"], str(MODULE.TARGET_DIR))

    def test_cached_binary_skips_build(self):
        with patch.object(MODULE.shutil, "which", return_value="/cached/uniffi-bindgen-go"), patch.object(
            MODULE.subprocess, "run"
        ) as run:
            self.assertEqual(MODULE.main(), 0)
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
