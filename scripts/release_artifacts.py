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


POST_RELEASE_CHANNELS = frozenset({"pypi", "homebrew", "winget", "scoop"})


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


def _renderer_archive_path(manifest: dict) -> str:
    value = _require_project(manifest).get("renderer_archive_path")
    if not isinstance(value, str) or not value:
        raise SystemExit("[project].renderer_archive_path must be a non-empty string")
    return value


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


def _channel_names(manifest: dict) -> tuple[str, ...]:
    channels = manifest["channels"]
    if not isinstance(channels, dict):
        raise SystemExit("[channels] must be a table")
    unknown = sorted(set(channels) - POST_RELEASE_CHANNELS)
    if unknown:
        raise SystemExit("unsupported release channel(s): " + ", ".join(unknown))
    if not channels:
        raise SystemExit("manifest must define at least one [channels.<name>] table")
    return tuple(channels)


def _channel_dispatch_config(manifest: dict, channel_name: str) -> tuple[str, dict[str, str]]:
    channel = _channel_config(manifest, channel_name)
    _require_keys(channel, ("workflow", "dispatch_inputs"), f"[channels.{channel_name}]")
    workflow = channel["workflow"]
    dispatch_inputs = channel["dispatch_inputs"]
    if not isinstance(workflow, str) or not workflow:
        raise SystemExit(f"[channels.{channel_name}].workflow must be a non-empty string")
    if not isinstance(dispatch_inputs, dict) or not all(
        isinstance(key, str) and isinstance(value, str)
        for key, value in dispatch_inputs.items()
    ):
        raise SystemExit(
            f"[channels.{channel_name}].dispatch_inputs must be a string-to-string table"
        )
    if "tag" in dispatch_inputs:
        raise SystemExit(f"[channels.{channel_name}].dispatch_inputs must not override tag")
    return workflow, dispatch_inputs


def _channel_credential_rehearsal(
    manifest: dict, channel_name: str
) -> tuple[str, dict[str, str]] | None:
    """Return a safe channel rehearsal for credentials not safely probed in preflight."""
    channel = _channel_config(manifest, channel_name)
    rehearsal_inputs = channel.get("credential_rehearsal_inputs")
    if rehearsal_inputs is None:
        return None
    if not isinstance(rehearsal_inputs, dict) or not all(
        isinstance(key, str) and isinstance(value, str)
        for key, value in rehearsal_inputs.items()
    ):
        raise SystemExit(
            f"[channels.{channel_name}].credential_rehearsal_inputs "
            "must be a string-to-string table"
        )
    if "tag" in rehearsal_inputs:
        raise SystemExit(
            f"[channels.{channel_name}].credential_rehearsal_inputs must not override tag"
        )
    workflow, _ = _channel_dispatch_config(manifest, channel_name)
    return workflow, rehearsal_inputs


def _post_release_channel_preflight(manifest: dict, channel_name: str) -> dict[str, object]:
    """Return the non-secret readiness contract a channel worker must consume."""
    repository_secrets: list[str] = []
    environment_secrets: list[dict[str, str]] = []
    liveness_checks: list[dict[str, str]] = []

    if channel_name == "homebrew":
        repository_secrets.append("HOMEBREW_TAP_TOKEN")
        liveness_checks.append({"name": "HOMEBREW_TAP_TOKEN", "kind": "github"})
    elif channel_name == "winget":
        repository_secrets.append("WINGET_GITHUB_TOKEN")
        liveness_checks.append({"name": "WINGET_GITHUB_TOKEN", "kind": "github"})
    elif channel_name == "scoop":
        repository_secrets.append("SCOOP_BUCKET_TOKEN")
        liveness_checks.append({"name": "SCOOP_BUCKET_TOKEN", "kind": "github"})
    elif channel_name == "pypi":
        environment_secrets.extend(
            (
                {"environment": "pypi", "name": "PYPI_API_TOKEN"},
                {"environment": "testpypi", "name": "TEST_PYPI_API_TOKEN"},
            )
        )

    rehearsal = _channel_credential_rehearsal(manifest, channel_name)
    rehearsal_plan = None
    if rehearsal is not None:
        workflow, inputs = rehearsal
        rehearsal_plan = {"workflow": workflow, "inputs": inputs}

    return {
        "repository_secrets": repository_secrets,
        "environment_secrets": environment_secrets,
        "liveness_checks": liveness_checks,
        "credential_rehearsal": rehearsal_plan,
    }


