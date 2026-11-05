#!/usr/bin/env python3
"""Set up the consumer's canonical, version-pinned sc-lint runtime."""
from __future__ import annotations

import argparse
from pathlib import Path
import subprocess
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    root = args.root.resolve()
    bootstrap = root / ".sc-lint" / "bootstrap.py"
    if not bootstrap.is_file():
        print(f"sc-lint bootstrap is missing: {bootstrap}", file=sys.stderr)
        return 1
    return subprocess.run(
        [sys.executable, str(bootstrap), "setup", "--config", "sc-lint.toml"],
        cwd=root,
        check=False,
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
