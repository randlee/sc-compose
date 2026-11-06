"""Release target -> hosted runner mapping must name only supported images.

GitHub retired the macOS 14 images; the Apple Silicon target needs an arm64
image and the Intel target an x86_64 image. These checks read the committed
source (install.json) and its generated manifest, with no network access.
"""

import json
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EXPECTED_DARWIN = {
    "aarch64-apple-darwin": "macos-15",
    "x86_64-apple-darwin": "macos-15-intel",
}
RETIRED = {"macos-12", "macos-13", "macos-14"}


def source_targets():
    data = json.loads((ROOT / "release/sc-publish-install.json").read_text())
    return {item["target"]: item["os"] for item in data["release_targets"]}


def manifest_targets():
    data = tomllib.loads((ROOT / "release/publish-artifacts.toml").read_text())
    return {item["target"]: item["os"] for item in data["release_targets"]}


class ReleaseTargetRunnerTests(unittest.TestCase):
    def test_darwin_targets_use_architecture_correct_runners(self):
        for label, targets in (("source", source_targets()), ("manifest", manifest_targets())):
            for target, runner in EXPECTED_DARWIN.items():
                self.assertEqual(targets[target], runner, f"{label}:{target}")

    def test_no_retired_runner_label_in_source_or_manifest(self):
        for label, targets in (("source", source_targets()), ("manifest", manifest_targets())):
            for target, runner in targets.items():
                self.assertNotIn(runner, RETIRED, f"{label}:{target}")

    def test_generated_manifest_agrees_with_source(self):
        self.assertEqual(source_targets(), manifest_targets())


if __name__ == "__main__":
    unittest.main()
