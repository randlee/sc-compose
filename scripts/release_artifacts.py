#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path


def load_manifest(path: Path) -> dict:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    crates = data.get("crates", [])
    if not crates:
        raise SystemExit("manifest must define [[crates]]")
    release_binaries = data.get("release_binaries", [])
    python_packages = data.get("python_packages", [])
    python_distributions = data.get("python_distributions", [])
    crates = sorted(crates, key=lambda item: (item["publish_order"], item["artifact"]))
    return {
        "crates": crates,
        "release_binaries": release_binaries,
        "python_packages": python_packages,
        "python_distributions": python_distributions,
    }


def _require_keys(entry: dict, required: tuple[str, ...], label: str) -> None:
    missing = [key for key in required if key not in entry]
    if missing:
        joined = ", ".join(missing)
        raise SystemExit(f"{label} missing required keys: {joined}")


def workspace_members(workspace_toml: Path) -> set[str]:
    data = tomllib.loads(workspace_toml.read_text(encoding="utf-8"))
    return set(data.get("workspace", {}).get("members", []))


def package_name(cargo_toml: Path) -> str:
    data = tomllib.loads(cargo_toml.read_text(encoding="utf-8"))
    return data["package"]["name"]


def workspace_version(workspace_toml: Path) -> str:
    data = tomllib.loads(workspace_toml.read_text(encoding="utf-8"))
    return data["workspace"]["package"]["version"]


def _python_project_version(pyproject_toml: Path) -> str:
    data = tomllib.loads(pyproject_toml.read_text(encoding="utf-8"))
    project = data.get("project", {})
    version = project.get("version")
    if not isinstance(version, str):
        raise SystemExit(f"{pyproject_toml}: [project].version must be a string")
    return version