def _root_channel_preflight(manifest: dict) -> list[dict[str, object]]:
    """Return non-secret requirements for root-workflow publish channels."""
    channels: list[dict[str, object]] = []
    if manifest["crates"]:
        channels.append(
            {
                "name": "crates_io",
                "repository_secrets": ["CARGO_REGISTRY_TOKEN"],
                "environment_secrets": [],
                "liveness_checks": [
                    {"name": "CARGO_REGISTRY_TOKEN", "kind": "crates_io"}
                ],
                "credential_rehearsal": None,
            }
        )
    channels.append(
        {
            "name": "github_release",
            "repository_secrets": [],
            "environment_secrets": [],
            "liveness_checks": [],
            "github_actions_permissions": ["contents:write"],
            "credential_rehearsal": None,
        }
    )
    return channels


def _preflight_outcome_status(outcome: str | None) -> str:
    """Map a GitHub Actions step outcome to a non-disclosing check status."""
    if outcome == "success":
        return "passed"
    if outcome in ("failure", "cancelled"):
        return "failed"
    return "blocked"


def _channel_preflight_result(
    channel: dict[str, object], outcomes: dict[str, str], tag: str | None
) -> dict[str, object]:
    """Materialize one worker-consumable result from its contract and check outcomes."""
    checks: list[dict[str, object]] = []

    for requirement, outcome_key in (
        ("publisher ownership", "ownership"),
        ("normalized release tag", "release_metadata"),
    ):
        checks.append(
            {
                "kind": "release_authorization",
                "requirements": [requirement],
                "status": _preflight_outcome_status(outcomes.get(outcome_key)),
            }
        )

    for key, outcome_key in (
        ("repository_secrets", "repository_secrets"),
        ("environment_secrets", "environment_secrets"),
        ("liveness_checks", "credential_liveness"),
        ("github_actions_permissions", "github_release_permissions"),
    ):
        requirements = channel.get(key, [])
        if requirements:
            checks.append(
                {
                    "kind": key,
                    "requirements": requirements,
                    "status": _preflight_outcome_status(outcomes.get(outcome_key)),
                }
            )

    rehearsal = channel.get("credential_rehearsal")
    if rehearsal is not None:
        checks.append(
            {
                "kind": "credential_rehearsal",
                "requirement": rehearsal,
                "status": "required",
            }
        )

    statuses = [check["status"] for check in checks if check["status"] != "required"]
    if "failed" in statuses:
        status = "failed"
        diagnostic = "PREFLIGHT.CHECK_FAILED"
    elif "blocked" in statuses:
        status = "blocked"
        diagnostic = "PREFLIGHT.CHECK_BLOCKED"
    else:
        status = "passed"
        diagnostic = ""

    return {
        "name": channel["name"],
        "tag": tag,
        "status": status,
        "checks": checks,
        "sanitized_diagnostic": diagnostic,
    }


def cmd_channel_preflight_results(args: argparse.Namespace) -> int:
    """Emit one non-secret result for every root and post-release channel."""
    try:
        outcomes = json.loads(args.outcomes)
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid preflight outcomes JSON: {error.msg}") from error
    if not isinstance(outcomes, dict) or not all(
        isinstance(name, str) and isinstance(outcome, str)
        for name, outcome in outcomes.items()
    ):
        raise SystemExit("preflight outcomes must be a string-to-string object")

    manifest = load_manifest(Path(args.manifest))
    contracts = [
        *_root_channel_preflight(manifest),
        *[
            {"name": channel_name, **_post_release_channel_preflight(manifest, channel_name)}
            for channel_name in _channel_names(manifest)
        ],
    ]
    tag = args.tag or None
    results = [
        _channel_preflight_result(channel, outcomes, tag) for channel in contracts
    ]
    print(json.dumps({"tag": tag, "channels": results}, separators=(",", ":")))
    return 0


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


