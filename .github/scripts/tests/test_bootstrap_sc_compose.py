"""Regression tests for the consumer-owned sc-compose renderer bootstrap."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
BOOTSTRAP = SCRIPTS / "bootstrap_sc_compose.py"
SPEC = importlib.util.spec_from_file_location("bootstrap_sc_compose", BOOTSTRAP)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class BootstrapScComposeTests(unittest.TestCase):
    def test_floor_rejects_a_stale_wheel_before_binding_import(self) -> None:
        with self.assertRaisesRegex(
            SystemExit,
            r"incompatible sc-compose wheel: stale version '1\.4\.0'; required >= 1\.4\.1",
        ):
            MODULE.require_version_floor("1.4.0")

    def test_floor_accepts_the_floor_and_newer_wheels(self) -> None:
        MODULE.require_version_floor("1.4.1")
        MODULE.require_version_floor("1.5.0")

    def test_floor_is_documented_as_a_historical_compatibility_pin(self) -> None:
        text = BOOTSTRAP.read_text(encoding="utf-8")
        self.assertIn("historical compatibility floor", text)
        self.assertIn("bindings/python/pyproject.toml", text)

    def test_version_probe_reads_distribution_metadata_before_binding_import(self) -> None:
        text = BOOTSTRAP.read_text(encoding="utf-8")
        probe = text[text.index("def installed_version"):text.index("def version_components")]
        self.assertIn("from importlib.metadata import version", probe)
        self.assertNotIn("import sc_compose", probe)


if __name__ == "__main__":
    unittest.main()
