"""Controlled tests for the local integration runner's teardown and reporting."""
from __future__ import annotations

import importlib.util
import io
import queue
from pathlib import Path
import subprocess
import socket
import tempfile
import unittest
from unittest.mock import MagicMock, patch
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("integration_repour", ROOT / "scripts" / "integration_repour.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class IntegrationRunnerTests(unittest.TestCase):
    def test_owned_child_ready_line_for_expected_port_signals_readiness(self) -> None:
        events = queue.Queue()
        stream = io.BytesIO(b'Starting server with Config HP="127.0.0.1:49200"\nServer ready. Accepting connections.\n')
        log = io.BytesIO()
        MODULE.read_server_output(stream, log, 49200, events)
        self.assertEqual(events.get_nowait(), "ready")
        self.assertIn(b"Server ready", log.getvalue())

    def test_ready_from_wrong_port_is_not_accepted(self) -> None:
        events = queue.Queue()
        stream = io.BytesIO(b'Starting server with Config HP="127.0.0.1:49201"\nServer ready. Accepting connections.\n')
        MODULE.read_server_output(stream, io.BytesIO(), 49200, events)
        self.assertEqual(events.get_nowait(), "wrong-port")

    def test_eof_reports_typed_bind_failure_and_captured_log(self) -> None:
        events = queue.Queue()
        with tempfile.TemporaryDirectory() as tmp:
            log_path = Path(tmp) / "server.log"
            with log_path.open("wb") as log:
                MODULE.read_server_output(io.BytesIO(b"address already in use\n"), log, 49200, events)
            with self.assertRaisesRegex(MODULE.ServerBindError, "DOLT_SERVER_BIND_FAILED.*address already in use"):
                MODULE.wait_for_server(events, MagicMock(), log_path)

    def test_readiness_timeout_is_a_failure_only_backstop(self) -> None:
        events = MagicMock()
        events.get.side_effect = queue.Empty
        with tempfile.TemporaryDirectory() as tmp:
            log_path = Path(tmp) / "server.log"
            log_path.write_text("partial server output")
            with self.assertRaisesRegex(AssertionError, "DOLT_SERVER_NOT_READY.*partial server output"):
                MODULE.wait_for_server(events, MagicMock(), log_path)
        events.get.assert_called_once_with(timeout=30)

    def test_os_selects_candidate_and_explicit_port_is_validated(self) -> None:
        listener = MagicMock()
        listener.__enter__.return_value = listener
        listener.getsockname.return_value = ("127.0.0.1", 49157)
        with patch.object(MODULE.socket, "socket", return_value=listener):
            self.assertEqual(MODULE.select_server_port(None), 49157)
        listener.bind.assert_called_once_with(("127.0.0.1", 0))
        self.assertEqual(MODULE.select_server_port("45678"), 45678)
        with self.assertRaisesRegex(AssertionError, "DOLT_PORT_INVALID"):
            MODULE.select_server_port("3308")
        with self.assertRaisesRegex(AssertionError, "DOLT_PORT_INVALID"):
            MODULE.select_server_port("invalid")

    def test_conflicting_bound_port_reports_child_log(self) -> None:
        with socket.socket() as held, tempfile.TemporaryDirectory() as tmp:
            held.bind(("127.0.0.1", 0))
            held.listen()
            port = held.getsockname()[1]
            child = MagicMock()
            child.poll.return_value = 1
            log = Path(tmp) / "server.log"
            log.write_text(f"listen 127.0.0.1:{port}: address already in use")
            with self.assertRaisesRegex(MODULE.ServerBindError, f"DOLT_SERVER_BIND_FAILED.*{port}"):
                MODULE.assert_server_running(child, log)
            log.write_text(f"Port {port} already in use.")
            with self.assertRaisesRegex(MODULE.ServerBindError, f"DOLT_SERVER_BIND_FAILED.*{port}"):
                MODULE.assert_server_running(child, log)

    @unittest.skipIf(MODULE.os.name == "nt", "Unix pgrep format")
    def test_server_listing_does_not_count_shells_that_mention_dolt(self) -> None:
        sample = "12 /opt/bin/dolt sql-server --port 3308\n13 /bin/zsh -c 'pgrep -fl dolt sql-server'\n"
        with patch.object(MODULE.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, sample, "")):
            self.assertEqual(MODULE.server_processes(), {"12 /opt/bin/dolt sql-server --port 3308"})

    def test_cleanup_refuses_to_remove_root_with_live_process(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "owned"
            root.mkdir()
            with patch.object(MODULE, "processes_referencing", return_value=["123 owned child"]):
                with self.assertRaisesRegex(AssertionError, "refusing cleanup"):
                    MODULE.cleanup_root(root, [])
            self.assertTrue(root.exists())

    def test_foreign_server_started_during_run_does_not_block_cleanup(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "owned"
            root.mkdir()
            server = MagicMock()
            server.poll.return_value = 0
            with patch.object(MODULE, "processes_referencing", return_value=[]), \
                 patch.object(MODULE, "server_processes", return_value={"shared", "new foreign"}) as host_servers:
                MODULE.cleanup_root(root, [server])
            host_servers.assert_not_called()
            self.assertFalse(root.exists())

    def test_cleanup_refuses_to_remove_root_with_owned_server_still_running(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "owned"
            root.mkdir()
            server = MagicMock()
            server.pid = 123
            server.poll.return_value = None
            with patch.object(MODULE, "processes_referencing", return_value=[]):
                with self.assertRaisesRegex(AssertionError, "owned Dolt server pid 123"):
                    MODULE.cleanup_root(root, [server])
            self.assertTrue(root.exists())

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

    def test_report_migrates_legacy_html_and_preserves_other_panels(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            reports = Path(tmp)
            index = reports / "index.html"
            index.write_text("<html><body>\n    <ul>\n    </ul>\n</body></html>")
            with patch.object(MODULE, "REPORTS", reports):
                report = MODULE.write_report("bd first", "abc", "dolt 2", [("first", "PASS", "")])
                legacy = report.with_suffix(".html")
                report.rename(legacy)
                index.write_text(index.read_text().replace(report.name, legacy.name))
                updated = MODULE.write_report("bd second", "def", "dolt 2", [("second", "PASS", "")])
            self.assertFalse(legacy.exists())
            self.assertEqual(updated.suffix, ".xhtml")
            self.assertEqual(len(list(ET.parse(updated).getroot().iter("{http://www.w3.org/1999/xhtml}section"))), 2)
            self.assertIn(updated.name, index.read_text())
            self.assertNotIn(legacy.name, index.read_text())


if __name__ == "__main__":
    unittest.main()
