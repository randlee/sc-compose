#!/usr/bin/env python3
"""Verify the generated Go package's static native release layout."""

from __future__ import annotations

import importlib.util
import json
import os
import re
import shlex
import subprocess
import sys
import tempfile
import tomllib
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
# Controlled unit fixture; it is not the shipped release declaration.
MANIFEST = Path(__file__).parent / "fixtures" / "release-targets.toml"
SHIPPED_MANIFEST = REPOSITORY_ROOT / "release" / "publish-artifacts.toml"
CONFIG = REPOSITORY_ROOT / "release" / "go-native-module.toml"
SCRIPT = REPOSITORY_ROOT / ".github" / "scripts" / "go_native_module.py"
BINDING_ROOT = REPOSITORY_ROOT / "bindings" / "sc-sha-go"


def target_matrix(manifest: Path = MANIFEST) -> list[dict[str, str]]:
    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "target-matrix",
            "--manifest",
            str(manifest),
            "--config",
            str(CONFIG),
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    return json.loads(result.stdout)["include"]


def go_native_targets(manifest: Path = MANIFEST) -> list[dict[str, str]]:
    return [
        {
            "rust_target": target["target"],
            "goos": target["goos"],
            "goarch": target["goarch"],
            "library": target["library"],
        }
        for target in target_matrix(manifest)
    ]


def rust_target_coordinate(triple: str) -> tuple[str, str]:
    """Go OS/architecture of a Rust target triple used by the release manifest."""
    goarch = {"x86_64": "amd64", "aarch64": "arm64"}[triple.split("-", 1)[0]]
    for marker, goos in (
        ("-linux-", "linux"),
        ("-apple-darwin", "darwin"),
        ("-windows-", "windows"),
    ):
        if marker in triple:
            return goos, goarch
    raise ValueError(f"unmapped release target: {triple}")


