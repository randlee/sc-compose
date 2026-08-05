#!/usr/bin/env python3
"""Validate output paths are confined to an approved existing directory."""

from pathlib import Path
import sys


def main() -> int:
    if len(sys.argv) < 3:
        print(
            f"usage: {Path(sys.argv[0]).name} APPROVED_ROOT OUTPUT_PATH [...]",
            file=sys.stderr,
        )
        return 2

    root = Path(sys.argv[1]).expanduser()
    try:
        root = root.resolve(strict=True)
    except FileNotFoundError:
        print(f"approved root does not exist: {root}", file=sys.stderr)
        return 2
    if not root.is_dir():
        print(f"approved root is not a directory: {root}", file=sys.stderr)
        return 2

    for raw_path in sys.argv[2:]:
        candidate = Path(raw_path).expanduser()
        if not candidate.is_absolute():
            candidate = root / candidate
        if ".." in candidate.parts:
            print(f"path traversal is not allowed: {raw_path}", file=sys.stderr)
            return 1
        resolved = candidate.resolve(strict=False)
        if resolved == root or root not in resolved.parents:
            print(f"output path escapes approved root: {raw_path}", file=sys.stderr)
            return 1
        print(resolved)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
