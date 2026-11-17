#!/usr/bin/env python3
"""Manifest-declared, read-only publication continuity admission.

The comparison accepts an injected previous-release inventory. Network and Git
access live in publication_inventory; unit tests never need either service.
"""
from __future__ import annotations

import argparse
import html
import json
import os
from pathlib import Path, PureWindowsPath
import re
import subprocess
import sys
import tomllib

from release_python import wheel_targets

CHANNELS = {"pypi", "npm", "homebrew", "scoop", "winget"}
KINDS = {"asset", "crate", "pypi", "npm", "go", "homebrew", "scoop", "winget"}


class ContinuityError(ValueError):
    """Publication inventory or manifest does not establish continuity."""


def string(value, label):
    if not isinstance(value, str) or not value.strip() or any(ord(c) < 32 for c in value):
        raise ContinuityError(f"{label} must be a nonempty string without controls")
    return value


def configuration(manifest):
    """Absence is opt-out; declaration is always blocking, with no bypass flag."""
    if "publication_continuity" not in manifest:
        return None
    config = manifest["publication_continuity"]
    if not isinstance(config, dict) or set(config) - {"deprecations"}:
        raise ContinuityError("publication_continuity must be a table containing only deprecations")
    rows = config.get("deprecations", [])
    if not isinstance(rows, list):
        raise ContinuityError("publication_continuity.deprecations must be an array")
    approved = {}
    required = {"identifier", "reason", "approved_by", "approval_ref"}
    for row in rows:
        if not isinstance(row, dict) or set(row) != required:
            raise ContinuityError("deprecation requires exactly identifier, reason, approved_by and approval_ref")
        for field in required:
            string(row[field], f"deprecation.{field}")
        identifier = row["identifier"]
        kind, separator, value = identifier.partition(":")
        if not separator or kind not in KINDS or not value.strip():
            raise ContinuityError(f"invalid publication identifier: {identifier!r}")
        if not re.fullmatch(r"https://[^\s]+/(?:issues|pull)/[1-9][0-9]*(?:#[^\s]+)?", row["approval_ref"]):
            raise ContinuityError("approval_ref must link to the issue or PR containing user approval")
        if identifier in approved:
            raise ContinuityError(f"duplicate deprecation: {identifier}")
        approved[identifier] = row
    return approved


def normalize_asset(name, version):
    string(name, "release asset name")
    version = version.removeprefix("v")
    return re.sub(rf"(?<![A-Za-z0-9])v?{re.escape(version)}(?![A-Za-z0-9])", "{version}", name)


def pypi_name(name):
    return re.sub(r"[-_.]+", "-", name).lower()


def planned_identifiers(manifest):
    """Registry/channel identities come only from this release's declarations."""
    result = {f"crate:{row['package']}" for row in manifest.get("crates", []) if row.get("publish") is True}
    channels = manifest.get("channels", {})
    unknown = set(channels) - CHANNELS
    if unknown:
        raise ContinuityError(f"no continuity provider for channel(s): {', '.join(sorted(unknown))}")
    if "pypi" in channels:
        result.update(f"pypi:{pypi_name(row['name'])}" for row in manifest.get("python_distributions", []))
    if "npm" in channels:
        result.update(f"npm:{row['name']}" for row in manifest.get("npm_packages", []))
    for row in manifest.get("go_modules", []):
        result.add(f"go:{row['module']}")
    if "homebrew" in channels:
        channel = channels["homebrew"]
        result.update(f"homebrew:{channel['tap_repository']}/{row['path']}" for row in channel["formulas"])
    if "scoop" in channels:
        channel = channels["scoop"]
        result.add(f"scoop:{channel['bucket_repository']}/{channel['manifest_path']}")
    if "winget" in channels:
        result.add(f"winget:{channels['winget']['identifier']}")
    return result


def planned_asset_patterns(manifest):
    """Use the declared artifact family/target; never infer it from current files."""
    patterns = [r"^checksums\.txt$"]
    if manifest.get("release_binaries"):
        prefix = re.escape(manifest["project"]["archive_prefix"])
        patterns += [rf"^{prefix}_.*_{re.escape(row['target'])}\.{re.escape(row['archive'])}$"
                     for row in manifest.get("release_targets", [])]
    for row in manifest.get("python_distributions", []):
        name = re.escape(row["name"]).replace(r"\-", "[-_]")
        if row.get("sdist"):
            patterns.append(rf"^{name}-[^/]+\.tar\.gz$")
        for wheel in wheel_targets(row, continuity=True):
            platform = wheel_platform_pattern(manifest, row, wheel)
            patterns.append(rf"^{name}-[^/]+-{platform}\.whl$")
    for row in manifest.get("npm_packages", []):
        name = re.escape(row["name"].replace("@", "").replace("/", "-"))
        patterns.append(rf"^{name}-[0-9][^/]*\.tgz$")
    for row in manifest.get("go_modules", []):
        for target in row["targets"]:
            triple = target["target"] if isinstance(target, dict) else target
            patterns.append(rf"^{re.escape(row['artifact_prefix'])}-{re.escape(triple)}\.zip$")
    return [re.compile(pattern) for pattern in patterns]


