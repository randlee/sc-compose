#!/usr/bin/env python3
"""Build and stage the host archive for source-tree Go tests via the canonical peer."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def run(argv: list[str], root: Path) -> str:
    return subprocess.run(
        argv, cwd=root, check=True, text=True, stdout=subprocess.PIPE
    ).stdout.strip()


def select_target(contract: dict, go: dict[str, str]) -> dict[str, str]:
    """Select the declared CGo ABI for the native Go host, rejecting cross execution."""
    if (go["GOOS"], go["GOARCH"]) != (go["GOHOSTOS"], go["GOHOSTARCH"]):
        raise ValueError(
            "source-tree preparation requires GOOS/GOARCH to match the Go host"
        )
    matches = [
        target
        for target in contract["targets"]
        if (target["goos"], target["goarch"]) == (go["GOHOSTOS"], go["GOHOSTARCH"])
    ]
    if len(matches) != 1:
        raise ValueError(
            f"unsupported or ambiguous sc-sha-go host: {go['GOHOSTOS']}/{go['GOHOSTARCH']}"
        )
    return matches[0]


def prepare(root: Path) -> Path:
    source = root / "bindings/sc-sha-go"
    contract = tomllib.loads(
        (source / "native/targets.toml").read_text(encoding="utf-8")
    )
    go = json.loads(
        run(["go", "env", "-json", "GOOS", "GOARCH", "GOHOSTOS", "GOHOSTARCH"], root)
    )
    target = select_target(contract, go)
    metadata = json.loads(
        run(["cargo", "metadata", "--format-version", "1", "--no-deps"], root)
    )
    package = next(
        package for package in metadata["packages"] if package["name"] == "sc-sha-go"
    )
    triple = target["rust_target"]
    run(["cargo", "build", "-p", "sc-sha-go", "--target", triple], root)
    library = Path(metadata["target_directory"]) / triple / "debug" / target["library"]
    # Stage outside the binding source: the peer validates the archive name and
    # complete module layout before we install its archive at the loader path.
    with tempfile.TemporaryDirectory(prefix="sc-sha-go-native-") as directory:
        staged = Path(directory) / "module"
        run(
            [
                sys.executable,
                str(root / ".github/scripts/go_native_module.py"),
                "stage",
                "--config",
                str(root / "release/go-native-module.toml"),
                "--target",
                triple,
                "--native-library",
                str(library),
                "--output",
                str(staged),
                "--version",
                package["version"],
            ],
            root,
        )
        destination = source / "native" / triple / target["library"]
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(staged / "native" / triple / target["library"], destination)
    return destination


def main() -> int:
    try:
        print(prepare(ROOT))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"sc-sha-go native preparation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
