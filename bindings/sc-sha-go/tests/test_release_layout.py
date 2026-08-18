#!/usr/bin/env python3
"""Verify the generated Go package's static native release layout."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
MANIFEST = REPOSITORY_ROOT / "release" / "publish-artifacts.toml"
SCRIPT = REPOSITORY_ROOT / "scripts" / "release_artifacts.py"
BINDING_ROOT = REPOSITORY_ROOT / "bindings" / "sc-sha-go"


def go_native_targets() -> list[dict[str, str]]:
    data = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    return data["go_native"]["targets"]


class ReleaseLayoutTests(unittest.TestCase):
    def stage(
        self, target: dict[str, str], library: Path, output: Path
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "stage-go-native-module",
                "--manifest",
                str(MANIFEST),
                "--target",
                target["rust_target"],
                "--native-library",
                str(library),
                "--output",
                str(output),
                "--version",
                "1.4.1",
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def test_target_contract_matches_cgo_loader(self) -> None:
        loader = (BINDING_ROOT / "go" / "sc_sha_go" / "native_loader.go").read_text(
            encoding="utf-8"
        )
        contract = tomllib.loads(
            (BINDING_ROOT / "native" / "targets.toml").read_text(encoding="utf-8")
        )
        self.assertEqual(contract["targets"], go_native_targets())
        for target in go_native_targets():
            self.assertIn(
                f"${{SRCDIR}}/../../native/{target['rust_target']}/{target['library']}",
                loader,
            )
        self.assertIn("unsupported native target", loader)

    def test_every_advertised_target_stages_exactly_one_matching_library(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for target in go_native_targets():
                with self.subTest(target=target["rust_target"]):
                    library = root / target["library"]
                    library.write_bytes(b"fixture static library")
                    output = root / target["rust_target"]
                    result = self.stage(target, library, output)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertTrue((output / "go.mod").is_file())
                    self.assertTrue((output / "go" / "sc_sha_go" / "sc_sha_go.go").is_file())
                    self.assertTrue((output / "go" / "sc_sha_go" / "sc_sha_go.h").is_file())
                    self.assertTrue((output / "testdata" / "conformance-v1.json").is_file())
                    native_root = output / "native"
                    self.assertTrue((native_root / "targets.toml").is_file())
                    self.assertEqual(
                        list(native_root.glob("*/*")),
                        [native_root / target["rust_target"] / target["library"]],
                    )

    def test_stage_rejects_a_library_for_a_different_target(self) -> None:
        targets = go_native_targets()
        linux = next(target for target in targets if target["goos"] == "linux")
        windows = next(target for target in targets if target["goos"] == "windows")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            mismatched_library = root / windows["library"]
            mismatched_library.write_bytes(b"wrong target")
            result = self.stage(linux, mismatched_library, root / "output")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("native library name mismatch", result.stderr)


if __name__ == "__main__":
    unittest.main()