def wheel_platform_pattern(manifest, distribution, wheel):
    if wheel["platform"]:
        return re.escape(wheel["platform"])
    if distribution.get("build_system") == "setuptools":
        return "any"
    raise ContinuityError("continuity requires explicit platform wheel declarations")


def check(manifest, previous):
    """Compare an actual previous inventory with the new manifest, fail closed."""
    approved = configuration(manifest)
    if approved is None:
        return {"status": "disabled"}
    if (not isinstance(previous, dict) or not {"tag", "assets", "publications"} <= set(previous)
            or set(previous) - {"tag", "assets", "publications", "notes"}):
        raise ContinuityError("previous inventory requires tag, assets and publications, with optional notes")
    notes = previous.get("notes", [])
    if not isinstance(notes, list) or any(not isinstance(note, str) for note in notes):
        raise ContinuityError("previous inventory notes must be an array of strings")
    tag = previous["tag"]
    if tag is not None and not re.fullmatch(r"v?\d+\.\d+\.\d+", string(tag, "previous tag")):
        raise ContinuityError("invalid previous release tag")
    if not isinstance(previous["assets"], list) or not isinstance(previous["publications"], list):
        raise ContinuityError("previous assets and publications must be arrays")
    if tag is None and previous["assets"]:
        raise ContinuityError("asset inventory without a previous release")
    published = set()
    missing = set()
    patterns = planned_asset_patterns(manifest)
    for asset in previous["assets"]:
        identifier = "asset:" + normalize_asset(asset, tag)
        if identifier in published:
            raise ContinuityError(f"duplicate asset family: {identifier}")
        published.add(identifier)
        if not any(pattern.fullmatch(asset) for pattern in patterns):
            missing.add(identifier)
    planned = planned_identifiers(manifest)
    for identifier in previous["publications"]:
        string(identifier, "previous publication")
        if identifier.partition(":")[0] not in KINDS - {"asset"}:
            raise ContinuityError(f"unknown previous publication kind: {identifier}")
        if identifier in published:
            raise ContinuityError(f"duplicate previous publication: {identifier}")
        published.add(identifier)
        if identifier not in planned:
            missing.add(identifier)
    stale = set(approved) - published - planned
    retained = (set(approved) - missing) - stale
    if retained:
        raise ContinuityError(f"deprecation does not name a dropped item: {', '.join(sorted(retained))}")
    denied = missing - set(approved)
    if denied:
        raise ContinuityError(f"publication continuity would drop: {', '.join(sorted(denied))}")
    return {"status": "pass", "previous_release": tag, "published": sorted(published),
            "deprecated": sorted(missing), "notes": [*notes, *[
                f"Stale deprecation (absent from previous release and current plan): {identifier}"
                for identifier in sorted(stale)]]}


def write_summary(result, approvals, path):
    """Label approved drops in the existing Actions summary; create no artifact."""
    if not path or result["status"] == "disabled":
        return
    escape = html.escape
    lines = ["\n### Publication continuity", f"Status: {result['status']}"]
    if result.get("deprecated"):
        lines.append("\nUser-approved deprecated publications:")
        for identifier in result["deprecated"]:
            row = approvals[identifier]
            lines.append(f"- <code>{escape(identifier)}</code>: {escape(row['reason'])}; "
                         f"approved by {escape(row['approved_by'])}; "
                         f"<a href=\"{escape(row['approval_ref'], quote=True)}\">approval reference</a>")
    for note in result.get("notes", []):
        lines.append(f"- Note: {escape(note)}")
    with Path(path).open("a", encoding="utf-8") as output:
        output.write("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--source-ref", help="resolved release commit; read the planned manifest at this ref")
    args = parser.parse_args()
    try:
        from publication_inventory import Inventory
        inventory = Inventory(args.repository)
        if args.source_ref:
            if Path(args.manifest).is_absolute() or PureWindowsPath(args.manifest).is_absolute():
                raise ContinuityError("--source-ref requires a repository-relative manifest path")
            commit = subprocess.run(["git", "rev-parse", "--verify", args.source_ref + "^{commit}"],
                                    capture_output=True, text=True, timeout=30, check=True).stdout.strip()
            entry = subprocess.run(["git", "ls-tree", "--name-only", commit, "--", args.manifest],
                                   capture_output=True, text=True, timeout=30, check=True).stdout.strip()
            if not entry:
                print(json.dumps({"status": "disabled"}))
                return 0
            source = subprocess.run(["git", "show", f"{commit}:{args.manifest}"],
                                    capture_output=True, text=True, timeout=30, check=True).stdout
            manifest = tomllib.loads(source)
        else:
            manifest = tomllib.loads(Path(args.manifest).read_text(encoding="utf-8"))
        if configuration(manifest) is None:
            print(json.dumps({"status": "disabled"}))
            return 0
        previous = inventory.previous(args.version, args.manifest)
        result = check(manifest, previous)
        write_summary(result, configuration(manifest), os.environ.get("GITHUB_STEP_SUMMARY"))
        print(json.dumps(result, sort_keys=True))
        return 0
    except (ContinuityError, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Publication continuity failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
