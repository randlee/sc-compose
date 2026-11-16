"""Deterministic tests for scripts/test_real_bd.py (controlled inputs only)."""
from __future__ import annotations

import importlib.util
import json
import os
import select
import sys
import stat
import subprocess
import tempfile
import time
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("test_real_bd", ROOT / "scripts" / "test_real_bd.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

ONE_PASS = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
PROCESS_TREE_HANG_GUARD_SECONDS = 60


def artifact(target: str, executable: str) -> str:
    return json.dumps({
        "reason": "compiler-artifact",
        "target": {"name": target, "kind": ["test"], "test": True},
        "executable": executable,
    }) + "\n"


class RealBdQualification(unittest.TestCase):
    def test_database_mode_matches_presence_even_for_empty_value(self) -> None:
        self.assertEqual(MODULE.database_mode({}), "embedded")
        self.assertEqual(MODULE.database_mode({"SC_COMPOSE_TEST_DOLT_PORT": ""}), "server")
        self.assertEqual(MODULE.database_mode({"SC_COMPOSE_TEST_DOLT_PORT": "3317"}), "server")

    def test_selected_suites_preserve_database_mode_filter(self) -> None:
        self.assertEqual(len(MODULE.selected_suites(False)), 3)
        label, args, test_filter = MODULE.selected_suites(True)[0]
        self.assertEqual(label, "database-mode")
        self.assertIn("graph_bd", args)
        self.assertEqual(test_filter, "repour_clean_dirty_database_mode")

    def test_invalid_arguments_are_rejected_before_bd(self) -> None:
        with patch.object(MODULE.sys, "argv", ["test_real_bd.py", "--invalid"]), \
             patch.object(MODULE, "resolve_bd") as resolve:
            self.assertEqual(MODULE.main(), 2)
            resolve.assert_not_called()

    def test_missing_bd_keeps_not_run_path_and_does_not_build_suites(self) -> None:
        stderr = StringIO()
        with patch.object(MODULE, "resolve_bd", return_value="BD_EXECUTABLE is not set"), \
             patch.object(MODULE.sys, "argv", ["test_real_bd.py"]), \
             patch.object(MODULE, "prepare_suites") as prepare, redirect_stderr(stderr):
            self.assertEqual(MODULE.main(), 2)
        self.assertIn("real-bd qualification NOT RUN: BD_EXECUTABLE is not set", stderr.getvalue())
        prepare.assert_not_called()

    def test_builds_all_targets_before_listing_ignored_tests(self) -> None:
        suites = (
            ("sc-composer-beads/graph_bd", ["-p", "sc-composer-beads", "--test", "graph_bd"], None),
            ("sc-composer-beads/bd_integration", ["-p", "sc-composer-beads", "--test", "bd_integration"], None),
        )
        results = [
            subprocess.CompletedProcess([], 0, artifact("graph_bd", "graph-bin"), ""),
            subprocess.CompletedProcess([], 0, artifact("bd_integration", "integration-bin"), ""),
            subprocess.CompletedProcess([], 0, "first_test: test\nrepour_clean_dirty_database_mode: test\n", ""),
            subprocess.CompletedProcess([], 0, "second_test: test\n", ""),
        ]
        with patch.object(MODULE, "selected_suites", return_value=suites), \
             patch.object(MODULE.subprocess, "run", side_effect=results) as spawn:
            prepared = MODULE.prepare_suites({})
        self.assertEqual(prepared, [("sc-composer-beads/graph_bd", "graph-bin", ["first_test"]),
                                    ("sc-composer-beads/bd_integration", "integration-bin", ["second_test"]) ])
        calls = spawn.call_args_list
        self.assertIn("--no-run", calls[0].args[0])
        self.assertIn("-j2", calls[0].args[0])
        self.assertIn("--no-run", calls[1].args[0])
        self.assertEqual(calls[2].args[0][-2:], ["--list", "--ignored"])
        self.assertEqual(calls[3].args[0][-2:], ["--list", "--ignored"])
        self.assertEqual(calls[0].kwargs["timeout"], MODULE.BUILD_TIMEOUT_SECONDS)
        self.assertEqual(calls[1].kwargs["timeout"], MODULE.BUILD_TIMEOUT_SECONDS)
        self.assertTrue(all(call.kwargs["timeout"] == MODULE.LIST_TIMEOUT_SECONDS for call in calls[2:]))

    def test_build_timeout_is_a_suite_failure(self) -> None:
        suites = (("graph", ["-p", "sc-composer-beads", "--test", "graph_bd"], None),)
        timeout = subprocess.TimeoutExpired(["cargo", "test"], MODULE.BUILD_TIMEOUT_SECONDS, output="partial")
        stderr = StringIO()
        with patch.object(MODULE, "selected_suites", return_value=suites), \
             patch.object(MODULE.subprocess, "run", side_effect=timeout), redirect_stderr(stderr):
            self.assertEqual(MODULE.prepare_suites({}), [("graph", "", [])])
        self.assertIn("build timed out", stderr.getvalue())

    def test_list_timeout_is_a_suite_failure(self) -> None:
        suites = (("graph", ["-p", "sc-composer-beads", "--test", "graph_bd"], None),)
        results = [subprocess.CompletedProcess([], 0, artifact("graph_bd", "graph-bin"), ""),
                   subprocess.TimeoutExpired(["graph-bin"], MODULE.LIST_TIMEOUT_SECONDS)]
        stderr = StringIO()
        with patch.object(MODULE, "selected_suites", return_value=suites), \
             patch.object(MODULE.subprocess, "run", side_effect=results), redirect_stderr(stderr):
            self.assertEqual(MODULE.prepare_suites({}), [("graph", "graph-bin", [])])
        self.assertIn("discovery timed out", stderr.getvalue())

    def test_each_ignored_test_runs_in_its_own_timed_process_and_counts_aggregate(self) -> None:
        output = StringIO()
        prepared = [("graph", "test-binary", ["first_test", "second_test"])]
        completed = subprocess.CompletedProcess([], 0, ONE_PASS, "")
        with patch.object(MODULE, "prepare_suites", return_value=prepared), \
             patch.object(MODULE, "run_test_process", return_value=(completed, None)) as spawn, \
             redirect_stdout(output):
            self.assertEqual(MODULE.run_suites({}, {"bd_version": "bd 1.3", "bd_sha256": "abc", "database_mode": "server"}), 0)
        self.assertEqual(spawn.call_count, 2)
        self.assertEqual([call.args[0][1] for call in spawn.call_args_list], ["first_test", "second_test"])
        self.assertTrue(all("--exact" in call.args[0] and "--ignored" in call.args[0] for call in spawn.call_args_list))
        self.assertTrue(all(call.args[2] == MODULE.PROCESS_TIMEOUT_SECONDS for call in spawn.call_args_list))
        self.assertIn('"passed": 2', output.getvalue())
        self.assertIn('"qualification": "qualified"', output.getvalue())
        self.assertIn('"bd_version": "bd 1.3"', output.getvalue())
        self.assertIn('"bd_sha256": "abc"', output.getvalue())
        self.assertIn('"database_mode": "server"', output.getvalue())

    def test_hung_stub_command_timeout_names_the_test(self) -> None:
        stdout = StringIO()
        stderr = StringIO()
        prepared = [("graph", "stub-test-binary", ["known_hung_test"])]
        timeout = subprocess.TimeoutExpired(
            ["stub-test-binary", "known_hung_test", "--exact", "--ignored"],
            MODULE.PROCESS_TIMEOUT_SECONDS,
            output=b"partial stdout",
            stderr=b"partial stderr",
        )
        with patch.object(MODULE, "prepare_suites", return_value=prepared), \
             patch.object(MODULE, "run_test_process", return_value=(None, timeout)), \
             redirect_stdout(stdout), redirect_stderr(stderr):
            self.assertEqual(MODULE.run_suites({}, {"database_mode": "server"}), 1)
        diagnostic = stderr.getvalue()
        self.assertIn("real-bd test timed out: graph::known_hung_test", diagnostic)
        self.assertIn("stub-test-binary", diagnostic)
        self.assertIn("known_hung_test", diagnostic)
        self.assertIn("partial stdout", diagnostic)
        self.assertIn("partial stderr", diagnostic)
        self.assertIn('"passed": 0', stdout.getvalue())
        self.assertIn('"failed": 1', stdout.getvalue())
        self.assertIn('"exit": 1', stdout.getvalue())
        self.assertIn('"qualification": "failed"', stdout.getvalue())

    def assert_bad_version(self, status: int, output: str) -> None:
        with patch.object(MODULE, "resolve_bd", return_value=Path("/fake/bd")), \
             patch.object(MODULE.sys, "argv", ["test_real_bd.py"]), \
             patch.object(MODULE.subprocess, "run", return_value=subprocess.CompletedProcess([], status, output, "")), \
             patch.object(MODULE, "run_suites") as suites:
            self.assertEqual(MODULE.main(), 1)
            suites.assert_not_called()

    def test_failed_version_probe_fails_before_suites(self) -> None:
        self.assert_bad_version(1, "bd 1.2")

    def test_empty_version_probe_fails_before_suites(self) -> None:
        self.assert_bad_version(0, "  ")

    def test_unsupported_database_mode_cannot_qualify(self) -> None:
        stdout = StringIO()
        with tempfile.TemporaryDirectory() as tmp:
            summary = Path(tmp) / "step-summary.md"
            result = subprocess.CompletedProcess(
                [], 0, "SC_COMPOSE_MODE_UNSUPPORTED: BEADS_GRAPH_DIRTY_UNKNOWN\n" + ONE_PASS, ""
            )
            with patch.object(MODULE, "prepare_suites", return_value=[("database-mode", "test-bin", ["test_a"])]), \
                 patch.object(MODULE, "run_test_process", return_value=(result, None)), \
                 patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(summary)}), redirect_stdout(stdout):
                self.assertEqual(MODULE.run_suites({}, {"database_mode": "server", "bd_version": "bd 1.3.1"}, database_mode=True), 0)
            self.assertIn('"qualification": "not_qualified"', stdout.getvalue())
            self.assertIn('"ok": false', stdout.getvalue())
            self.assertEqual(summary.read_text(), "bd bd 1.3.1: server not_qualified (unsupported)\n")

    def test_timeout_kills_test_and_grandchild_process_group(self) -> None:
        if os.name == "nt":
            self.skipTest("process group liveness check is POSIX-specific")
        read_fd, write_fd = os.pipe()
        self.addCleanup(os.close, read_fd)
        write_fd_open = [True]

        def close_test_writer() -> None:
            if write_fd_open[0]:
                os.close(write_fd)
                write_fd_open[0] = False

        self.addCleanup(close_test_writer)
        grandchild = "import threading; threading.Event().wait(60)"
        parent = (
            "import subprocess,sys,threading; "
            "subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]], pass_fds=(int(sys.argv[2]),)); "
            "print('READY '+sys.argv[3],flush=True); threading.Event().wait(60)"
        )
        command = [sys.executable, "-c", parent, grandchild, str(write_fd), "known_hung_test"]
        real_popen = MODULE.subprocess.Popen

        def cleanup_process(proc) -> None:
            MODULE.kill_process_tree(proc)
            try:
                proc.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.communicate(timeout=5)

        def tracked_popen(*args, **kwargs):
            proc = real_popen(*args, **kwargs)
            self.addCleanup(cleanup_process, proc)
            return proc

        group_pid = []

        def trigger_timeout(proc, _timeout):
            group_pid.append(proc.pid)
            deadline = time.monotonic() + PROCESS_TREE_HANG_GUARD_SECONDS
            output = {proc.stdout: bytearray(), proc.stderr: bytearray()}
            streams = list(output)
            captured = ""
            while streams and time.monotonic() < deadline:
                remaining = max(0.0, deadline - time.monotonic())
                readable, _, _ = select.select(streams, [], [], remaining)
                if not readable:
                    break
                for stream in readable:
                    chunk = os.read(stream.fileno(), 4096)
                    output[stream].extend(chunk)
                    if stream is proc.stdout and b"\n" in output[stream]:
                        captured = output[stream].split(b"\n", 1)[0].decode("utf-8", errors="replace")
                        break
                    if not chunk:
                        streams.remove(stream)
                if captured:
                    break
            captured_output = "stdout: " + output[proc.stdout].decode("utf-8", errors="replace")
            captured_output += "\nstderr: " + output[proc.stderr].decode("utf-8", errors="replace")
            self.assertEqual(
                captured,
                "READY known_hung_test",
                f"READY handshake timed out or mismatched; command={command!r}; {captured_output}",
            )
            raise subprocess.TimeoutExpired(command, 1)

        with patch.object(MODULE.subprocess, "Popen", side_effect=tracked_popen), \
             patch.object(MODULE, "wait_for_process", side_effect=trigger_timeout):
            completed, timeout = MODULE.run_test_process(
                command, os.environ.copy(), 1, pass_fds=(write_fd,)
            )
        self.assertIsNone(completed)
        self.assertIsNotNone(timeout)
        self.assertIn("known_hung_test", timeout.cmd)
        close_test_writer()
        eof_ready = bool(select.select([read_fd], [], [], PROCESS_TREE_HANG_GUARD_SECONDS)[0])
        try:
            os.killpg(group_pid[0], 0)
            group_state = "present"
        except ProcessLookupError:
            group_state = "absent"
        except PermissionError:
            group_state = "present (permission denied)"
        self.assertTrue(
            eof_ready,
            f"grandchild retained the inherited pipe after process-tree kill; process group {group_pid[0]}: {group_state}",
        )
        self.assertEqual(os.read(read_fd, 1), b"", "expected EOF after the parent and grandchild exited")

    def test_counts_sum_across_result_lines(self) -> None:
        two_results = ONE_PASS + "test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
        self.assertEqual(MODULE.parse_counts(two_results), (13, 0, 0))

    def test_zero_executed_is_not_a_pass(self) -> None:
        zero = "test result: ok. 0 passed; 0 failed; 15 ignored; 0 measured\n"
        self.assertIn("0 passed", MODULE.verdict(0, MODULE.parse_counts(zero)))

    def test_missing_result_lines_is_not_a_pass(self) -> None:
        self.assertIsNotNone(MODULE.verdict(0, MODULE.parse_counts("")))

    def test_failures_and_nonzero_exit_fail(self) -> None:
        self.assertIsNotNone(MODULE.verdict(101, (3, 1, 0)))
        self.assertIsNotNone(MODULE.verdict(1, (3, 0, 0)))

    def test_clean_run_passes(self) -> None:
        self.assertIsNone(MODULE.verdict(0, MODULE.parse_counts(ONE_PASS)))

    def test_missing_or_bad_bd_is_refused_not_skipped(self) -> None:
        self.assertIsInstance(MODULE.resolve_bd({}), str)
        self.assertIsInstance(MODULE.resolve_bd({"BD_EXECUTABLE": "/nonexistent/bd"}), str)

    def test_fixture_home_is_isolated_and_ambient_beads_dir_is_removed(self) -> None:
        env = MODULE.isolated_environment({"HOME": "/real/home", "BEADS_DIR": "/real/home/.beads"}, Path("/tmp/test-home"))
        self.assertEqual(env["HOME"], "/tmp/test-home")
        self.assertNotEqual(env["HOME"], "/real/home")
        self.assertNotIn("BEADS_DIR", env)
        self.assertEqual(env["CARGO_HOME"], "/real/home/.cargo")

    def test_executable_bd_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bd = Path(tmp) / "bd"
            bd.write_text("#!/bin/sh\n")
            bd.chmod(bd.stat().st_mode | stat.S_IXUSR)
            self.assertEqual(MODULE.resolve_bd({"BD_EXECUTABLE": str(bd)}), bd)


if __name__ == "__main__":
    unittest.main()
