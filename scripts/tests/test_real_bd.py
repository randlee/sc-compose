"""Deterministic tests for scripts/test_real_bd.py (controlled inputs only)."""
from __future__ import annotations

import importlib.util
import os
import stat
import tempfile
import unittest
from unittest.mock import patch
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("test_real_bd", ROOT / "scripts" / "test_real_bd.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

OK_OUT = (
    "test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out\n"
    "test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
)


class RealBdQualification(unittest.TestCase):
    def test_database_mode_matches_presence_even_for_empty_value(self) -> None:
        self.assertEqual(MODULE.database_mode({}), "embedded")
        self.assertEqual(MODULE.database_mode({"SC_COMPOSE_TEST_DOLT_PORT": ""}), "server")
        self.assertEqual(MODULE.database_mode({"SC_COMPOSE_TEST_DOLT_PORT": "3317"}), "server")

    def test_invalid_arguments_are_rejected_before_bd(self) -> None:
        with patch.object(MODULE.sys, "argv", ["test_real_bd.py", "--invalid"]), \
             patch.object(MODULE, "resolve_bd") as resolve:
            self.assertEqual(MODULE.main(), 2)
            resolve.assert_not_called()

    def test_database_mode_selects_one_test_and_ordinary_mode_skips_it(self) -> None:
        result = subprocess.CompletedProcess([], 0, OK_OUT, "")
        with patch.object(MODULE.subprocess, "run", return_value=result) as spawn:
            self.assertEqual(MODULE.run_suites({}, {"database_mode": "server"}, database_mode=True), 0)
            args = spawn.call_args.args[0]
            self.assertIn("repour_clean_dirty_database_mode", args)
            self.assertNotIn("--skip", args)
        with patch.object(MODULE, "SUITES", (("graph", []),)), \
             patch.object(MODULE.subprocess, "run", return_value=result) as spawn:
            self.assertEqual(MODULE.run_suites({}, {"database_mode": "embedded"}), 0)
            args = spawn.call_args.args[0]
            self.assertIn("--skip", args)
            self.assertIn("repour_clean_dirty_database_mode", args)

    def test_timed_out_cargo_reports_partial_output_and_fails(self) -> None:
        from contextlib import redirect_stderr
        from io import StringIO

        output = StringIO()
        error = subprocess.TimeoutExpired(["cargo"], 600, output=b"partial out", stderr=b"partial err")
        with patch.object(MODULE, "SUITES", (("graph", []),)), \
             patch.object(MODULE.subprocess, "run", side_effect=error), redirect_stderr(output):
            self.assertEqual(MODULE.run_suites({}, {"database_mode": "server"}), 1)
        self.assertIn("partial out", output.getvalue())
        self.assertIn("partial err", output.getvalue())

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

    def test_each_suite_result_records_version_hash_and_mode(self) -> None:
        from contextlib import redirect_stdout
        from io import StringIO

        provenance = {"bd_version": "bd 1.3", "bd_sha256": "abc", "database_mode": "server"}
        output = StringIO()
        with patch.object(MODULE, "SUITES", (("graph", []),)), \
             patch.object(MODULE.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, OK_OUT, "")), \
             redirect_stdout(output):
            self.assertEqual(MODULE.run_suites({}, provenance), 0)
        self.assertIn('"bd_version": "bd 1.3"', output.getvalue())
        self.assertIn('"bd_sha256": "abc"', output.getvalue())
        self.assertIn('"database_mode": "server"', output.getvalue())

    def test_unsupported_database_mode_cannot_qualify(self) -> None:
        from contextlib import redirect_stdout
        from io import StringIO

        output = "SC_COMPOSE_MODE_UNSUPPORTED: BEADS_GRAPH_DIRTY_UNKNOWN\n" + OK_OUT
        captured = StringIO()
        with patch.object(MODULE.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, output, "")), redirect_stdout(captured):
            self.assertEqual(MODULE.run_suites({}, {"database_mode": "server", "bd_version": "bd 1.3.1"}, database_mode=True), 0)
        self.assertIn('"qualification": "not_qualified"', captured.getvalue())
        self.assertIn('"ok": false', captured.getvalue())

    def test_counts_sum_across_result_lines(self) -> None:
        self.assertEqual(MODULE.parse_counts(OK_OUT), (15, 0, 1))

    def test_zero_executed_is_not_a_pass(self) -> None:
        zero = "test result: ok. 0 passed; 0 failed; 15 ignored; 0 measured\n"
        self.assertIn("0 passed", MODULE.verdict(0, MODULE.parse_counts(zero)))

    def test_missing_result_lines_is_not_a_pass(self) -> None:
        self.assertIsNotNone(MODULE.verdict(0, MODULE.parse_counts("")))

    def test_failures_and_nonzero_exit_fail(self) -> None:
        self.assertIsNotNone(MODULE.verdict(101, (3, 1, 0)))
        self.assertIsNotNone(MODULE.verdict(1, (3, 0, 0)))

    def test_clean_run_passes(self) -> None:
        self.assertIsNone(MODULE.verdict(0, MODULE.parse_counts(OK_OUT)))

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
