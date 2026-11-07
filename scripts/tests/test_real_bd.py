"""Deterministic tests for scripts/test_real_bd.py (controlled inputs only)."""
from __future__ import annotations

import importlib.util
import os
import stat
import tempfile
import unittest
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

    def test_executable_bd_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bd = Path(tmp) / "bd"
            bd.write_text("#!/bin/sh\n")
            bd.chmod(bd.stat().st_mode | stat.S_IXUSR)
            self.assertEqual(MODULE.resolve_bd({"BD_EXECUTABLE": str(bd)}), bd)


if __name__ == "__main__":
    unittest.main()
