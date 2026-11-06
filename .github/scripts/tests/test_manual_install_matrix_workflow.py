"""Structure checks for the manual install-matrix producer in go-native-preflight.yml.

Deterministic: parses the workflow and canonical contracts only; nothing runs.
"""

import sys
import tomllib
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import check_install_matrix as matrix  # noqa: E402

WORKFLOW = ROOT / ".github" / "workflows" / "go-native-preflight.yml"


class ManualInstallMatrixWorkflow(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        cls.jobs = cls.workflow["jobs"]

    def test_runs_only_by_manual_dispatch(self):
        self.assertEqual(self.workflow[True], {"workflow_dispatch": None})

    def test_managed_release_preflight_is_not_the_entrypoint(self):
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertNotIn("release-preflight", text)

    def test_wheel_matrix_covers_each_distribution_on_six_coordinates(self):
        manifest = tomllib.loads((ROOT / "release" / "publish-artifacts.toml").read_text(encoding="utf-8"))
        declared = {d["name"] for d in manifest["python_distributions"]}
        strategy = self.jobs["matrix-wheels"]["strategy"]["matrix"]
        self.assertEqual({d["name"] for d in strategy["dist"]}, declared)
        for dist in strategy["dist"]:
            self.assertTrue((ROOT / dist["cargo_manifest"]).is_file())
        runners = {c["id"]: c["os"] for c in strategy["coordinate"]}
        self.assertEqual(
            set(runners),
            {"linux-x64", "linux-arm64", "macos-arm64", "macos-x64", "windows-x64", "windows-arm64"},
        )
        self.assertEqual(len(declared) * len(runners), 18)

    def test_collect_publishes_the_artifact_names_the_verifier_expects(self):
        steps = self.jobs["matrix-collect"]["steps"]
        uploaded = {s["with"]["name"] for s in steps if str(s.get("uses", "")).startswith("actions/upload-artifact")}
        self.assertEqual(uploaded, {matrix.FILES_ARTIFACT, matrix.CHECKSUMS_ARTIFACT})
        for step in steps:
            if str(step.get("uses", "")).startswith("actions/upload-artifact"):
                self.assertEqual(step["with"]["if-no-files-found"], "error")

    def test_collect_waits_for_every_cell_producer(self):
        self.assertEqual(
            set(self.jobs["matrix-collect"]["needs"]),
            {"matrix-plan", "matrix-rust", "matrix-wheels", "go-native-matrix"},
        )

    def test_go_cells_upload_the_name_the_verifier_classifies(self):
        upload = [s for s in self.jobs["go-native-matrix"]["steps"] if str(s.get("uses", "")).startswith("actions/upload-artifact")]
        self.assertEqual(len(upload), 1)
        self.assertTrue(upload[0]["with"]["name"].startswith("matrix-cell-"))
        self.assertTrue(upload[0]["with"]["path"].endswith("${{ matrix.artifact_prefix }}-${{ matrix.target }}.zip"))

    def test_gnullvm_toolchain_is_pinned_by_digest_in_the_go_job_only(self):
        step = next(s for s in self.jobs["go-native-matrix"]["steps"] if "LLVM-MinGW" in s.get("name", ""))
        self.assertEqual(len(step["env"]["LLVM_MINGW_SHA256"]), 64)
        self.assertIn("gnullvm", step["if"])
        # The Rust CLI gnullvm coupling is tracked separately (sc-publish#164), not worked around here.
        self.assertFalse([s for s in self.jobs["matrix-rust"]["steps"] if "LLVM-MinGW" in s.get("name", "")])


if __name__ == "__main__":
    unittest.main()
