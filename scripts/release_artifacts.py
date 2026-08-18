#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import tarfile
import tomllib
import zipfile
from email import message_from_bytes
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
        "project": data.get("project", {}),
        "crates": crates,
        "release_binaries": release_binaries,
        "release_targets": data.get("release_targets", []),
        "go_native": data.get("go_native"),
        "python_packages": python_packages,
        "python_distributions": python_distributions,
        "channels": data.get("channels", {}),
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


def _resolve_workspace_path(workspace_toml: Path, relative_path: str) -> Path:
    return workspace_toml.parent / relative_path


def _assert_workspace_inherited_version(workspace_toml: Path, relative_path: str) -> None:
    path = _resolve_workspace_path(workspace_toml, relative_path)
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    value = data.get("package", {}).get("version")
    if not isinstance(value, dict) or value.get("workspace") is not True:
        raise SystemExit(
            f"{relative_path}: [package].version must inherit workspace.package.version"
        )


def _assert_python_package_version(
    workspace_toml: Path,
    relative_path: str,
    expected_version: str,
) -> None:
    path = _resolve_workspace_path(workspace_toml, relative_path)
    actual_version = _python_project_version(path)
    if actual_version != expected_version:
        raise SystemExit(
            f"{relative_path}: [project].version mismatch: "
            f"expected {expected_version}, got {actual_version!r}"
        )


def _python_project_version(pyproject_toml: Path) -> str:
    data = tomllib.loads(pyproject_toml.read_text(encoding="utf-8"))
    project = data.get("project", {})
    version = project.get("version")
    if not isinstance(version, str):
        raise SystemExit(f"{pyproject_toml}: [project].version must be a string")
    return version


def _python_project_name(pyproject_toml: Path) -> str:
    data = tomllib.loads(pyproject_toml.read_text(encoding="utf-8"))
    project = data.get("project", {})
    name = project.get("name")
    if not isinstance(name, str):
        raise SystemExit(f"{pyproject_toml}: [project].name must be a string")
    return name


def _python_distribution_entries(manifest: dict) -> list[dict]:
    """Return normalized Python distribution entries from the release manifest.

    The manifest is deliberately the package inventory for both build and
    post-release upload workflows.  Optional paths accommodate repositories
    whose Maturin or module layout differs from the repository's convention.
    """
    packages = {entry["package"]: entry for entry in manifest["python_packages"]}
    entries: list[dict] = []
    for distribution in manifest["python_distributions"]:
        package = packages[distribution["name"]]
        source = distribution["source"]
        entries.append(
            {
                "artifact": package["artifact"],
                "name": distribution["name"],
                "source": source,
                "pyproject": package["manifest"],
                "cargo_manifest": distribution.get("cargo_manifest", f"{source}/Cargo.toml"),
                "module_path": distribution.get(
                    "module_path", f"{source}/python/{package['module']}"
                ),
                "sdist": distribution["sdist"],
                "wheels": distribution["wheels"],
            }
        )
    return entries


def _python_distribution_expectations(manifest: dict) -> dict[str, dict[str, int]]:
    return {
        entry["name"]: {
            "wheel": len(entry["wheels"]),
            "sdist": int(entry["sdist"]),
        }
        for entry in _python_distribution_entries(manifest)
    }


def _require_project(manifest: dict) -> dict:
    project = manifest["project"]
    _require_keys(
        project,
        ("name", "archive_prefix", "description", "homepage", "license"),
        "[project]",
    )
    return project


def _release_targets_by_name(manifest: dict) -> dict[str, dict]:
    targets: dict[str, dict] = {}
    for index, target in enumerate(manifest["release_targets"], start=1):
        _require_keys(target, ("target", "os", "archive"), f"[[release_targets]] #{index}")
        name = target["target"]
        if name in targets:
            raise SystemExit(f"duplicate release target: {name}")
        targets[name] = target
    if not targets:
        raise SystemExit("manifest must define [[release_targets]]")
    return targets


def _channel_config(manifest: dict, channel_name: str) -> dict:
    try:
        channel = manifest["channels"][channel_name]
    except KeyError as error:
        raise SystemExit(f"manifest must define [channels.{channel_name}]") from error
    if not isinstance(channel, dict):
        raise SystemExit(f"[channels.{channel_name}] must be a table")
    return channel


