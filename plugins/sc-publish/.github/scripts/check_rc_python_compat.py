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
    data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    platforms = {os_name for package in data["python_distributions"] for os_name in package["wheels"]}
    expected = {f"python-compat ({os_name}, {candidate})" for os_name in platforms}
    if not expected:
        raise ValueError("release manifest has no Python wheel platforms")
    endpoint = f"repos/{repository}/actions/workflows/release-candidate.yml/runs?event=workflow_dispatch&status=success&per_page=100"
    for page in pages(endpoint):
        for run in page["workflow_runs"]:
            if run["conclusion"] != "success":
                continue
            jobs = [job for p in pages(f"repos/{repository}/actions/runs/{run['id']}/jobs?per_page=100") for job in p["jobs"]]
            successful = {job["name"] for job in jobs if job["conclusion"] == "success"}
            if expected <= successful:
                print(f"python compatibility passed for {candidate}: run {run['id']}")
                return True
    return False


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
