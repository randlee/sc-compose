"""Explicit Python wheel build targets and artifact platform validation."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import tomllib

WINDOWS_PLATFORMS = {"x86_64-pc-windows-msvc": "win_amd64", "aarch64-pc-windows-msvc": "win_arm64"}


def wheel_targets(distribution, *, continuity=False):
    wheels = distribution["wheels"]
    if not isinstance(wheels, list):
        raise ValueError("wheels must be an array")
    result = []
    for wheel in wheels:
        item = {"target": "", "platform": "", "manylinux": "off", "deployment_target": ""}
        if isinstance(wheel, str):
            if not re.fullmatch(r"(?:ubuntu|macos|windows)-[A-Za-z0-9.-]+", wheel):
                raise ValueError("legacy wheels must name GitHub runner labels, not wheel tags")
            item.update(id=wheel, os=wheel)
        elif isinstance(wheel, dict):
            for field in ("id", "os", "target", "platform"):
                if not isinstance(wheel.get(field), str) or not re.fullmatch(r"[A-Za-z0-9_.-]+", wheel[field]):
                    raise ValueError(f"wheel {field} must be a nonempty safe identifier")
            item.update(wheel)
            if not re.fullmatch(r"(?:ubuntu|macos|windows)-[A-Za-z0-9.-]+", item["os"]):
                raise ValueError("wheel os must name a GitHub runner")
            if item["target"].endswith("linux-gnu"):
                manylinux = item.get("manylinux")
                if continuity:
                    if not isinstance(manylinux, str) or not re.fullmatch(r"2_[0-9]+", manylinux):
                        raise ValueError("explicit Linux wheels require a manylinux=2_N policy")
                    arch = item["target"].split("-", 1)[0]
                    base = f"manylinux_{manylinux}_{arch}"
                    allowed = {base, base + f".manylinux2014_{arch}"} if manylinux == "2_17" else {base}
                    if not item["os"].startswith("ubuntu-") or item["platform"] not in allowed:
                        raise ValueError("Linux wheel runner/target/platform mismatch")
                else:
                    if manylinux != "2_28":
                        raise ValueError("explicit Linux wheels require manylinux=2_28")
                    arch = item["target"].split("-", 1)[0]
                    if not item["os"].startswith("ubuntu-") or item["platform"] != f"manylinux_2_28_{arch}":
                        raise ValueError("Linux wheel runner/target/platform mismatch")
            elif item["target"].endswith("apple-darwin"):
                deployment = item.get("deployment_target")
                arch = "arm64" if item["target"].startswith("aarch64-") else "x86_64"
                if not isinstance(deployment, str) or not re.fullmatch(r"[0-9]+\.[0-9]+", deployment):
                    raise ValueError("macOS wheels require deployment_target")
                if not item["os"].startswith("macos-") or item["platform"] != f"macosx_{deployment.replace('.', '_')}_{arch}":
                    raise ValueError("macOS wheel runner/target/platform mismatch")
            elif item["target"] in WINDOWS_PLATFORMS:
                if not item["os"].startswith("windows-") or item["platform"] != WINDOWS_PLATFORMS[item["target"]]:
                    raise ValueError("Windows wheel runner/target/platform mismatch")
            else:
                raise ValueError("unsupported explicit wheel target")
            if distribution.get("build_system") == "setuptools":
                raise ValueError("explicit platform wheels currently require maturin")
        else:
            raise ValueError("wheels entries must be runner strings or target objects")
        result.append(item)
    if len({item["id"] for item in result}) != len(result):
        raise ValueError("wheel target ids must be unique")
    platforms = [item["platform"] for item in result if item["platform"]]
    if len(platforms) != len(set(platforms)):
        raise ValueError("wheel platforms must be unique")
    return result


def matrix_entry(distribution):
    return {key: distribution[key] or "" for key in ("artifact", "name", "source", "pyproject", "cargo_manifest", "build_system")}


def cmd_python_wheel_matrix(args):
    from release_manifest import load_manifest, _python_distribution_entries
    manifest = load_manifest(Path(args.manifest))
    include = [{**matrix_entry(distribution), **(wheel if wheel["target"] else {"os": wheel["os"]})} for distribution in _python_distribution_entries(manifest) for wheel in wheel_targets(distribution, continuity="publication_continuity" in manifest)]
    print(json.dumps({"include": include}, separators=(",", ":")))
    return 0


def explicit_asset_patterns(manifest):
    for distribution in manifest.get("python_distributions", []):
        targets = [item for item in wheel_targets(distribution, continuity="publication_continuity" in manifest) if item["platform"]]
        if not targets:
            continue
        name = re.escape(distribution["name"].replace("-", "_"))
        for item in targets:
            yield "^" + name + r"-.*-" + re.escape(item["platform"]) + r"\.whl$"
        if distribution["sdist"]:
            yield "^" + re.escape(distribution["name"]).replace(r"\-", "[-_]") + r"-.*\.tar\.gz$"


def verify_platforms(distribution, paths, *, continuity=False):
    expected = {item["platform"] for item in wheel_targets(distribution, continuity=continuity) if item["platform"]}
    if not expected:
        return
    found = set()
    for path in paths:
        tags = {path.stem.rsplit("-", 1)[-1]} if continuity else set(path.stem.rsplit("-", 1)[-1].split("."))
        matched = expected & tags
        if len(matched) != 1 or found & matched:
            raise ValueError("Python wheel platforms do not match declared unique targets")
        found |= matched
    if found != expected:
        raise ValueError("Python wheel platform missing from release assets")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--asset-dir", type=Path, required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("--manifest", help="manifest selecting declaration-only continuity policy")
    args = parser.parse_args()
    continuity = bool(args.manifest and "publication_continuity" in tomllib.loads(Path(args.manifest).read_text()))
    paths = list(args.asset_dir.glob("*.whl"))
    tags = ({paths[0].stem.rsplit("-", 1)[-1]} if continuity else set(paths[0].stem.rsplit("-", 1)[-1].split("."))) if len(paths) == 1 else set()
    if len(paths) != 1 or args.platform not in tags:
        raise SystemExit("built wheel does not match declared platform")


if __name__ == "__main__":
    main()