def _channel_renderer_target(manifest: dict, channel_name: str) -> dict | None:
    """Return the published Linux renderer asset required by a channel workflow."""
    if channel_name not in ("homebrew", "scoop"):
        return None

    channel = _channel_config(manifest, channel_name)
    _require_keys(channel, ("renderer_target",), f"[channels.{channel_name}]")
    target_name = channel["renderer_target"]
    targets = _release_targets_by_name(manifest)
    try:
        target = targets[target_name]
    except KeyError as error:
        raise SystemExit(
            f"[channels.{channel_name}].renderer_target references unknown release target: {target_name}"
        ) from error
    if target["os"] != "ubuntu-latest" or target["archive"] != "tar.gz":
        raise SystemExit(
            f"[channels.{channel_name}].renderer_target must name an ubuntu-latest tar.gz release target"
        )
    return target


def _release_asset_pattern(project: dict, target: dict) -> str:
    return (
        rf"^{re.escape(project['archive_prefix'])}_.*_"
        rf"{re.escape(target['target'])}\.{re.escape(target['archive'])}$"
    )


def _release_binaries(manifest: dict) -> list[dict]:
    binaries = manifest["release_binaries"]
    if not binaries:
        raise SystemExit("manifest must define [[release_binaries]]")
    for index, binary in enumerate(binaries, start=1):
        _require_keys(binary, ("name",), f"[[release_binaries]] #{index}")
        for bundle in binary.get("bundled_paths", []):
            _require_keys(bundle, ("source", "destination"), "bundled_paths entry")
    return binaries


def _go_native_config(manifest: dict) -> dict | None:
    """Return the optional generated-Go native-module release configuration."""
    config = manifest["go_native"]
    if config is None:
        return None
    if not isinstance(config, dict):
        raise SystemExit("[go_native] must be a table")
    _require_keys(
        config,
        ("module", "source", "package", "cargo_package", "artifact_prefix", "tag_prefix", "targets"),
        "[go_native]",
    )
    if not all(isinstance(config[key], str) and config[key] for key in config if key != "targets"):
        raise SystemExit("[go_native] string fields must be non-empty")
    if not isinstance(config["targets"], list) or not config["targets"]:
        raise SystemExit("[go_native].targets must be a non-empty array")

    source = Path(config["source"])
    package = source / config["package"]
    go_mod = source / "go.mod"
    if not go_mod.is_file():
        raise SystemExit(f"[go_native].source is missing go.mod: {source}")
    if not package.is_dir():
        raise SystemExit(f"[go_native].package does not exist: {package}")
    native_targets = source / "native" / "targets.toml"
    if not native_targets.is_file():
        raise SystemExit(f"[go_native].source is missing native target contract: {native_targets}")
    go_module_line = f"module {config['module']}"
    if go_module_line not in go_mod.read_text(encoding="utf-8").splitlines():
        raise SystemExit(f"{go_mod}: must declare {go_module_line}")

    release_targets = _release_targets_by_name(manifest)
    seen_go_targets: set[tuple[str, str]] = set()
    seen_rust_targets: set[str] = set()
    for index, target in enumerate(config["targets"], start=1):
        if not isinstance(target, dict):
            raise SystemExit(f"[go_native].targets #{index} must be a table")
        _require_keys(
            target,
            ("rust_target", "goos", "goarch", "library"),
            f"[go_native].targets #{index}",
        )
        rust_target = target["rust_target"]
        go_target = (target["goos"], target["goarch"])
        library = target["library"]
        if not all(isinstance(value, str) and value for value in (rust_target, *go_target, library)):
            raise SystemExit(f"[go_native].targets #{index} fields must be non-empty strings")
        if Path(library).name != library:
            raise SystemExit(f"[go_native].targets #{index}.library must be a filename")
        if rust_target not in release_targets:
            raise SystemExit(
                f"[go_native].targets #{index}.rust_target references unknown release target: {rust_target}"
            )
        if rust_target in seen_rust_targets or go_target in seen_go_targets:
            raise SystemExit(f"duplicate [go_native] target: {rust_target} / {target['goos']}/{target['goarch']}")
        seen_rust_targets.add(rust_target)
        seen_go_targets.add(go_target)

    contract = tomllib.loads(native_targets.read_text(encoding="utf-8"))
    contract_targets = contract.get("targets", [])
    contract_index = {
        entry.get("rust_target"): entry
        for entry in contract_targets
        if isinstance(entry, dict) and isinstance(entry.get("rust_target"), str)
    }
    if set(contract_index) != seen_rust_targets:
        raise SystemExit("native target contract and [go_native].targets must name the same Rust targets")
    for target in config["targets"]:
        contract_target = contract_index[target["rust_target"]]
        if any(contract_target.get(key) != target[key] for key in ("goos", "goarch", "library")):
            raise SystemExit(
                "native target contract disagrees with [go_native].targets for "
                f"{target['rust_target']}"
            )
    return config


