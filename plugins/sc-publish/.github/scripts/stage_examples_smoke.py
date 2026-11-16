#!/usr/bin/env python3
"""Run manifest-declared checks against binaries in a staged release archive."""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import tomllib


def run_checks(stage: Path, target: str, manifest: dict) -> None:
    stage = stage.resolve(strict=True)
    env = dict(os.environ)
    env.pop("SC_COMPOSE_DATA_DIR", None)
    for binary in manifest.get("release_binaries", []):
        executable = stage / "bin" / (binary["name"] + (".exe" if "windows" in target else ""))
        for bundle in binary.get("bundled_paths", []):
            args = bundle.get("stage_smoke_args")
            expected = bundle.get("stage_smoke_line")
            if args is None and expected is None:
                continue
            if not isinstance(args, list) or not args or not all(isinstance(arg, str) for arg in args):
                raise ValueError("stage_smoke_args must be a non-empty string array")
            if not isinstance(expected, str) or not expected:
                raise ValueError("stage_smoke_line must be a non-empty string")
            result = subprocess.run(
                [str(executable), *args], cwd=stage, env=env, capture_output=True,
                text=True, timeout=120, check=False,
            )
            if result.returncode != 0 or expected not in result.stdout.splitlines():
                raise RuntimeError(
                    f"staged smoke check failed: exit={result.returncode}; "
                    f"expected={expected!r}; stdout={result.stdout!r}; stderr={result.stderr!r}"
                )
            print(f"staged smoke passed: {executable.name} {' '.join(args)} lists {expected!r}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", type=Path)
    parser.add_argument("target")
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    with args.manifest.open("rb") as handle:
        manifest = tomllib.load(handle)
    run_checks(args.stage, args.target, manifest)


if __name__ == "__main__":
    main()
