#!/usr/bin/env python3
"""Target-aware release helpers for Go modules with native static archives."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import tempfile
import tomllib
from pathlib import Path
from typing import Any


class ContractError(ValueError):
    """Raised when a consumer does not meet the peer-package contract."""


GO_MODULE_RE = re.compile(r"^module\s+(\S+)\s*$", re.MULTILINE)
SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")
CONFIG_FIELDS = {"schema_version", "source", "cargo_package", "artifact_prefix"}


def _mapping(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ContractError(f"{label} must be a table")
    return value


def _string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ContractError(f"{label} must be a non-empty string")
    return value


def _safe_relative(value: str, label: str) -> Path:
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ContractError(f"{label} must be a repository-relative safe path")
    return path


def _read_toml(path: Path, label: str) -> dict[str, Any]:
    try:
        with path.open("rb") as source:
            return _mapping(tomllib.load(source), label)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ContractError(f"cannot read {label} {path}: {error}") from error


def _inside(root: Path, path: Path, label: str) -> Path:
    resolved_root = root.resolve()
    resolved_path = path.resolve()
    try:
        resolved_path.relative_to(resolved_root)
    except ValueError as error:
        raise ContractError(f"{label} escapes the consumer repository") from error
    return resolved_path


def _module_name(go_mod: Path) -> str:
    try:
        matches = GO_MODULE_RE.findall(go_mod.read_text(encoding="utf-8"))
    except OSError as error:
        raise ContractError(f"missing go.mod: {go_mod}") from error
    if len(matches) != 1:
        raise ContractError(f"{go_mod}: must contain exactly one module declaration")
    return matches[0]


def _target_entries(source: Path) -> tuple[dict[str, str], list[dict[str, str]]]:
    data = _read_toml(source / "native" / "targets.toml", "native target contract")
    if data.get("schema_version") != 1:
        raise ContractError("native target contract schema_version must be 1")
    contract = _mapping(data.get("contract"), "native target contract [contract]")
    generated_package = _string(contract.get("generated_package"), "contract.generated_package")
    native_library = _string(contract.get("native_library"), "contract.native_library")
    if Path(generated_package).is_absolute() or ".." in Path(generated_package).parts:
        raise ContractError("contract.generated_package must be a safe relative path")
    if Path(native_library).name != native_library:
        raise ContractError("contract.native_library must be a filename")

    raw_targets = data.get("targets")
    if not isinstance(raw_targets, list) or not raw_targets:
        raise ContractError("native target contract must define non-empty [[targets]]")
    seen_rust: set[str] = set()
    seen_go: set[tuple[str, str]] = set()
    targets: list[dict[str, str]] = []
    for index, raw in enumerate(raw_targets, start=1):
        target = _mapping(raw, f"native target #{index}")
        entry = {
            "target": _string(target.get("rust_target"), f"native target #{index}.rust_target"),
            "goos": _string(target.get("goos"), f"native target #{index}.goos"),
            "goarch": _string(target.get("goarch"), f"native target #{index}.goarch"),
            "library": _string(target.get("library"), f"native target #{index}.library"),
        }
        if Path(entry["library"]).name != entry["library"]:
            raise ContractError(f"native target #{index}.library must be a filename")
        if entry["target"] in seen_rust or (entry["goos"], entry["goarch"]) in seen_go:
            raise ContractError(f"duplicate native target: {entry['target']}")
        seen_rust.add(entry["target"])
        seen_go.add((entry["goos"], entry["goarch"]))
        targets.append(entry)
    return {"generated_package": generated_package, "native_library": native_library}, targets


def validate_binding_source(
    consumer_root: Path, source_value: str, cargo_package: str, artifact_prefix: str
) -> dict[str, Any]:
    """Validate consumer-owned facts and derive binding-owned identities."""
    source_relative = _safe_relative(source_value, "source")
    cargo_package = _string(cargo_package, "cargo_package")
    artifact_prefix = _string(artifact_prefix, "artifact_prefix")
    if Path(artifact_prefix).name != artifact_prefix:
        raise ContractError("artifact_prefix must be a filename prefix")
    source = _inside(consumer_root, consumer_root / source_relative, "source")
    if not source.is_dir():
        raise ContractError(f"source does not exist: {source_relative}")
    cargo = _read_toml(source / "Cargo.toml", "binding Cargo manifest")
    package = _mapping(cargo.get("package"), "binding Cargo [package]")
    if _string(package.get("name"), "binding Cargo package.name") != cargo_package:
        raise ContractError("cargo_package does not match binding Cargo package.name")
    if package.get("version") != {"workspace": True}:
        raise ContractError("binding Cargo [package].version must inherit workspace")
    module = _module_name(source / "go.mod")
    contract, targets = _target_entries(source)
    generated = source / contract["generated_package"]
    if not generated.is_dir():
        raise ContractError("native target contract generated_package does not exist")
    return {
        "consumer_root": consumer_root,
        "source": source,
        "source_relative": source_relative.as_posix(),
        "cargo_package": cargo_package,
        "artifact_prefix": artifact_prefix,
        "module": module,
        "generated_package": contract["generated_package"],
        "native_library": contract["native_library"],
        "targets": targets,
    }


def load_config(config_path: Path) -> dict[str, Any]:
    """Validate config and derive every binding-owned identity from its source."""
    config_path = config_path.resolve()
    data = _read_toml(config_path, "go native module config")
    if set(data) != CONFIG_FIELDS or data.get("schema_version") != 1:
        raise ContractError("go native module config must contain exactly the v1 schema fields")
    result = validate_binding_source(
        config_path.parent.parent,
        _string(data.get("source"), "source"),
        _string(data.get("cargo_package"), "cargo_package"),
        _string(data.get("artifact_prefix"), "artifact_prefix"),
    )
    result["config_path"] = config_path
    return result


def _release_targets(path: Path) -> dict[str, dict[str, str]]:
    data = _read_toml(path, "release artifact manifest")
    raw_targets = data.get("release_targets")
    if not isinstance(raw_targets, list) or not raw_targets:
        raise ContractError("release artifact manifest must define non-empty [[release_targets]]")
    targets: dict[str, dict[str, str]] = {}
    for index, raw in enumerate(raw_targets, start=1):
        target = _mapping(raw, f"release target #{index}")
        entry = {
            "target": _string(target.get("target"), f"release target #{index}.target"),
            "os": _string(target.get("os"), f"release target #{index}.os"),
            "archive": _string(target.get("archive"), f"release target #{index}.archive"),
        }
        if entry["target"] in targets:
            raise ContractError(f"duplicate release target: {entry['target']}")
        targets[entry["target"]] = entry
    return targets


def _runner_matches_go_target(entry: dict[str, str]) -> bool:
    expected = {
        "linux": ("ubuntu-", "tar.gz"),
        "darwin": ("macos-", "tar.gz"),
        "windows": ("windows-", "zip"),
    }.get(entry["goos"])
    return expected is not None and entry["os"].startswith(expected[0]) and entry["archive"] == expected[1]


def target_matrix(manifest_path: Path, config_path: Path) -> dict[str, list[dict[str, str]]]:
    """Join binding targets to generic release runner/archive entries."""
    config = load_config(config_path)
    release_targets = _release_targets(manifest_path)
    include: list[dict[str, str]] = []
    for native in config["targets"]:
        release = release_targets.get(native["target"])
        if release is None:
            raise ContractError(f"native target lacks a generic release target: {native['target']}")
        entry = {
            **release,
            "goos": native["goos"],
            "goarch": native["goarch"],
            "library": native["library"],
            "cargo_package": config["cargo_package"],
            "module": config["module"],
            "artifact_prefix": config["artifact_prefix"],
        }
        if not _runner_matches_go_target(entry):
            raise ContractError(f"runner/archive does not match Go target: {native['target']}")
        include.append(entry)
    return {"include": include}


def _find_target(config: dict[str, Any], target_name: str) -> dict[str, str]:
    for target in config["targets"]:
        if target["target"] == target_name:
            return target
    raise ContractError(f"unknown Go native target: {target_name}")


def _validate_stage_version(version: str) -> str:
    if not SEMVER_RE.fullmatch(version):
        raise ContractError("version must be a non-empty semantic version without a v prefix")
    return version


def _safe_stage_output(output: Path, config: dict[str, Any]) -> Path:
    resolved = output.resolve()
    if resolved == config["consumer_root"] or resolved == config["source"]:
        raise ContractError("Go native module output must not be the consumer root or source")
    if config["source"] in resolved.parents:
        raise ContractError("Go native module output must not be nested in the source")
    if output.exists():
        raise ContractError(f"Go native module output already exists: {output}")
    if not resolved.parent.is_dir():
        raise ContractError("Go native module output parent must exist")
    return resolved


def stage(config_path: Path, target_name: str, native_library: Path, output: Path, version: str) -> Path:
    """Create a complete target-specific module atomically, or leave no output."""
    config = load_config(config_path)
    target = _find_target(config, target_name)
    _validate_stage_version(version)
    library = native_library.resolve()
    if not library.is_file():
        raise ContractError(f"native library does not exist: {native_library}")
    if library.name != target["library"]:
        raise ContractError(
            f"native library name mismatch for {target_name}: expected {target['library']}, got {library.name}"
        )
    destination = _safe_stage_output(output, config)
    required = ("go.mod", "README.md", "go", "testdata", "native/targets.toml")
    for relative in required:
        if not (config["source"] / relative).exists():
            raise ContractError(f"source asset is missing: {relative}")

    temporary = Path(tempfile.mkdtemp(prefix="go-native-module-", dir=destination.parent))
    try:
        source = config["source"]
        shutil.copy2(source / "go.mod", temporary / "go.mod")
        shutil.copy2(source / "README.md", temporary / "README.md")
        shutil.copytree(source / "go", temporary / "go")
        shutil.copytree(source / "testdata", temporary / "testdata")
        (temporary / "native").mkdir()
        shutil.copy2(source / "native" / "targets.toml", temporary / "native" / "targets.toml")
        native_destination = temporary / "native" / target_name
        native_destination.mkdir()
        shutil.copy2(library, native_destination / target["library"])
        (temporary / "VERSION").write_text(f"{version}\n", encoding="utf-8")
        temporary.rename(destination)
    except Exception:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    return destination


def verify_version_lockstep(config_path: Path, workspace_toml: Path) -> str:
    """Verify binding workspace inheritance and return the shared release version."""
    load_config(config_path)
    workspace = _read_toml(workspace_toml, "workspace Cargo manifest")
    package = _mapping(workspace.get("workspace"), "workspace [workspace]")
    workspace_package = _mapping(package.get("package"), "workspace [workspace.package]")
    return _string(workspace_package.get("version"), "workspace.package.version")


def _emit_json(payload: dict[str, Any]) -> None:
    print(json.dumps(payload, separators=(",", ":"), sort_keys=False))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    matrix_parser = subparsers.add_parser("target-matrix")
    matrix_parser.add_argument("--manifest", type=Path, required=True)
    matrix_parser.add_argument("--config", type=Path, required=True)
    stage_parser = subparsers.add_parser("stage")
    stage_parser.add_argument("--config", type=Path, required=True)
    stage_parser.add_argument("--target", required=True)
    stage_parser.add_argument("--native-library", type=Path, required=True)
    stage_parser.add_argument("--output", type=Path, required=True)
    stage_parser.add_argument("--version", required=True)
    lockstep_parser = subparsers.add_parser("verify-version-lockstep")
    lockstep_parser.add_argument("--config", type=Path, required=True)
    lockstep_parser.add_argument("--workspace-toml", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "target-matrix":
            _emit_json(target_matrix(args.manifest, args.config))
        elif args.command == "stage":
            print(stage(args.config, args.target, args.native_library, args.output, args.version))
        else:
            print(verify_version_lockstep(args.config, args.workspace_toml))
    except (ContractError, OSError) as error:
        print(f"go-native-module {args.command} failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