def _channel_asset_patterns(manifest: dict, channel_name: str) -> list[str]:
    project = _require_project(manifest)
    targets = _release_targets_by_name(manifest)
    channel = _channel_config(manifest, channel_name)
    if channel_name == "homebrew":
        assets = channel.get("assets", [])
        if not assets:
            raise SystemExit("[channels.homebrew] must define [[channels.homebrew.assets]]")
        target_names = []
        for asset in assets:
            _require_keys(asset, ("key", "target"), "[[channels.homebrew.assets]]")
            target_names.append(asset["target"])
    elif channel_name in ("winget", "scoop"):
        _require_keys(channel, ("installer_target",), f"[channels.{channel_name}]")
        target_names = [channel["installer_target"]]
    else:
        return []

    renderer_target = _channel_renderer_target(manifest, channel_name)
    if renderer_target is not None:
        target_names.append(renderer_target["target"])

    missing = [name for name in target_names if name not in targets]
    if missing:
        raise SystemExit(
            f"[channels.{channel_name}] references unknown release target(s): {', '.join(missing)}"
        )
    return [
        _release_asset_pattern(project, targets[name])
        for name in dict.fromkeys(target_names)
    ]


def cmd_validate_manifest(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    _require_project(manifest)
    _release_targets_by_name(manifest)
    _release_binaries(manifest)
    _go_native_config(manifest)
    for channel_name in ("pypi", "homebrew", "winget", "scoop"):
        _channel_config(manifest, channel_name)
        _channel_asset_patterns(manifest, channel_name)
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
        if not manifest_path.is_file():
            raise SystemExit(f"{manifest_path}: missing Python package manifest")
        python_package_version = _python_project_version(manifest_path)
        if not python_package_version:
            raise SystemExit(f"{manifest_path}: missing [project].version")
        actual_package_name = _python_project_name(manifest_path)
        if actual_package_name != package["package"]:
            raise SystemExit(
                f"{manifest_path}: python package mismatch: manifest={package['package']} actual={actual_package_name}"
            )
        python_packages_by_name[package["package"]] = package
    for index, distribution in enumerate(manifest["python_distributions"], start=1):
        _require_keys(distribution, ("name", "source", "sdist", "wheels"), f"[[python_distributions]] #{index}")
        if distribution["name"] not in python_packages_by_name:
            raise SystemExit(
                f"[[python_distributions]] #{index}: no matching [[python_packages]] entry for {distribution['name']}"
            )
        source = Path(distribution["source"])
        if not source.is_dir():
            raise SystemExit(f"[[python_distributions]] #{index}: source directory does not exist: {source}")
        if not isinstance(distribution["sdist"], bool):
            raise SystemExit(f"[[python_distributions]] #{index}: sdist must be a boolean")
        wheels = distribution["wheels"]
        if not isinstance(wheels, list) or not all(isinstance(entry, str) for entry in wheels):
            raise SystemExit(f"[[python_distributions]] #{index}: wheels must be a list of strings")
        cargo_manifest = Path(distribution.get("cargo_manifest", source / "Cargo.toml"))
        if not cargo_manifest.is_file():
            raise SystemExit(
                f"[[python_distributions]] #{index}: missing Maturin Cargo manifest: {cargo_manifest}"
            )
        package = python_packages_by_name[distribution["name"]]
        module_root = Path(distribution.get("module_path", source / "python" / package["module"]))
        if not module_root.is_dir():
            raise SystemExit(
                f"[[python_distributions]] #{index}: Python module path does not exist: {module_root}"
            )
    print("manifest validation passed")
    return 0


def cmd_list_publish_plan(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    for crate in manifest["crates"]:
        print(f"{crate['package']}|{crate['wait_after_publish_seconds']}")
    return 0


def cmd_python_wheel_matrix(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    include = [
        {
            "artifact": distribution["artifact"],
            "name": distribution["name"],
            "os": os_name,
            "pyproject": distribution["pyproject"],
            "cargo_manifest": distribution["cargo_manifest"],
        }
        for distribution in _python_distribution_entries(manifest)
        for os_name in distribution["wheels"]
    ]
    if not include:
        raise SystemExit("manifest must define at least one Python wheel build")
    print(json.dumps({"include": include}, separators=(",", ":")))
    return 0


def cmd_python_sdist_matrix(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    include = [
        {
            "artifact": distribution["artifact"],
            "name": distribution["name"],
            "pyproject": distribution["pyproject"],
            "cargo_manifest": distribution["cargo_manifest"],
        }
        for distribution in _python_distribution_entries(manifest)
        if distribution["sdist"]
    ]
    print(json.dumps({"include": include}, separators=(",", ":")))
    return 0


def cmd_release_target_matrix(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    print(json.dumps({"include": list(_release_targets_by_name(manifest).values())}, separators=(",", ":")))
    return 0


def cmd_go_native_target_matrix(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    config = _go_native_config(manifest)
    if config is None:
        raise SystemExit("manifest does not define [go_native]")
    release_targets = _release_targets_by_name(manifest)
    include = [
        {
            **release_targets[target["rust_target"]],
            "goos": target["goos"],
            "goarch": target["goarch"],
            "library": target["library"],
        }
        for target in config["targets"]
    ]
    print(json.dumps({"include": include}, separators=(",", ":")))
    return 0


def _go_native_target(config: dict, rust_target: str) -> dict:
    for target in config["targets"]:
        if target["rust_target"] == rust_target:
            return target
    raise SystemExit(f"unknown Go native target: {rust_target}")


def cmd_stage_go_native_module(args: argparse.Namespace) -> int:
    """Create a self-contained, target-specific static-link Go module bundle."""
    manifest = load_manifest(Path(args.manifest))
    config = _go_native_config(manifest)
    if config is None:
        raise SystemExit("manifest does not define [go_native]")
    target = _go_native_target(config, args.target)
    source = Path(config["source"])
    library = Path(args.native_library)
    if not library.is_file():
        raise SystemExit(f"native library does not exist: {library}")
    if library.name != target["library"]:
        raise SystemExit(
            f"native library name mismatch for {args.target}: "
            f"expected {target['library']}, got {library.name}"
        )

    output = Path(args.output)
    if output.exists():
        raise SystemExit(f"Go native module output already exists: {output}")
    output.mkdir(parents=True)
    shutil.copy2(source / "go.mod", output / "go.mod")
    shutil.copy2(source / "README.md", output / "README.md")
    shutil.copytree(source / "go", output / "go")
    shutil.copytree(source / "testdata", output / "testdata")
    native_output = output / "native"
    native_output.mkdir()
    shutil.copy2(source / "native" / "targets.toml", native_output / "targets.toml")
    target_output = native_output / args.target
    target_output.mkdir()
    shutil.copy2(library, target_output / library.name)
    (output / "VERSION").write_text(f"{args.version}\n", encoding="utf-8")
    print(output)
    return 0


def cmd_install_go_native_library(args: argparse.Namespace) -> int:
    """Install a locally built static library for source-tree Go tests only."""
    manifest = load_manifest(Path(args.manifest))
    config = _go_native_config(manifest)
    if config is None:
        raise SystemExit("manifest does not define [go_native]")
    target = _go_native_target(config, args.target)
    library = Path(args.native_library)
    if not library.is_file():
        raise SystemExit(f"native library does not exist: {library}")
    if library.name != target["library"]:
        raise SystemExit(
            f"native library name mismatch for {args.target}: "
            f"expected {target['library']}, got {library.name}"
        )
    destination = Path(config["source"]) / "native" / args.target / library.name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(library, destination)
    print(destination)
    return 0


def cmd_release_package_config(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    targets = _release_targets_by_name(manifest)
    try:
        target = targets[args.target]
    except KeyError as error:
        raise SystemExit(f"unknown release target: {args.target}") from error
    binaries = _release_binaries(manifest)
    print(
        json.dumps(
            {"project": _require_project(manifest), "target": target, "binaries": binaries},
            separators=(",", ":"),
        )
    )
    return 0


def cmd_channel_config(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    project = _require_project(manifest)
    channel = _channel_config(manifest, args.channel)
    result = {
        "project": project,
        "channel": channel,
        "asset_patterns": _channel_asset_patterns(manifest, args.channel),
        "release_binaries": manifest["release_binaries"],
        "release_targets": _release_targets_by_name(manifest),
    }
    print(json.dumps(result, separators=(",", ":")))
    return 0


def _python_distribution_name_from_wheel(path: Path, expected: set[str]) -> str:
    with zipfile.ZipFile(path) as archive:
        metadata = [name for name in archive.namelist() if name.endswith(".dist-info/METADATA")]
        if len(metadata) != 1:
            raise SystemExit(f"{path}: expected exactly one wheel METADATA file")
        name = message_from_bytes(archive.read(metadata[0])).get("Name")
    if name not in expected:
        raise SystemExit(f"{path}: unexpected Python distribution {name!r}")
    return name


def _python_distribution_name_from_sdist(path: Path, expected: set[str]) -> str | None:
    with tarfile.open(path, "r:gz") as archive:
        metadata = [member for member in archive.getmembers() if member.name.endswith("/PKG-INFO")]
        if not metadata:
            return None
        if len(metadata) != 1:
            raise SystemExit(f"{path}: expected exactly one sdist PKG-INFO file")
        extracted = archive.extractfile(metadata[0])
        if extracted is None:
            raise SystemExit(f"{path}: unable to read sdist PKG-INFO")
        name = message_from_bytes(extracted.read()).get("Name")
    if name not in expected:
        raise SystemExit(f"{path}: unexpected Python distribution {name!r}")
    return name


def cmd_verify_python_release_assets(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    asset_dir = Path(args.asset_dir)
    if not asset_dir.is_dir():
        raise SystemExit(f"Python asset directory does not exist: {asset_dir}")
    expected = _python_distribution_expectations(manifest)
    found = {name: {"wheel": 0, "sdist": 0} for name in expected}
    destination = Path(args.copy_to) if args.copy_to else None
    if destination:
        destination.mkdir(parents=True, exist_ok=True)

    for asset in sorted(asset_dir.iterdir()):
        if not asset.is_file():
            continue
        if asset.suffix == ".whl":
            name = _python_distribution_name_from_wheel(asset, set(expected))
            found[name]["wheel"] += 1
        elif asset.name.endswith(".tar.gz"):
            name = _python_distribution_name_from_sdist(asset, set(expected))
            if name is None:
                continue
            found[name]["sdist"] += 1
        else:
            continue
        if destination:
            shutil.copy2(asset, destination / asset.name)

    if found != expected:
        raise SystemExit(
            "published GitHub Release Python assets mismatch: "
            f"expected {expected}, found {found}"
        )
    print(f"verified Python release assets: {expected}")
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


def cmd_verify_version_lockstep(args: argparse.Namespace) -> int:
    workspace_toml = Path(args.workspace_toml)
    version = workspace_version(workspace_toml)
    manifest = load_manifest(Path(args.manifest))
    checked_cargo_manifests: set[str] = set()
    for crate in manifest["crates"]:
        cargo_toml = crate["cargo_toml"]
        _assert_workspace_inherited_version(workspace_toml, cargo_toml)
        checked_cargo_manifests.add(cargo_toml)
    for distribution in _python_distribution_entries(manifest):
        cargo_toml = distribution["cargo_manifest"]
        if cargo_toml not in checked_cargo_manifests:
            _assert_workspace_inherited_version(workspace_toml, cargo_toml)
            checked_cargo_manifests.add(cargo_toml)
    go_native = _go_native_config(manifest)
    if go_native is not None:
        cargo_toml = f"{go_native['source']}/Cargo.toml"
        if cargo_toml not in checked_cargo_manifests:
            _assert_workspace_inherited_version(workspace_toml, cargo_toml)
            checked_cargo_manifests.add(cargo_toml)
    for package in manifest["python_packages"]:
        _assert_python_package_version(workspace_toml, package["manifest"], version)
    print("version lockstep verification passed")
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


def _readme_dependency_crate(manifest: dict) -> str:
    project = manifest["project"]
    dependency_crate = project.get("readme_dependency_crate")
    if not isinstance(dependency_crate, str) or not dependency_crate:
        raise SystemExit("[project].readme_dependency_crate must be a non-empty string")
    if dependency_crate not in {crate["package"] for crate in manifest["crates"]}:
        raise SystemExit(
            "[project].readme_dependency_crate must name a package declared in [[crates]]"
        )
    return dependency_crate


def _readme_version_checks(
    version: str, dependency_crate: str
) -> tuple[tuple[str, str, str], ...]:
    minor_version = version.rsplit(".", 1)[0]
    return (
        (
            f"{dependency_crate} dependency example",
            rf'({re.escape(dependency_crate)}\s*=\s*")[^"]+(")',
            version,
        ),
        ("Status table Version row", rf'(\|\s*Version\s*\|\s*)[^\s|]+(\s*\|)', version),
        ("Status table Stability row", rf'(\|\s*Stability\s*\|\s*stable\s+)\S+(\s+release line\s*\|)', minor_version),
    )


def cmd_verify_readme_version(args: argparse.Namespace) -> int:
    version = workspace_version(Path(args.workspace_toml))
    dependency_crate = _readme_dependency_crate(load_manifest(Path(args.manifest)))
    readme = Path(args.readme)
    text = readme.read_text(encoding="utf-8")

    mismatches = []
    for label, pattern, expected in _readme_version_checks(version, dependency_crate):
        match = re.search(pattern, text)
        if match is None:
            raise SystemExit(f"{readme}: could not locate {label}")
        found = text[match.end(1):match.start(2)]
        if found != expected:
            mismatches.append(f"{label}: expected {expected}, found {found}")

    if mismatches:
        raise SystemExit(
            f"{readme}: stale version reference(s) (run 'sync-readme-version' to fix):\n"
            + "\n".join(mismatches)
        )
    print("readme version verification passed")
    return 0


def cmd_sync_readme_version(args: argparse.Namespace) -> int:
    version = workspace_version(Path(args.workspace_toml))
    dependency_crate = _readme_dependency_crate(load_manifest(Path(args.manifest)))
    readme = Path(args.readme)
    text = readme.read_text(encoding="utf-8")

    updated = 0
    for label, pattern, expected in _readme_version_checks(version, dependency_crate):
        new_text, count = re.subn(pattern, rf'\g<1>{expected}\g<2>', text, count=1)
        if count == 0:
            raise SystemExit(f"{readme}: could not locate {label}")
        text = new_text
        updated += count

    readme.write_text(text, encoding="utf-8")
    print(f"synced {updated} readme version reference(s) to {version}")
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

    p = sub.add_parser("python-wheel-matrix")
    p.add_argument("--manifest", required=True)
    p.set_defaults(func=cmd_python_wheel_matrix)

    p = sub.add_parser("python-sdist-matrix")
    p.add_argument("--manifest", required=True)
    p.set_defaults(func=cmd_python_sdist_matrix)

    p = sub.add_parser("release-target-matrix")
    p.add_argument("--manifest", required=True)
    p.set_defaults(func=cmd_release_target_matrix)

    p = sub.add_parser("go-native-target-matrix")
    p.add_argument("--manifest", required=True)
    p.set_defaults(func=cmd_go_native_target_matrix)

    p = sub.add_parser("stage-go-native-module")
    p.add_argument("--manifest", required=True)
    p.add_argument("--target", required=True)
    p.add_argument("--native-library", required=True)
    p.add_argument("--output", required=True)
    p.add_argument("--version", required=True)
    p.set_defaults(func=cmd_stage_go_native_module)

    p = sub.add_parser("install-go-native-library")
    p.add_argument("--manifest", required=True)
    p.add_argument("--target", required=True)
    p.add_argument("--native-library", required=True)
    p.set_defaults(func=cmd_install_go_native_library)

    p = sub.add_parser("release-package-config")
    p.add_argument("--manifest", required=True)
    p.add_argument("--target", required=True)
    p.set_defaults(func=cmd_release_package_config)

    p = sub.add_parser("channel-config")
    p.add_argument("--manifest", required=True)
    p.add_argument("--channel", choices=("pypi", "homebrew", "winget", "scoop"), required=True)
    p.set_defaults(func=cmd_channel_config)

    p = sub.add_parser("verify-python-release-assets")
    p.add_argument("--manifest", required=True)
    p.add_argument("--asset-dir", required=True)
    p.add_argument("--copy-to")
    p.set_defaults(func=cmd_verify_python_release_assets)

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

    p = sub.add_parser("verify-version-lockstep")
    p.add_argument("--manifest", required=True)
    p.add_argument("--workspace-toml", required=True)
    p.set_defaults(func=cmd_verify_version_lockstep)

    p = sub.add_parser("sync-python-version")
    p.add_argument("--workspace-toml", required=True)
    p.add_argument("--pyproject", required=True)
    p.set_defaults(func=cmd_sync_python_version)

    p = sub.add_parser("verify-readme-version")
    p.add_argument("--manifest", required=True)
    p.add_argument("--workspace-toml", required=True)
    p.add_argument("--readme", required=True)
    p.set_defaults(func=cmd_verify_readme_version)

    p = sub.add_parser("sync-readme-version")
    p.add_argument("--manifest", required=True)
    p.add_argument("--workspace-toml", required=True)
    p.add_argument("--readme", required=True)
    p.set_defaults(func=cmd_sync_readme_version)

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
