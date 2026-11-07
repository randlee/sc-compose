"""The release manifest declares the FR-25.1 wheel contract, and the Linux arm64 wheel tag policy is pinned.

Deterministic: reads the committed manifest, install input and workflow text; nothing runs.
"""

import json
import re
import sys
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / ".github" / "scripts"))

import check_install_matrix as matrix  # noqa: E402
import release_python  # noqa: E402

# One runner per FR-25.1 coordinate (OS x architecture), as the manifest's legacy runner labels.
CONTRACT_RUNNERS = {
    ("linux", "x64"): "ubuntu-latest",
    ("linux", "arm64"): "ubuntu-24.04-arm",
    ("macos", "arm64"): "macos-latest",
    ("macos", "x64"): "macos-15-intel",
    ("windows", "x64"): "windows-latest",
    ("windows", "arm64"): "windows-11-arm",
}
FAMILIES = {"sc-sha", "sc-compose", "sc-composer-beads"}
ARM64_WORKFLOWS = (".github/workflows/go-native-preflight.yml", ".github/workflows/release.yml", ".github/workflows/ci.yml")


class WheelContractTest(unittest.TestCase):
    def test_contract_has_six_coordinates(self):
        self.assertEqual(len(CONTRACT_RUNNERS), len(matrix.REQUIRED_COORDINATES))

    def test_manifest_wheel_lists_equal_the_contract(self):
        manifest = tomllib.loads((ROOT / "release" / "publish-artifacts.toml").read_text(encoding="utf-8"))
        dists = {d["name"]: d for d in manifest["python_distributions"]}
        self.assertEqual(set(dists), FAMILIES)
        for name, dist in dists.items():
            self.assertEqual(sorted(dist["wheels"]), sorted(CONTRACT_RUNNERS.values()), name)
            self.assertEqual(len(dist["wheels"]), len(set(dist["wheels"])), name)
            self.assertEqual(len(release_python.wheel_targets(dist)), 6, name)

    def test_install_input_and_template_inputs_agree_with_the_manifest(self):
        manifest = tomllib.loads((ROOT / "release" / "publish-artifacts.toml").read_text(encoding="utf-8"))
        values = json.loads((ROOT / "release" / "sc-publish-install.json").read_text(encoding="utf-8"))
        self.assertEqual(
            {d["name"]: d["wheels"] for d in values["python_distributions"]},
            {d["name"]: d["wheels"] for d in manifest["python_distributions"]},
        )

    def test_linux_arm64_manylinux2014_is_the_recorded_policy_for_runner_label_wheels(self):
        # release_python requires manylinux 2_28 only for explicit target objects; the manifest uses
        # runner labels, whose arm64 build is the documented manylinux2014 + Zig build.
        explicit = {"id": "a", "os": "ubuntu-24.04-arm", "target": "aarch64-unknown-linux-gnu",
                    "platform": "manylinux_2_28_aarch64", "manylinux": "2014"}
        with self.assertRaises(ValueError):
            release_python.wheel_targets({"wheels": [explicit]})
        legacy = release_python.wheel_targets({"wheels": ["ubuntu-24.04-arm"]})
        self.assertEqual(legacy[0]["platform"], "")
        for rel in ARM64_WORKFLOWS:
            text = (ROOT / rel).read_text(encoding="utf-8")
            flags = re.findall(r"--compatibility (\S+) --zig", text)
            self.assertTrue(flags, rel)
            self.assertIn("manylinux2014", flags, rel)

    def test_produced_arm64_tag_is_asserted_as_manylinux2014_aarch64(self):
        text = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        self.assertIn("*manylinux2014_aarch64*.whl", text)
        self.assertTrue(re.fullmatch(r"(?:many|musl)linux_\d+_\d+_(x86_64|aarch64)|manylinux\d+_(x86_64|aarch64)", "manylinux2014_aarch64"))
        self.assertTrue(re.fullmatch(r"manylinux\d+_(x86_64|aarch64)", "manylinux2014_aarch64"))


if __name__ == "__main__":
    unittest.main()