class ReleaseLayoutTests(unittest.TestCase):
    def stage(
        self, target: dict[str, str], library: Path, output: Path
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "stage",
                "--config",
                str(CONFIG),
                "--target",
                target["rust_target"],
                "--native-library",
                str(library),
                "--output",
                str(output),
                "--version",
                # Staging validates a non-empty SemVer and writes it to VERSION;
                # workspace/binding version lockstep is covered independently.
                "1.5.0",
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

    def test_all_six_coordinates_have_exact_cgo_archive_selection(self) -> None:
        targets = go_native_targets()
        self.assertEqual(
            {(t["goos"], t["goarch"]) for t in targets},
            {
                (os_name, arch)
                for os_name in ("linux", "darwin", "windows")
                for arch in ("amd64", "arm64")
            },
        )
        self.assertEqual(len(targets), 6)
        windows = {
            t["goarch"]: (t["rust_target"], t["library"])
            for t in targets
            if t["goos"] == "windows"
        }
        self.assertEqual(
            windows,
            {
                "amd64": ("x86_64-pc-windows-gnu", "libsc_sha_go.a"),
                "arm64": ("aarch64-pc-windows-gnullvm", "libsc_sha_go.a"),
            },
        )
        for target in targets:
            with self.subTest(target=target["rust_target"]):
                env = {
                    **os.environ,
                    "GOOS": target["goos"],
                    "GOARCH": target["goarch"],
                    "CGO_ENABLED": "1",
                    "GOWORK": "off",
                    "GOFLAGS": "",
                }
                result = subprocess.run(
                    ["go", "list", "-json", "./go/sc_sha_go"],
                    cwd=BINDING_ROOT,
                    env=env,
                    text=True,
                    capture_output=True,
                    check=True,
                )
                flags = json.loads(result.stdout)["CgoLDFLAGS"]
                archives = [flag for flag in flags if flag.endswith((".a", ".lib"))]
                self.assertEqual(len(archives), 1)
                self.assertEqual(
                    Path(archives[0]).resolve(),
                    BINDING_ROOT / "native" / target["rust_target"] / target["library"],
                )
                if target["goos"] == "windows":
                    for library in ("-lws2_32", "-lntdll", "-lbcrypt", "-luserenv"):
                        self.assertIn(library, flags)

    def test_loader_preprocessor_accepts_supported_compiler_macros(self) -> None:
        loader = (BINDING_ROOT / "go/sc_sha_go/native_loader.go").read_text()
        preamble = loader.split("/*", 1)[1].split("*/", 1)[0]
        preamble = re.sub(r"^#cgo.*$", "", preamble, flags=re.MULTILINE)
        compiler = shlex.split(os.environ.get("CC", "cc"))
        for os_macro in ("__linux__", "__APPLE__", "_WIN32"):
            for arch_macro in ("__x86_64__", "__aarch64__", "_M_X64", "_M_ARM64"):
                with self.subTest(os=os_macro, arch=arch_macro):
                    result = subprocess.run(
                        [
                            *compiler,
                            "-E",
                            "-x",
                            "c",
                            "-undef",
                            "-",
                            f"-D{os_macro}",
                            f"-D{arch_macro}",
                        ],
                        input=preamble,
                        text=True,
                        capture_output=True,
                        check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
        for macros in (("__linux__", "__i386__"), ("__FreeBSD__", "__x86_64__"), ()):
            result = subprocess.run(
                [
                    *compiler,
                    "-E",
                    "-x",
                    "c",
                    "-undef",
                    "-",
                    *[f"-D{macro}" for macro in macros],
                ],
                input=preamble,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("unsupported native target", result.stderr)

    def test_every_advertised_target_stages_exactly_one_matching_library(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for target in go_native_targets():
                with self.subTest(target=target["rust_target"]):
                    library = root / target["library"]
                    library.write_bytes(target["rust_target"].encode())
                    output = root / target["rust_target"]
                    result = self.stage(target, library, output)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertTrue((output / "go.mod").is_file())
                    self.assertTrue(
                        (output / "go" / "sc_sha_go" / "sc_sha_go.go").is_file()
                    )
                    self.assertTrue(
                        (output / "go" / "sc_sha_go" / "sc_sha_go.h").is_file()
                    )
                    self.assertTrue(
                        (output / "testdata" / "conformance-v1.json").is_file()
                    )
                    native_root = output / "native"
                    self.assertTrue((native_root / "targets.toml").is_file())
                    self.assertEqual(
                        list(native_root.glob("*/*")),
                        [native_root / target["rust_target"] / target["library"]],
                    )
                    # Exercise the target ZIP module channel without pretending
                    # the deterministic archive bytes are executable libraries.
                    archive = root / f"{target['rust_target']}.zip"
                    with zipfile.ZipFile(archive, "w") as bundle:
                        for entry in sorted(output.rglob("*")):
                            if entry.is_file():
                                bundle.write(entry, entry.relative_to(output))
                    with zipfile.ZipFile(archive) as bundle:
                        native_files = [
                            name
                            for name in bundle.namelist()
                            if name.endswith((".a", ".lib"))
                        ]
                        self.assertEqual(
                            native_files,
                            [f"native/{target['rust_target']}/{target['library']}"],
                        )
                        self.assertEqual(
                            bundle.read(native_files[0]), library.read_bytes()
                        )
                        self.assertEqual(bundle.read("VERSION"), b"1.5.0\n")

    def test_stage_rejects_a_library_for_a_different_target(self) -> None:
        targets = go_native_targets()
        linux = next(target for target in targets if target["goos"] == "linux")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            mismatched_library = root / "sc_sha_go.a"
            mismatched_library.write_bytes(b"wrong target")
            result = self.stage(linux, mismatched_library, root / "output")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("native library name mismatch", result.stderr)


class ShippedManifestTests(unittest.TestCase):
    """Integration coverage of the shipped release manifest, not the fixture."""

    def test_shipped_manifest_maps_every_native_target(self) -> None:
        contract = tomllib.loads(
            (BINDING_ROOT / "native" / "targets.toml").read_text(encoding="utf-8")
        )
        self.assertEqual(
            sorted(go_native_targets(SHIPPED_MANIFEST), key=lambda t: t["rust_target"]),
            sorted(contract["targets"], key=lambda t: t["rust_target"]),
        )

    def test_shipped_runners_are_declared_for_the_same_coordinate(self) -> None:
        declared = tomllib.loads(SHIPPED_MANIFEST.read_text(encoding="utf-8"))
        runners: dict[tuple[str, str], set[tuple[str, str]]] = {}
        for row in declared["release_targets"]:
            runners.setdefault(rust_target_coordinate(row["target"]), set()).add(
                (row["os"], row["archive"])
            )
        for target in target_matrix(SHIPPED_MANIFEST):
            with self.subTest(target=target["target"]):
                self.assertIn(
                    (target["os"], target["archive"]),
                    runners.get((target["goos"], target["goarch"]), set()),
                )


class SourcePreparationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        spec = importlib.util.spec_from_file_location(
            "prepare_native", BINDING_ROOT / "prepare_native.py"
        )
        cls.prepare = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.prepare)
        cls.contract = tomllib.loads((BINDING_ROOT / "native/targets.toml").read_text())

    def test_host_selection_covers_all_six_and_rejects_foreign_execution(self) -> None:
        for target in self.contract["targets"]:
            go = {
                "GOOS": target["goos"],
                "GOARCH": target["goarch"],
                "GOHOSTOS": target["goos"],
                "GOHOSTARCH": target["goarch"],
            }
            self.assertEqual(vars(self.prepare.select_target(self.contract, go)), target)
        with self.assertRaisesRegex(ValueError, "match the Go host"):
            self.prepare.select_target(
                self.contract,
                {
                    "GOOS": "windows",
                    "GOARCH": "arm64",
                    "GOHOSTOS": "darwin",
                    "GOHOSTARCH": "arm64",
                },
            )
        with self.assertRaisesRegex(ValueError, "unsupported"):
            self.prepare.select_target(
                self.contract,
                {
                    "GOOS": "freebsd",
                    "GOARCH": "amd64",
                    "GOHOSTOS": "freebsd",
                    "GOHOSTARCH": "amd64",
                },
            )

    def test_missing_target_fields_are_reported_as_contract_errors(self) -> None:
        target = self.contract["targets"][0]
        go = {
            "GOOS": target["goos"],
            "GOARCH": target["goarch"],
            "GOHOSTOS": target["goos"],
            "GOHOSTARCH": target["goarch"],
        }
        for key in ("rust_target", "goos", "goarch", "library"):
            with self.subTest(key=key):
                incomplete = dict(target)
                del incomplete[key]
                with self.assertRaisesRegex(
                    ValueError, f"native target 0 missing required key: {key}"
                ):
                    self.prepare.select_target({"targets": [incomplete]}, go)

    def test_preparation_uses_cargo_metadata_and_canonical_stage(self) -> None:
        for target in self.contract["targets"]:
            with self.subTest(
                target=target["rust_target"]
            ), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / "bindings/sc-sha-go"
                (source / "native").mkdir(parents=True)
                (source / "native/targets.toml").write_bytes(
                    (BINDING_ROOT / "native/targets.toml").read_bytes()
                )
                target_dir = root / "custom cargo output"
                calls = []

                def run(argv: list[str], cwd: Path) -> str:
                    calls.append(argv)
                    self.assertEqual(cwd, root)
                    if argv[:2] == ["go", "env"]:
                        return json.dumps(
                            {
                                "GOOS": target["goos"],
                                "GOARCH": target["goarch"],
                                "GOHOSTOS": target["goos"],
                                "GOHOSTARCH": target["goarch"],
                            }
                        )
                    if argv[:2] == ["cargo", "metadata"]:
                        return json.dumps(
                            {
                                "target_directory": str(target_dir),
                                "packages": [
                                    {"name": "sc-sha-go", "version": "1.8.0-rc.1"}
                                ],
                            }
                        )
                    if argv[:2] == ["cargo", "build"]:
                        self.assertEqual(
                            argv,
                            [
                                "cargo",
                                "build",
                                "-p",
                                "sc-sha-go",
                                "--target",
                                target["rust_target"],
                            ],
                        )
                        return ""
                    self.assertEqual(
                        argv[:3],
                        [
                            sys.executable,
                            str(root / ".github/scripts/go_native_module.py"),
                            "stage",
                        ],
                    )
                    args = dict(zip(argv[3::2], argv[4::2]))
                    self.assertEqual(
                        args["--config"], str(root / "release/go-native-module.toml")
                    )
                    self.assertEqual(args["--target"], target["rust_target"])
                    self.assertEqual(args["--version"], "1.8.0-rc.1")
                    self.assertEqual(
                        args["--native-library"],
                        str(
                            target_dir
                            / target["rust_target"]
                            / "debug"
                            / target["library"]
                        ),
                    )
                    staged = (
                        Path(args["--output"])
                        / "native"
                        / target["rust_target"]
                        / target["library"]
                    )
                    staged.parent.mkdir(parents=True)
                    staged.write_bytes(b"validated stage bytes")
                    return args["--output"]

                with patch.object(self.prepare, "run", side_effect=run):
                    destination = self.prepare.prepare(root)
                self.assertEqual(
                    destination,
                    source / "native" / target["rust_target"] / target["library"],
                )
                self.assertEqual(destination.read_bytes(), b"validated stage bytes")
                self.assertEqual(len(calls), 4)

    def test_failed_build_does_not_install_an_archive(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            native = root / "bindings/sc-sha-go/native"
            native.mkdir(parents=True)
            (native / "targets.toml").write_bytes(
                (BINDING_ROOT / "native/targets.toml").read_bytes()
            )
            responses = [
                json.dumps(
                    {
                        "GOOS": "linux",
                        "GOARCH": "amd64",
                        "GOHOSTOS": "linux",
                        "GOHOSTARCH": "amd64",
                    }
                ),
                json.dumps(
                    {
                        "target_directory": str(root / "target"),
                        "packages": [{"name": "sc-sha-go", "version": "1.8.0"}],
                    }
                ),
                subprocess.CalledProcessError(1, ["cargo", "build"]),
            ]
            with patch.object(self.prepare, "run", side_effect=responses) as run:
                with self.assertRaises(subprocess.CalledProcessError):
                    self.prepare.prepare(root)
            self.assertEqual(run.call_count, 3)
            self.assertEqual(list(native.iterdir()), [native / "targets.toml"])


if __name__ == "__main__":
    unittest.main()