def _validate_homebrew_bundle_destinations(binaries: list[dict]) -> None:
    """Require explicit, safe Homebrew Pathname components for bundled assets."""
    for binary in binaries:
        for bundle in binary.get("bundled_paths", []):
            components = bundle.get("homebrew_destination_components")
            if not isinstance(components, list) or not components or not all(
                isinstance(component, str) and component for component in components
            ):
                raise SystemExit(
                    "bundled_paths entry must define non-empty "
                    "homebrew_destination_components when Homebrew is configured"
                )
            if re.fullmatch(r"[a-z_][a-z0-9_]*", components[0]) is None:
                raise SystemExit(
                    "bundled_paths homebrew_destination_components[0] must be a "
                    "lowercase Homebrew Pathname helper"
                )


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
    binaries = _release_binaries(manifest)
    channel_names = _channel_names(manifest)
    for channel_name in channel_names:
        _channel_dispatch_config(manifest, channel_name)
        _channel_credential_rehearsal(manifest, channel_name)
        _channel_asset_patterns(manifest, channel_name)
        if channel_name in ("homebrew", "scoop"):
            _renderer_archive_path(manifest)
    if "homebrew" in channel_names:
        _validate_homebrew_bundle_destinations(binaries)
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


def cmd_channel_dispatch_plan(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    channels = []
    for channel_name in _channel_names(manifest):
        workflow, dispatch_inputs = _channel_dispatch_config(manifest, channel_name)
        preflight = _post_release_channel_preflight(manifest, channel_name)
        rehearsal = preflight["credential_rehearsal"]
        rehearsal_plan = None
        if rehearsal is not None:
            rehearsal_plan = {
                "workflow": rehearsal["workflow"],
                "inputs": {"tag": args.tag, **rehearsal["inputs"]},
            }
        channels.append(
            {
                "name": channel_name,
                "workflow": workflow,
                "inputs": {"tag": args.tag, **dispatch_inputs},
                "credential_rehearsal": rehearsal_plan,
                "preflight": preflight,
            }
        )
    print(json.dumps({"channels": channels}, separators=(",", ":")))
    return 0


def cmd_preflight_secret_plan(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    channel_names = _channel_names(manifest)
    repository_secrets: list[str] = []
    liveness_checks: list[dict[str, str]] = []
    environment_secrets: list[dict[str, str]] = []

    if manifest["crates"]:
        repository_secrets.append("CARGO_REGISTRY_TOKEN")
        liveness_checks.append({"name": "CARGO_REGISTRY_TOKEN", "kind": "crates_io"})
    post_release_channels = []
    for channel_name in channel_names:
        channel_preflight = _post_release_channel_preflight(manifest, channel_name)
        repository_secrets.extend(channel_preflight["repository_secrets"])
        environment_secrets.extend(channel_preflight["environment_secrets"])
        liveness_checks.extend(channel_preflight["liveness_checks"])
        post_release_channels.append({"name": channel_name, **channel_preflight})

    print(
        json.dumps(
            {
                "repository_secrets": repository_secrets,
                "environment_secrets": environment_secrets,
                "liveness_checks": liveness_checks,
                "root_channels": _root_channel_preflight(manifest),
                "post_release_channels": post_release_channels,
            },
            separators=(",", ":"),
        )
    )
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

    p = sub.add_parser("release-package-config")
    p.add_argument("--manifest", required=True)
    p.add_argument("--target", required=True)
    p.set_defaults(func=cmd_release_package_config)

    p = sub.add_parser("channel-config")
    p.add_argument("--manifest", required=True)
    p.add_argument("--channel", required=True)
    p.set_defaults(func=cmd_channel_config)

    p = sub.add_parser("channel-dispatch-plan")
    p.add_argument("--manifest", required=True)
    p.add_argument("--tag", required=True)
    p.set_defaults(func=cmd_channel_dispatch_plan)

    p = sub.add_parser("preflight-secret-plan")
    p.add_argument("--manifest", required=True)
    p.set_defaults(func=cmd_preflight_secret_plan)

    p = sub.add_parser("channel-preflight-results")
    p.add_argument("--manifest", required=True)
    p.add_argument("--outcomes", required=True)
    p.add_argument("--tag", required=True)
    p.set_defaults(func=cmd_channel_preflight_results)

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
