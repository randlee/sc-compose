"""Hermetic tests for source and installed go-native-module helper layouts."""

from __future__ import annotations

import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HELPER = ROOT / "go_native_module.py"
SPEC = importlib.util.spec_from_file_location("go_native_module", HELPER)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

TARGETS = [
    ("x86_64-unknown-linux-gnu", "linux", "amd64", "ubuntu-latest", "tar.gz"),
    ("aarch64-apple-darwin", "darwin", "arm64", "macos-14", "tar.gz"),
    ("x86_64-pc-windows-gnu", "windows", "amd64", "windows-latest", "zip"),
]


def write_consumer(root: Path) -> tuple[Path, Path, Path]:
    """Build one minimal but complete consumer fixture and return config/manifest/workspace."""
    source = root / "bindings" / "sc-sha-go"
    (source / "go" / "sc_sha_go").mkdir(parents=True)
    (source / "testdata").mkdir()
    (source / "native").mkdir()
    (source / "go.mod").write_text("module github.com/example/sc-sha-go\n", encoding="utf-8")
    (source / "README.md").write_text("# sc-sha-go\n", encoding="utf-8")
    (source / "Cargo.toml").write_text(
        "[package]\nname = \"sc-sha-go\"\nversion.workspace = true\n", encoding="utf-8"
    )
    (source / "go" / "sc_sha_go" / "sc_sha_go.go").write_text("package sc_sha_go\n", encoding="utf-8")
    (source / "go" / "sc_sha_go" / "sc_sha_go.h").write_text("/* fixture */\n", encoding="utf-8")
    (source / "testdata" / "conformance.json").write_text("{}\n", encoding="utf-8")
    target_toml = [
        "schema_version = 1",
        "",
        "[contract]",
        "generated_package = \"go/sc_sha_go\"",
        "native_library = \"libsc_sha_go.a\"",
    ]
    release_toml = ["schema_version = 1"]
    for target, goos, goarch, os_name, archive in TARGETS:
        target_toml.extend(
            [
                "",
                "[[targets]]",
                f"rust_target = \"{target}\"",
                f"goos = \"{goos}\"",
                f"goarch = \"{goarch}\"",
                'library = "libsc_sha_go.a"',
            ]
        )
        release_toml.extend(
            [
                "",
                "[[release_targets]]",
                f"target = \"{target}\"",
                f"os = \"{os_name}\"",
                f"archive = \"{archive}\"",
            ]
        )
    (source / "native" / "targets.toml").write_text("\n".join(target_toml) + "\n", encoding="utf-8")
    release = root / "release"
    release.mkdir()
    config = release / "go-native-module.toml"
    config.write_text(
        "schema_version = 1\n"
        'source = "bindings/sc-sha-go"\n'
        'cargo_package = "sc-sha-go"\n'
        'artifact_prefix = "sc-sha-go"\n',
        encoding="utf-8",
    )
    manifest = release / "publish-artifacts.toml"
    manifest.write_text("\n".join(release_toml) + "\n", encoding="utf-8")
    workspace = root / "Cargo.toml"
    workspace.write_text("[workspace.package]\nversion = \"1.6.0\"\n", encoding="utf-8")
    return config, manifest, workspace


