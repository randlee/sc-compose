"""Database-free tests for fuzz options and failure result expansion."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "integration_repour", ROOT / "scripts" / "integration_repour.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FuzzRunnerContractTests(unittest.TestCase):
    def test_fuzz_argument_validation_and_default_seeds(self) -> None:
        self.assertEqual(MODULE.fuzz_seeds_from_args(False, None), ())
        self.assertEqual(MODULE.fuzz_seeds_from_args(True, None), (24034, 24035))
        self.assertEqual(MODULE.fuzz_seeds_from_args(True, [7]), (7,))
        self.assertEqual(MODULE.fuzz_seeds_from_args(True, [7, 8]), (7, 8))
        with self.assertRaisesRegex(ValueError, "requires --fuzz"):
            MODULE.fuzz_seeds_from_args(False, [7])
        with self.assertRaisesRegex(ValueError, "at most two seeds"):
            MODULE.fuzz_seeds_from_args(True, [7, 8, 9])

    def test_failure_expansion_adds_only_missing_scenarios_in_order(self) -> None:
        results = [
            ("setup", "PASS", ""),
            ("changed pour", "FAIL", "controlled failure"),
        ]
        expected = ("setup", "changed pour", "fuzz seed 7", "teardown")

        self.assertEqual(
            MODULE.expand_unrun_results(results, expected),
            [
                ("setup", "PASS", ""),
                ("changed pour", "FAIL", "controlled failure"),
                ("fuzz seed 7", "NOT RUN", "A prior scenario failed"),
                ("teardown", "NOT RUN", "A prior scenario failed"),
            ],
        )


if __name__ == "__main__":
    unittest.main()