def cmd_validate_manifest(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    members = workspace_members(Path(args.workspace_toml))
    missing = []
    for crate in manifest["crates"]:
        if crate["cargo_toml"].removesuffix("/Cargo.toml") not in members:
            missing.append(crate["cargo_toml"])
    if missing:
        raise SystemExit(f"manifest references non-member crates: {', '.join(missing)}")
    seen = set()
    for crate in manifest["crates"]:
        artifact = crate["artifact"]
        if artifact in seen:
            raise SystemExit(f"duplicate artifact: {artifact}")
        seen.add(artifact)
        actual = package_name(Path(crate["cargo_toml"]))
        if actual != crate["package"]:
            raise SystemExit(f"{crate['cargo_toml']}: package mismatch: manifest={crate['package']} actual={actual}")
    python_artifacts = set()
    python_packages_by_name: dict[str, dict] = {}
    for index, package in enumerate(manifest["python_packages"], start=1):
        _require_keys(package, ("artifact", "package", "manifest", "module", "publish"), f"[[python_packages]] #{index}")
        artifact = package["artifact"]
        if artifact in seen or artifact in python_artifacts:
            raise SystemExit(f"duplicate artifact: {artifact}")
        python_artifacts.add(artifact)
        manifest_path = Path(package["manifest"])
        if manifest_path.is_file():
            python_package_version = _python_project_version(manifest_path)
            if not python_package_version:
                raise SystemExit(f"{manifest_path}: missing [project].version")
        python_packages_by_name[package["package"]] = package
    for index, distribution in enumerate(manifest["python_distributions"], start=1):
        _require_keys(distribution, ("name", "source", "sdist", "wheels"), f"[[python_distributions]] #{index}")
        if distribution["name"] not in python_packages_by_name:
            raise SystemExit(
                f"[[python_distributions]] #{index}: no matching [[python_packages]] entry for {distribution['name']}"
            )
        source = Path(distribution["source"])
        if not isinstance(distribution["sdist"], bool):
            raise SystemExit(f"[[python_distributions]] #{index}: sdist must be a boolean")
        wheels = distribution["wheels"]
        if not isinstance(wheels, list) or not all(isinstance(entry, str) for entry in wheels):
            raise SystemExit(f"[[python_distributions]] #{index}: wheels must be a list of strings")
    print("manifest validation passed")
    return 0


def cmd_list_publish_plan(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    for crate in manifest["crates"]:
        print(f"{crate['package']}|{crate['wait_after_publish_seconds']}")
    return 0


def cmd_verify_version(args: argparse.Namespace) -> int:
    version = workspace_version(Path(args.workspace_toml))
    if version != args.version:
        raise SystemExit(f"workspace version mismatch: expected {args.version}, got {version}")
    manifest = load_manifest(Path(args.manifest))
    for crate in manifest["crates"]:
        data = tomllib.loads(Path(crate["cargo_toml"]).read_text(encoding='utf-8'))
        pkg_version = data["package"]["version"]
        if isinstance(pkg_version, str):
            actual = pkg_version
        elif isinstance(pkg_version, dict) and pkg_version.get("workspace") is True:
            actual = version
        else:
            raise SystemExit(f"{crate['package']}: unsupported version shape: {pkg_version!r}")
        if actual != version:
            raise SystemExit(f"{crate['package']}: version mismatch: expected {version}, got {actual}")
    print("version verification passed")
    return 0


def cmd_verify_python_version(args: argparse.Namespace) -> int:
    version = workspace_version(Path(args.workspace_toml))
    if version != args.version:
        raise SystemExit(f"workspace version mismatch: expected {args.version}, got {version}")
    actual = _python_project_version(Path(args.pyproject))
    if actual != version:
        raise SystemExit(f"python package version mismatch: expected {version}, got {actual}")
    print("python version verification passed")
    return 0


def cmd_sync_python_version(args: argparse.Namespace) -> int:
    version = workspace_version(Path(args.workspace_toml))
    pyproject = Path(args.pyproject)
    lines = pyproject.read_text(encoding="utf-8").splitlines()
    output: list[str] = []
    in_project = False
    updated = False

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_project = stripped == "[project]"
        if in_project and re.match(r'^\s*version\s*=\s*"[^"]+"\s*$', line):
            output.append(re.sub(r'"[^"]+"', f'"{version}"', line, count=1))
            updated = True
            continue
        output.append(line)

    if not updated:
        raise SystemExit(f"{pyproject}: could not find [project].version to rewrite")

    pyproject.write_text("\n".join(output) + "\n", encoding="utf-8")
    print(f"synced python package version to {version}")
    return 0


def cmd_cargo_build_bin_args(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    print(" ".join(f"--bin {entry['name']}" for entry in manifest["release_binaries"]))
    return 0


def cargo_search_version_exists(crate: str, version: str) -> bool:
    result = subprocess.run(
        ["cargo", "search", crate, "--limit", "1"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    return f'{crate} = "{version}"' in result.stdout


def cmd_check_version_unpublished(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    published = []
    for crate in manifest["crates"]:
        if cargo_search_version_exists(crate["package"], args.version):
            published.append(crate["artifact"])
    if published:
        raise SystemExit("release version already published for: " + ", ".join(sorted(published)))
    print(f"ok: no publishable artifacts found at version {args.version}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("validate-manifest")
    p.add_argument("--manifest", required=True)
    p.add_argument("--workspace-toml", required=True)
    p.set_defaults(func=cmd_validate_manifest)

    p = sub.add_parser("list-publish-plan")
    p.add_argument("--manifest", required=True)
    p.set_defaults(func=cmd_list_publish_plan)

    p = sub.add_parser("verify-version")
    p.add_argument("--manifest", required=True)
    p.add_argument("--workspace-toml", required=True)
    p.add_argument("--version", required=True)
    p.set_defaults(func=cmd_verify_version)

    p = sub.add_parser("verify-python-version")
    p.add_argument("--workspace-toml", required=True)
    p.add_argument("--pyproject", required=True)
    p.add_argument("--version", required=True)
    p.set_defaults(func=cmd_verify_python_version)

    p = sub.add_parser("sync-python-version")
    p.add_argument("--workspace-toml", required=True)
    p.add_argument("--pyproject", required=True)
    p.set_defaults(func=cmd_sync_python_version)

    p = sub.add_parser("cargo-build-bin-args")
    p.add_argument("--manifest", required=True)
    p.set_defaults(func=cmd_cargo_build_bin_args)

    p = sub.add_parser("check-version-unpublished")
    p.add_argument("--manifest", required=True)
    p.add_argument("--version", required=True)
    p.set_defaults(func=cmd_check_version_unpublished)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