class GoNativeModuleTests(unittest.TestCase):
    def test_supported_targets_emit_exact_stable_matrix(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            config, manifest, _workspace = write_consumer(Path(directory))
            result = MODULE.target_matrix(manifest, config)
            self.assertEqual(
                result,
                {
                    "include": [
                        {
                            "target": target,
                            "os": os_name,
                            "archive": archive,
                            "goos": goos,
                            "goarch": goarch,
                            "library": "libsc_sha_go.a",
                            "cargo_package": "sc-sha-go",
                            "module": "github.com/example/sc-sha-go",
                            "artifact_prefix": "sc-sha-go",
                        }
                        for target, goos, goarch, os_name, archive in TARGETS
                    ]
                },
            )
            first = json.dumps(result, separators=(",", ":"))
            self.assertEqual(first, json.dumps(MODULE.target_matrix(manifest, config), separators=(",", ":")))
            cli = subprocess.run(
                [sys.executable, str(HELPER), "target-matrix", "--manifest", str(manifest), "--config", str(config)],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(cli.returncode, 0, cli.stderr)
            self.assertEqual(cli.stdout, first + "\n")

    def test_matrix_rejects_missing_generic_target_and_runner_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config, manifest, _workspace = write_consumer(root)
            data = tomllib.loads(manifest.read_text(encoding="utf-8"))
            data["release_targets"].pop()
            manifest.write_text("schema_version = 1\n", encoding="utf-8")
            for target in data["release_targets"]:
                manifest.write_text(
                    manifest.read_text(encoding="utf-8")
                    + "\n[[release_targets]]\n"
                    + "\n".join(f'{key} = "{value}"' for key, value in target.items())
                    + "\n",
                    encoding="utf-8",
                )
            with self.assertRaisesRegex(MODULE.ContractError, "lacks a generic"):
                MODULE.target_matrix(manifest, config)
            write_consumer(root / "second")
            config2 = root / "second" / "release" / "go-native-module.toml"
            manifest2 = root / "second" / "release" / "publish-artifacts.toml"
            malformed = manifest2.read_text(encoding="utf-8").replace('os = "ubuntu-latest"', 'os = "windows-latest"')
            manifest2.write_text(malformed, encoding="utf-8")
            with self.assertRaisesRegex(MODULE.ContractError, "runner/archive"):
                MODULE.target_matrix(manifest2, config2)

    def test_matrix_rejects_duplicate_or_malformed_binding_targets_without_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config, manifest, _workspace = write_consumer(root)
            targets = root / "bindings" / "sc-sha-go" / "native" / "targets.toml"
            targets.write_text(
                targets.read_text(encoding="utf-8")
                + "\n[[targets]]\n"
                + 'rust_target = "x86_64-unknown-linux-gnu"\n'
                + 'goos = "linux"\n'
                + 'goarch = "amd64"\n'
                + 'library = "libsc_sha_go.a"\n',
                encoding="utf-8",
            )
            command = [sys.executable, str(HELPER), "target-matrix", "--manifest", str(manifest), "--config", str(config)]
            result = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")
            self.assertIn("duplicate", result.stderr)
            fresh = root / "fresh"
            config2, manifest2, _workspace2 = write_consumer(fresh)
            broken = fresh / "bindings" / "sc-sha-go" / "native" / "targets.toml"
            broken.write_text(broken.read_text(encoding="utf-8").replace('goarch = "amd64"', "", 1), encoding="utf-8")
            with self.assertRaisesRegex(MODULE.ContractError, "goarch"):
                MODULE.target_matrix(manifest2, config2)

    def test_stage_is_complete_and_rejects_invalid_inputs_without_partial_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config, _manifest, _workspace = write_consumer(root)
            library = root / "libsc_sha_go.a"
            library.write_bytes(b"archive")
            output = root / "module"
            staged = MODULE.stage(config, TARGETS[0][0], library, output, "1.6.0")
            self.assertEqual(staged, output.resolve())
            self.assertEqual((output / "VERSION").read_text(encoding="utf-8"), "1.6.0\n")
            self.assertTrue((output / "go" / "sc_sha_go" / "sc_sha_go.go").is_file())
            self.assertTrue((output / "testdata" / "conformance.json").is_file())
            self.assertTrue((output / "native" / TARGETS[0][0] / "libsc_sha_go.a").is_file())
            with self.assertRaisesRegex(MODULE.ContractError, "already exists"):
                MODULE.stage(config, TARGETS[0][0], library, output, "1.6.0")
            wrong = root / "wrong.a"
            wrong.write_bytes(b"archive")
            failed = root / "failed"
            with self.assertRaisesRegex(MODULE.ContractError, "name mismatch"):
                MODULE.stage(config, TARGETS[0][0], wrong, failed, "1.6.0")
            self.assertFalse(failed.exists())
            with self.assertRaisesRegex(MODULE.ContractError, "does not exist"):
                MODULE.stage(config, TARGETS[0][0], root / "missing" / "libsc_sha_go.a", root / "absent", "1.6.0")
            self.assertFalse((root / "absent").exists())
            with self.assertRaisesRegex(MODULE.ContractError, "semantic version"):
                MODULE.stage(config, TARGETS[0][0], library, root / "bad-version", "v1.6.0")
            with self.assertRaisesRegex(MODULE.ContractError, "consumer root"):
                MODULE.stage(config, TARGETS[0][0], library, root, "1.6.0")

    def test_stage_rejects_missing_source_asset_and_unsafe_config_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config, _manifest, _workspace = write_consumer(root)
            library = root / "libsc_sha_go.a"
            library.write_bytes(b"archive")
            shutil.rmtree(root / "bindings" / "sc-sha-go" / "testdata")
            with self.assertRaisesRegex(MODULE.ContractError, "source asset"):
                MODULE.stage(config, TARGETS[0][0], library, root / "output", "1.6.0")
            config.write_text(
                "schema_version = 1\nsource = \"../escape\"\ncargo_package = \"sc-sha-go\"\nartifact_prefix = \"sc-sha-go\"\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(MODULE.ContractError, "safe path"):
                MODULE.load_config(config)

    def test_lockstep_and_source_relationships(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config, _manifest, workspace = write_consumer(root)
            self.assertEqual(MODULE.verify_version_lockstep(config, workspace), "1.6.0")
            cargo = root / "bindings" / "sc-sha-go" / "Cargo.toml"
            cargo.write_text('[package]\nname = "sc-sha-go"\nversion = "9.9.9"\n', encoding="utf-8")
            with self.assertRaisesRegex(MODULE.ContractError, "inherit workspace"):
                MODULE.verify_version_lockstep(config, workspace)
            cargo.write_text('[package]\nname = "other"\nversion.workspace = true\n', encoding="utf-8")
            with self.assertRaisesRegex(MODULE.ContractError, "does not match"):
                MODULE.load_config(config)
            cargo.write_text('[package]\nname = "sc-sha-go"\nversion.workspace = true\n', encoding="utf-8")
            go_mod = root / "bindings" / "sc-sha-go" / "go.mod"
            go_mod.write_text("not a module\n", encoding="utf-8")
            with self.assertRaisesRegex(MODULE.ContractError, "module declaration"):
                MODULE.load_config(config)


if __name__ == "__main__":
    unittest.main()
