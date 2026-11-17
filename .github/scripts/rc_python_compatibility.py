#!/usr/bin/env python3
"""Manifest-declared, once-per-candidate Python compatibility hooks and evidence."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib


class CompatibilityError(ValueError):
    pass


def configuration(manifest):
    if "rc_python_compatibility" not in manifest:
        return None
    config = manifest["rc_python_compatibility"]
    required = {"versions", "build_python", "build", "smoke"}
    optional = {"setup", "rust_toolchain", "exclusions", "runner_overrides"}
    if not isinstance(config, dict) or not required <= set(config) or set(config) - required - optional:
        raise CompatibilityError("rc_python_compatibility requires versions, build_python, build and smoke; unknown fields refused")
    versions = config["versions"]
    if (not isinstance(versions, list) or not versions or
            any(not isinstance(v, str) or not re.fullmatch(r"[0-9]+\.[0-9]+", v) for v in versions) or
            len(versions) != len(set(versions))):
        raise CompatibilityError("compatibility versions must be unique Python X.Y strings")
    if not isinstance(config["build_python"], str) or not re.fullmatch(r"[0-9]+\.[0-9]+", config["build_python"]):
        raise CompatibilityError("build_python must be a Python X.Y string")
    for name in ("setup", "build", "smoke"):
        if name in config and (not isinstance(config[name], list) or not config[name] or
                               any(not isinstance(arg, str) or not arg or "\0" in arg for arg in config[name])):
            raise CompatibilityError(f"compatibility {name} must be a nonempty argv array")
    if "rust_toolchain" in config and (not isinstance(config["rust_toolchain"], str) or
            not re.fullmatch(r"[A-Za-z0-9_.-]+", config["rust_toolchain"])):
        raise CompatibilityError("invalid compatibility Rust toolchain")
    platforms = set()
    for distribution in manifest.get("python_distributions", []):
        for wheel in distribution.get("wheels", []):
            runner = wheel if isinstance(wheel, str) else wheel.get("os") if isinstance(wheel, dict) else None
            if not isinstance(runner, str) or not re.fullmatch(r"[A-Za-z0-9_.-]+", runner):
                raise CompatibilityError("compatibility wheel platforms must declare runner labels")
            platforms.add(runner)
    if not platforms:
        raise CompatibilityError("RC compatibility requires declared Python wheel platforms")
    for name in ("exclusions", "runner_overrides"):
        values = config.get(name, {})
        if not isinstance(values, dict) or set(values) - platforms:
            raise CompatibilityError(f"compatibility {name} must name only declared wheel platforms")
    for runner, excluded in config.get("exclusions", {}).items():
        if (not isinstance(excluded, list) or any(not isinstance(v, str) or v not in versions for v in excluded) or
                len(excluded) != len(set(excluded)) or not set(versions) - set(excluded)):
            raise CompatibilityError(f"invalid interpreter exclusions for {runner}")
    for runner in config.get("runner_overrides", {}).values():
        if not isinstance(runner, str) or not re.fullmatch(r"[A-Za-z0-9_.-]+", runner):
            raise CompatibilityError("invalid compatibility runner override")
    return config


def matrix(manifest):
    config = configuration(manifest)
    if config is None:
        return []
    platforms = {wheel if isinstance(wheel, str) else wheel["os"]
                 for distribution in manifest["python_distributions"] for wheel in distribution["wheels"]}
    return [{"os": platform, "runner": config.get("runner_overrides", {}).get(platform, platform),
             "versions": json.dumps([v for v in config["versions"] if v not in config.get("exclusions", {}).get(platform, [])])}
            for platform in sorted(platforms)]


def workflow_job(manifest):
    config = configuration(manifest)
    if config is None:
        return ""
    rows = json.dumps({"include": matrix(manifest)}, separators=(",", ":"))
    lines = ["", "  python-compat:", "    needs: establish-provenance",
             "    name: python-compat (${{ matrix.os }}, ${{ needs.establish-provenance.outputs.release_candidate_sha }})",
             "    runs-on: ${{ matrix.runner }}", "    strategy:", "      fail-fast: false",
             f"      matrix: {rows}", "    steps:", "      - uses: actions/checkout@v5", "        with:",
             "          ref: ${{ needs.establish-provenance.outputs.release_candidate_sha }}",
             "      - uses: actions/setup-python@v6", "        with:",
             "          python-version: " + json.dumps(config["build_python"])]
    if "rust_toolchain" in config:
        lines += ["      - uses: dtolnay/rust-toolchain@master", "        with:",
                  "          toolchain: " + json.dumps(config["rust_toolchain"])]
    if "setup" in config:
        lines += ["      - name: Set up declared compatibility build tools",
                  "        run: python .github/scripts/rc_python_compatibility.py setup"]
    lines += ["      - name: Build candidate wheels once",
              "        run: python .github/scripts/rc_python_compatibility.py build"]
    for version in config["versions"]:
        condition = "${{ contains(fromJSON(matrix.versions), '" + version + "') }}"
        lines += ["      - name: Set up Python " + version, "        if: " + condition,
                  "        uses: actions/setup-python@v6", "        with:",
                  "          python-version: " + json.dumps(version),
                  "      - name: Smoke-test wheels on Python " + version, "        if: " + condition,
                  "        run: python .github/scripts/rc_python_compatibility.py smoke"]
    return "\n".join(lines) + "\n"


GATE_STEP = '''      - name: Require declared release-candidate Python compatibility
        env:
          GH_TOKEN: ${{ github.token }}
          CANDIDATE_TAG: release-candidate-${{ steps.meta.outputs.release_tag }}
          RELEASE_SHA: ${{ steps.release_gate.outputs.release_sha }}
        shell: bash
        run: |
          set -euo pipefail
          python3 .github/scripts/rc_python_compatibility.py check --manifest "$RELEASE_ARTIFACT_MANIFEST" --source-ref "$RELEASE_SHA" --candidate "$CANDIDATE_TAG"

'''


def extend_workflow(text, relative, manifest):
    if configuration(manifest) is None:
        return text
    if relative == ".github/workflows/release-candidate.yml":
        return text + workflow_job(manifest)
    if relative == ".github/workflows/release.yml":
        marker = "      - name: Verify npm source at resolved release commit\n"
        job_marker = "  gate-and-tag:\n    runs-on:"
        if text.count(marker) != 1 or text.count(job_marker) != 1:
            raise CompatibilityError("cannot locate the release compatibility extension point")
        return text.replace(job_marker, "  gate-and-tag:\n    permissions:\n      contents: write\n      actions: read\n    runs-on:", 1).replace(marker, GATE_STEP + marker, 1)
    return text


def pages(endpoint):
    result = subprocess.run(["gh", "api", "--paginate", "--slurp", endpoint],
                            check=True, text=True, stdout=subprocess.PIPE, timeout=900)
    rows = json.loads(result.stdout)
    if not isinstance(rows, list) or any(not isinstance(row, dict) for row in rows):
        raise CompatibilityError("invalid compatibility evidence response")
    return rows


def check(manifest, candidate, repository, *, query=pages):
    if configuration(manifest) is None:
        return True
    if not re.fullmatch(r"[0-9a-f]{40}", candidate):
        raise CompatibilityError("compatibility evidence requires an exact candidate commit")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise CompatibilityError("compatibility evidence requires OWNER/REPO")
    expected = {f"python-compat ({row['os']}, {candidate})" for row in matrix(manifest)}
    endpoint = f"repos/{repository}/actions/workflows/release-candidate.yml/runs?event=workflow_dispatch&status=success&per_page=100"
    for page in query(endpoint):
        for run in page["workflow_runs"]:
            if run["conclusion"] != "success":
                continue
            successful = {job["name"] for page in query(f"repos/{repository}/actions/runs/{run['id']}/jobs?per_page=100")
                          for job in page["jobs"] if job["conclusion"] == "success"}
            if expected <= successful:
                print(f"Python compatibility passed for {candidate}: run {run['id']}")
                return True
    return False


def execute_hook(manifest, command):
    config = configuration(manifest)
    if config is None:
        raise CompatibilityError("RC compatibility job requires its candidate's manifest declaration")
    if command not in config:
        raise CompatibilityError(f"compatibility {command} hook was not declared")
    subprocess.run(config[command], check=True, timeout=900)


def committed_manifest(path, ref):
    if Path(path).is_absolute() or ":" in path or "\\" in path or ".." in Path(path).parts:
        raise CompatibilityError("--source-ref requires a repository-relative manifest path")
    def git(*args):
        return subprocess.run(["git", *args], check=True, text=True, stdout=subprocess.PIPE, timeout=900).stdout.strip()
    commit = git("rev-parse", "--verify", ref + "^{commit}")
    if not git("ls-tree", "--name-only", commit, "--", path):
        return {}
    return tomllib.loads(git("show", f"{commit}:{path}"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["setup", "build", "smoke", "check"])
    parser.add_argument("--manifest", default="release/publish-artifacts.toml")
    parser.add_argument("--source-ref")
    parser.add_argument("--candidate")
    args = parser.parse_args()
    try:
        manifest = committed_manifest(args.manifest, args.source_ref) if args.source_ref else tomllib.loads(Path(args.manifest).read_text())
        if args.command != "check":
            execute_hook(manifest, args.command)
            return 0
        if configuration(manifest) is None:
            return 0
        candidate = subprocess.run(["git", "rev-parse", "--verify", args.candidate + "^{commit}"],
                                   check=True, text=True, stdout=subprocess.PIPE, timeout=900).stdout.strip()
        if check(manifest, candidate, os.environ.get("GITHUB_REPOSITORY", "")):
            return 0
        raise CompatibilityError(f"no successful release-candidate Python compatibility run for {candidate}")
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Cannot verify RC Python compatibility: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
