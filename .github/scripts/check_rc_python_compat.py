#!/usr/bin/env python3
"""Require successful RC compatibility jobs for the release-candidate commit."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib


def pages(endpoint: str) -> list[dict]:
    result = subprocess.run(
        ["gh", "api", "--paginate", "--slurp", endpoint],
        check=True, capture_output=True, text=True,
    )
    return json.loads(result.stdout)


def check(candidate: str, manifest: Path, repository: str) -> bool:
    from rc_python_compatibility import check as check_declared, configuration
    data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    if not any(row.get("wheels") for row in data.get("python_distributions", [])):
        raise ValueError("release manifest has no Python wheel platforms")
    if configuration(data) is None:
        raise ValueError("release manifest has no RC Python compatibility declaration")
    return check_declared(data, candidate, repository, query=pages)


def main() -> int:
    try:
        candidate, manifest = sys.argv[1:]
        if check(candidate, Path(manifest), os.environ["GITHUB_REPOSITORY"]):
            return 0
        print(f"no successful release-candidate Python compatibility run for {candidate}", file=sys.stderr)
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"cannot verify release-candidate Python compatibility: {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
