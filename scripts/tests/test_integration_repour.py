"""Controlled tests for the local integration runner's teardown and reporting."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import MagicMock, patch
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("integration_repour", ROOT / "scripts" / "integration_repour.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class IntegrationRunnerTests(unittest.TestCase):
    @unittest.skipIf(MODULE.os.name == "nt", "Unix pgrep format")
    def test_server_listing_does_not_count_shells_that_mention_dolt(self) -> None:
        sample = "12 /opt/bin/dolt sql-server --port 3308\n13 /bin/zsh -c 'pgrep -fl dolt sql-server'\n"
        with patch.object(MODULE.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, sample, "")):
            self.assertEqual(MODULE.server_processes(), {"12 /opt/bin/dolt sql-server --port 3308"})

    def test_cleanup_refuses_to_remove_root_with_live_process(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "owned"
            root.mkdir()
            with patch.object(MODULE, "processes_referencing", return_value=["123 running"]), \
                 patch.object(MODULE, "server_processes", return_value=set()):
                with self.assertRaisesRegex(AssertionError, "refusing cleanup"):
                    MODULE.cleanup_root(root, set())
            self.assertTrue(root.exists())

    def test_cleanup_removes_root_only_after_process_checks_pass(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "owned"
            root.mkdir()
            with patch.object(MODULE, "processes_referencing", return_value=[]), \
                 patch.object(MODULE, "server_processes", return_value={"shared"}):
                MODULE.cleanup_root(root, {"shared"})
            self.assertFalse(root.exists())

    def test_server_is_killed_and_reaped_after_failure_deadline(self) -> None:
        server = MagicMock()
        server.poll.return_value = None
        server.wait.side_effect = [subprocess.TimeoutExpired("dolt", 15), 0]
        MODULE.stop_server(server)
        server.terminate.assert_called_once()
        server.kill.assert_called_once()
        self.assertEqual(server.wait.call_count, 2)

    def test_report_is_xhtml_and_escapes_assertion(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            reports = Path(tmp)
            (reports / "index.html").write_text("<html><body>\n    <ul>\n    </ul>\n</body></html>")
            with patch.object(MODULE, "REPORTS", reports):
                report = MODULE.write_report("bd <version>", "abc", "dolt 2", [("scenario", "FAIL", "a < b\x1b")])
                MODULE.write_report("bd other", "def", "dolt 2", [("other", "PASS", "")])
            tree = ET.parse(report)
            self.assertIn("a < b", "".join(tree.getroot().itertext()))
            self.assertEqual(len(list(tree.getroot().iter("{http://www.w3.org/1999/xhtml}section"))), 2)
            self.assertIn(report.name, (reports / "index.html").read_text())


if __name__ == "__main__":
    unittest.main()
