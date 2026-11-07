"""Pin the Rust components needed by the consumer sc-lint workflow."""

from pathlib import Path
import unittest

import yaml


ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / ".github/workflows/sc-compose-lint.yml"


class ScComposeLintToolchainTests(unittest.TestCase):
    def test_pinned_toolchain_installs_fmt_and_clippy(self) -> None:
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        steps = workflow["jobs"]["sc-lint"]["steps"]
        rust_toolchain = next(
            step for step in steps if step.get("uses") == "dtolnay/rust-toolchain@master"
        )

        self.assertEqual(rust_toolchain["with"]["toolchain"], "1.94.1")
        components = {
            component.strip()
            for component in rust_toolchain["with"]["components"].split(",")
        }
        self.assertTrue({"rustfmt", "clippy"}.issubset(components))


if __name__ == "__main__":
    unittest.main()
