#!/usr/bin/env python3
"""Publish or install manifest-declared GitHub prerelease assets.

A prerelease carries the same manifest-declared assets a real release builds:
one binary archive per release target (when the manifest declares release
binaries), every declared Python wheel for every platform, each declared
sdist, and checksums.txt.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
from urllib.request import Request, urlopen
import zipfile
from typing import Any, Callable, Sequence


MINIMUM_PYTHON_VERSION = (3, 11)
try:
    import tomllib
except ModuleNotFoundError:  # Let old interpreters reach the typed remediation below.
    tomllib = None  # type: ignore[assignment]


STABLE_VERSION_PARTS = 3
PUBLIC_DOWNLOAD_TIMEOUT_SECONDS = 60
ARCHIVE_WORKFLOW = "prerelease-archive.yml"
ARCHIVE_WAIT_SECONDS = 20 * 60
ARCHIVE_POLL_SECONDS = 60


def command(args: Sequence[str], *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    """Run one external command with the current repository as its working directory."""
    return subprocess.run(args, check=True, text=True, capture_output=capture)


def require_supported_python() -> None:
    if sys.version_info < MINIMUM_PYTHON_VERSION:
        raise SystemExit(
            "PRERELEASE.PYTHON_VERSION_UNSUPPORTED: Python 3.11 or newer is required; "
            "fix: install Python 3.11+ and run prerelease.py with that interpreter "
            "(for example, python3.11 .claude/skills/prerelease/scripts/prerelease.py)."
        )


def shell(command_text: str, *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    """Run a manifest command after the manifest was validated by the installer."""
    return subprocess.run(command_text, shell=True, check=True, text=True, capture_output=capture)


def parse_version(value: str) -> str:
    if len(value.split(".")) != STABLE_VERSION_PARTS or not all(part.isdigit() for part in value.split(".")):
        raise SystemExit("version must be X.Y.Z")
    return value


def read_manifest(path: Path) -> dict[str, Any]:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise SystemExit(f"cannot read manifest {path}: {error}") from error
    if not isinstance(data, dict):
        raise SystemExit("publish-artifacts.toml must be a TOML table")
    return data


def prerelease_manifest(path: Path) -> dict[str, Any]:
    """Read the opt-in prerelease table from a publish artifacts manifest."""
    value = read_manifest(path).get("prerelease")
    if not isinstance(value, dict):
        raise SystemExit("[prerelease] is not enabled in release/publish-artifacts.toml")
    return value


def required_string(config: dict[str, Any], name: str) -> str:
    value = config.get(name)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"[prerelease].{name} must be a non-empty string")
    return value


def required_string_list(config: dict[str, Any], name: str) -> list[str]:
    value = config.get(name)
    if (
        not isinstance(value, list)
        or not value
        or not all(isinstance(item, str) and item for item in value)
    ):
        raise SystemExit(f"[prerelease].{name} must be a non-empty string array")
    return value


def release_targets(manifest: dict[str, Any]) -> list[dict[str, str]]:
    values = manifest.get("release_targets")
    if not isinstance(values, list):
        raise SystemExit("release_targets must be an array")
    targets: list[dict[str, str]] = []
    for value in values:
        if not isinstance(value, dict):
            raise SystemExit("release_targets entries must be tables")
        target = value.get("target")
        archive = value.get("archive")
        if not isinstance(target, str) or not isinstance(archive, str):
            raise SystemExit("release_targets entries require target and archive")
        targets.append({"target": target, "archive": archive})
    return targets


def archive_name(manifest: dict[str, Any], version: str, target: dict[str, str]) -> str:
    project = manifest.get("project")
    if not isinstance(project, dict) or not isinstance(project.get("archive_prefix"), str):
        raise SystemExit("[project].archive_prefix is required")
    return f"{project['archive_prefix']}_{version}_{target['target']}.{target['archive']}"


def release_binaries(manifest: dict[str, Any]) -> list[Any]:
    values = manifest.get("release_binaries", [])
    if not isinstance(values, list):
        raise SystemExit("release_binaries must be an array")
    return values


def archive_assets(manifest: dict[str, Any], version: str) -> set[str]:
    """Binary archives exist only when the manifest declares release binaries."""
    if not release_binaries(manifest):
        return set()
    return {archive_name(manifest, version, target) for target in release_targets(manifest)}


def expected_assets(manifest: dict[str, Any], version: str) -> set[str]:
    """Assets whose exact names the manifest determines (wheel names carry build tags)."""
    return {"checksums.txt", *archive_assets(manifest, version)}


def prerelease_binaries(config: dict[str, Any]) -> list[str]:
    values = config.get("binaries", [])
    if not isinstance(values, list) or not all(isinstance(value, str) and value for value in values):
        raise SystemExit("[prerelease].binaries must be a string array")
    return values


def python_distributions(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    values = manifest.get("python_distributions", [])
    if not isinstance(values, list):
        raise SystemExit("python_distributions must be an array")
    for value in values:
        if (
            not isinstance(value, dict)
            or not isinstance(value.get("name"), str)
            or not value["name"]
            or not isinstance(value.get("wheels", []), list)
        ):
            raise SystemExit("python_distributions entries require a name and a wheels array")
    return values


def normalized_distribution(name: str) -> str:
    """Return the wheel/sdist filename form of a distribution name (PEP 427/625)."""
    return re.sub(r"[-_.]+", "_", name).lower()


def parse_wheel(name: str) -> dict[str, Any] | None:
    if not name.endswith(".whl"):
        return None
    parts = name[: -len(".whl")].split("-")
    if len(parts) not in (5, 6):
        return None
    python_tags, abi_tags, platform_tags = (part.split(".") for part in parts[-3:])
    return {
        "name": name,
        "distribution": normalized_distribution(parts[0]),
        "version": parts[1],
        "python": python_tags,
        "abi": abi_tags,
        "platform": platform_tags,
    }


def parse_sdist(name: str) -> tuple[str, str] | None:
    if not name.endswith(".tar.gz"):
        return None
    distribution, separator, version = name[: -len(".tar.gz")].rpartition("-")
    if not separator or not distribution:
        return None
    return normalized_distribution(distribution), version


def distribution_assets(names: set[str], distribution: str, version: str) -> tuple[list[str], list[str]]:
    """Return the wheel and sdist asset names belonging to one distribution version."""
    expected = normalized_distribution(distribution)
    wheels = sorted(
        name
        for name in names
        if (wheel := parse_wheel(name)) is not None
        and wheel["distribution"] == expected
        and wheel["version"] == version
    )
    sdists = sorted(name for name in names if parse_sdist(name) == (expected, version))
    return wheels, sdists


def declared_wheel_platforms(distribution: dict[str, Any]) -> list[str]:
    return [
        entry["platform"]
        for entry in distribution.get("wheels", [])
        if isinstance(entry, dict) and isinstance(entry.get("platform"), str) and entry["platform"]
    ]


def require_python_assets(names: set[str], manifest: dict[str, Any], version: str) -> set[str]:
    """Require every declared wheel and sdist, matching the release asset contract."""
    required: set[str] = set()
    for distribution in python_distributions(manifest):
        label = distribution["name"]
        wheels, sdists = distribution_assets(names, label, version)
        expected_wheels = len(distribution.get("wheels", []))
        if len(wheels) != expected_wheels:
            raise SystemExit(
                f"GitHub prerelease has {len(wheels)} wheel(s) for {label} {version}; "
                f"the manifest declares {expected_wheels}"
            )
        for declared in declared_wheel_platforms(distribution):
            matches = [name for name in wheels if declared in parse_wheel(name)["platform"]]
            if len(matches) != 1:
                raise SystemExit(f"GitHub prerelease has no unique {declared} wheel for {label} {version}")
        expected_sdists = 1 if distribution.get("sdist") else 0
        if len(sdists) != expected_sdists:
            raise SystemExit(
                f"GitHub prerelease has {len(sdists)} sdist(s) for {label} {version}; "
                f"the manifest declares {expected_sdists}"
            )
        required.update(wheels)
        required.update(sdists)
    return required


def host_target(manifest: dict[str, Any]) -> dict[str, str]:
    triples = {
        ("Darwin", "x86_64"): "x86_64-apple-darwin",
        ("Darwin", "amd64"): "x86_64-apple-darwin",
        ("Darwin", "arm64"): "aarch64-apple-darwin",
        ("Darwin", "aarch64"): "aarch64-apple-darwin",
        ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
        ("Linux", "amd64"): "x86_64-unknown-linux-gnu",
        ("Linux", "arm64"): "aarch64-unknown-linux-gnu",
        ("Linux", "aarch64"): "aarch64-unknown-linux-gnu",
        ("Windows", "x86_64"): "x86_64-pc-windows-msvc",
        ("Windows", "amd64"): "x86_64-pc-windows-msvc",
    }
    system, machine = platform.system(), platform.machine().lower()
    try:
        triple = triples[(system, machine)]
    except KeyError as error:
        raise SystemExit(f"unsupported prerelease host: {system} {machine}") from error
    for target in release_targets(manifest):
        if target["target"] == triple:
            return target
    raise SystemExit(f"manifest does not declare a prerelease archive for {triple}")


def host_wheel_platforms(system: str, machine: str, libc: str) -> tuple[str, ...] | None:
    """Return platform-tag predicates (prefix, suffix) for this host, best first."""
    machine = machine.lower()
    if system == "Linux":
        arch = {"x86_64": "x86_64", "amd64": "x86_64", "aarch64": "aarch64", "arm64": "aarch64"}.get(machine)
        if arch is None:
            return None
        family = "manylinux" if libc == "glibc" else "musllinux"
        return (f"{family}|_{arch}", f"linux_{arch}|")
    if system == "Darwin":
        arch = {"x86_64": "x86_64", "amd64": "x86_64", "arm64": "arm64", "aarch64": "arm64"}.get(machine)
        if arch is None:
            return None
        return (f"macosx_|_{arch}", "macosx_|_universal2")
    if system == "Windows":
        arch = {"x86_64": "amd64", "amd64": "amd64", "arm64": "arm64", "aarch64": "arm64"}.get(machine)
        if arch is None:
            return None
        return (f"win_{arch}|",)
    return None


def platform_rank(tags: Sequence[str], predicates: Sequence[str]) -> int | None:
    """Rank a wheel's platform tags: higher is more specific; None is incompatible."""
    best: int | None = None
    for tag in tags:
        if tag == "any":
            rank = 0
        else:
            rank = None
            for position, predicate in enumerate(predicates):
                prefix, _, suffix = predicate.partition("|")
                if tag.startswith(prefix) and tag.endswith(suffix) and (suffix or tag == prefix):
                    rank = len(predicates) - position
                    break
        if rank is not None and (best is None or rank > best):
            best = rank
    return best


def python_rank(wheel: dict[str, Any], python: tuple[int, int], implementation: str) -> tuple[int, int] | None:
    """Rank a wheel's interpreter tags against the running interpreter."""
    major, minor = python
    best: tuple[int, int] | None = None
    for python_tag in wheel["python"]:
        for abi in wheel["abi"]:
            rank: tuple[int, int] | None = None
            if abi == "none" and python_tag in (f"py{major}", f"py{major}{minor}"):
                rank = (1, 0)
            elif implementation == "cpython" and (found := re.fullmatch(rf"cp{major}(\d+)", python_tag)):
                tagged_minor = int(found.group(1))
                if abi == "abi3" and tagged_minor <= minor:
                    rank = (2, tagged_minor)
                elif abi == f"cp{major}{minor}" and tagged_minor == minor:
                    rank = (3, tagged_minor)
            if rank is not None and (best is None or rank > best):
                best = rank
    return best


def host_wheel_names(
    names: Sequence[str] | set[str] | dict[str, str],
    manifest: dict[str, Any],
    version: str,
    *,
    system: str | None = None,
    machine: str | None = None,
    libc: str | None = None,
    python: tuple[int, int] | None = None,
    implementation: str | None = None,
) -> list[str]:
    """Select exactly one host-compatible wheel per declared distribution.

    Candidates are ranked by interpreter fit (exact CPython ABI, then the
    highest compatible abi3, then pure Python) and then by platform specificity.
    A tie at the top rank is ambiguous and fails closed instead of guessing.
    """
    system = platform.system() if system is None else system
    machine = platform.machine() if machine is None else machine
    libc = (platform.libc_ver()[0] or "musl") if libc is None and system == "Linux" else (libc or "")
    python = sys.version_info[:2] if python is None else python
    implementation = sys.implementation.name if implementation is None else implementation
    predicates = host_wheel_platforms(system, machine, libc)
    if predicates is None:
        raise SystemExit(f"unsupported prerelease wheel host: {system} {machine}")
    available = set(names)
    selected: list[str] = []
    for distribution in python_distributions(manifest):
        if not distribution.get("wheels"):
            continue
        wheels, _sdists = distribution_assets(available, distribution["name"], version)
        ranked: list[tuple[tuple[int, int, int], str]] = []
        for name in wheels:
            wheel = parse_wheel(name)
            interpreter = python_rank(wheel, python, implementation)
            host = platform_rank(wheel["platform"], predicates)
            if interpreter is not None and host is not None:
                ranked.append(((*interpreter, host), name))
        if not ranked:
            raise SystemExit(
                f"GitHub prerelease has no {system} {machine} wheel for {distribution['name']} {version}"
            )
        ranked.sort(reverse=True)
        if len(ranked) > 1 and ranked[0][0] == ranked[1][0]:
            tied = sorted(name for rank, name in ranked if rank == ranked[0][0])
            raise SystemExit(
                f"GitHub prerelease has ambiguous host wheels for {distribution['name']}: {', '.join(tied)}"
            )
        selected.append(ranked[0][1])
    return selected


def checksums(text: str) -> dict[str, str]:
    """Parse the sha256sum format emitted by prerelease-archive.yml."""
    values: dict[str, str] = {}
    for line in text.splitlines():
        digest, separator, filename = line.partition("  ")
        if not separator:
            continue
        filename = filename.removeprefix("*")
        if len(digest) == 64 and all(character in "0123456789abcdef" for character in digest.lower()):
            values[filename] = digest.lower()
    return values


def verify_checksum(path: Path, expected: str) -> None:
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"sha256 mismatch for {path.name}")




def gh_json(args: Sequence[str]) -> Any:
    result = command(["gh", *args], capture=True)
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise SystemExit(f"gh returned invalid JSON: {error}") from error


def gh_available() -> bool:
    """Return whether an authenticated gh CLI can read this repository's releases."""
    if shutil.which("gh") is None:
        return False
    status = subprocess.run(["gh", "auth", "status"], text=True, capture_output=True, check=False)
    return status.returncode == 0


def release_for_tag(tag: str) -> dict[str, Any]:
    release = gh_json(["release", "view", tag, "--json", "url,isPrerelease,isDraft,assets"])
    if not isinstance(release, dict) or release.get("isDraft") or not release.get("isPrerelease"):
        raise SystemExit(f"{tag} is not a published GitHub prerelease")
    return release


def latest_prerelease_tag(releases: Any, prefix: str) -> str:
    if not isinstance(releases, list):
        raise SystemExit("GitHub release list returned an invalid response")
    for release in releases:
        if not isinstance(release, dict):
            continue
        tag = release.get("tagName")
        if release.get("isPrerelease") and not release.get("isDraft") and isinstance(tag, str) and tag.startswith(prefix):
            return tag
    raise SystemExit("no published GitHub prerelease matches the manifest tag prefix")


def select_release(config: dict[str, Any], requested: str) -> tuple[str, dict[str, Any]]:
    prefix = required_string(config, "tag_prefix")
    if requested != "latest":
        version = parse_version(requested)
        return version, release_for_tag(f"{prefix}{version}")
    releases = gh_json(["release", "list", "--limit", "100", "--json", "tagName,isPrerelease,isDraft"])
    tag = latest_prerelease_tag(releases, prefix)
    return parse_version(tag.removeprefix(prefix)), release_for_tag(tag)


def public_repository(manifest: dict[str, Any]) -> str:
    """Return OWNER/REPO from [project].homepage for unauthenticated installs."""
    project = manifest.get("project")
    homepage = project.get("homepage") if isinstance(project, dict) else None
    found = (
        re.fullmatch(r"https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+?)(?:\.git)?/?", homepage)
        if isinstance(homepage, str)
        else None
    )
    if found is None:
        raise SystemExit(
            "PRERELEASE.GH_UNAVAILABLE: gh is not installed or not authenticated, and "
            "[project].homepage is not a https://github.com/OWNER/REPO URL; fix: run "
            "`gh auth login`, or declare the GitHub repository URL as [project].homepage."
        )
    return f"{found.group(1)}/{found.group(2)}"


def public_json(url: str) -> Any:
    request = Request(url, headers={"Accept": "application/vnd.github+json", "User-Agent": "sc-publish-prerelease"})
    with urlopen(request, timeout=PUBLIC_DOWNLOAD_TIMEOUT_SECONDS) as response:  # noqa: S310 - fixed GitHub API host
        try:
            return json.loads(response.read())
        except json.JSONDecodeError as error:
            raise SystemExit(f"GitHub API returned invalid JSON for {url}: {error}") from error


def public_release_for_tag(repository: str, tag: str) -> dict[str, Any]:
    """Read one public release through the unauthenticated GitHub REST API."""
    release = public_json(f"https://api.github.com/repos/{repository}/releases/tags/{tag}")
    if not isinstance(release, dict) or release.get("draft") or not release.get("prerelease"):
        raise SystemExit(f"{tag} is not a published GitHub prerelease")
    assets = release.get("assets")
    return {
        "url": release.get("html_url"),
        "isPrerelease": True,
        "isDraft": False,
        "assets": [
            {"name": asset["name"]}
            for asset in assets if isinstance(asset, dict) and isinstance(asset.get("name"), str)
        ] if isinstance(assets, list) else None,
    }


def select_public_release(config: dict[str, Any], repository: str, requested: str) -> tuple[str, dict[str, Any]]:
    prefix = required_string(config, "tag_prefix")
    if requested != "latest":
        version = parse_version(requested)
        return version, public_release_for_tag(repository, f"{prefix}{version}")
    listed = public_json(f"https://api.github.com/repos/{repository}/releases?per_page=100")
    releases = [
        {"tagName": item.get("tag_name"), "isPrerelease": item.get("prerelease"), "isDraft": item.get("draft")}
        for item in listed if isinstance(item, dict)
    ] if isinstance(listed, list) else listed
    tag = latest_prerelease_tag(releases, prefix)
    return parse_version(tag.removeprefix(prefix)), public_release_for_tag(repository, tag)


def resolve_release(
    manifest: dict[str, Any], config: dict[str, Any], requested: str
) -> tuple[str, dict[str, Any], str | None]:
    """Select the release through gh, or through public GitHub URLs on a headless host.

    Returns the version, the release metadata, and the public download base URL
    (None when assets download through gh).
    """
    if gh_available():
        version, release = select_release(config, requested)
        return version, release, None
    repository = public_repository(manifest)
    version, release = select_public_release(config, repository, requested)
    tag = f"{required_string(config, 'tag_prefix')}{version}"
    return version, release, f"https://github.com/{repository}/releases/download/{tag}"


def release_asset_names(release: dict[str, Any]) -> set[str]:
    assets = release.get("assets")
    if not isinstance(assets, list):
        raise SystemExit("GitHub prerelease response has no assets")
    return {asset["name"] for asset in assets if isinstance(asset, dict) and isinstance(asset.get("name"), str)}


def require_release_assets(release: dict[str, Any], manifest: dict[str, Any], version: str) -> set[str]:
    """Require every manifest-declared asset; return them, excluding checksums.txt."""
    names = release_asset_names(release)
    missing = expected_assets(manifest, version) - names
    if missing:
        raise SystemExit(f"GitHub prerelease is missing required assets: {', '.join(sorted(missing))}")
    required = archive_assets(manifest, version) | require_python_assets(names, manifest, version)
    if not required:
        raise SystemExit("the manifest declares no prerelease assets: declare release_binaries or python_distributions")
    return required


def download_asset(tag: str, name: str, destination: Path, *, public_url: str | None = None) -> Path:
    destination.mkdir(parents=True, exist_ok=True)
    path = destination / name
    if public_url is None:
        command(["gh", "release", "download", tag, "--pattern", name, "--dir", str(destination)])
    else:
        request = Request(f"{public_url}/{name}", headers={"User-Agent": "sc-publish-prerelease"})
        with urlopen(request, timeout=PUBLIC_DOWNLOAD_TIMEOUT_SECONDS) as response:  # noqa: S310 - manifest-declared GitHub repository
            path.write_bytes(response.read())
    if not path.is_file():
        raise SystemExit(f"GitHub prerelease did not download {name}")
    return path


def safe_extract(archive: Path, destination: Path) -> None:
    root = destination.resolve()
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            names = bundle.namelist()
            if any(not (destination / name).resolve().is_relative_to(root) for name in names):
                raise SystemExit("prerelease archive contains an unsafe path")
            bundle.extractall(destination)
        return
    with tarfile.open(archive, mode="r:gz") as bundle:
        members = bundle.getmembers()
        if any(not (destination / member.name).resolve().is_relative_to(root) for member in members):
            raise SystemExit("prerelease archive contains an unsafe path")
        bundle.extractall(destination, members=members, filter="data")


def binary_name(binary: str) -> str:
    return f"{binary}.exe" if platform.system() == "Windows" else binary


def staged_binaries(stage: Path, binaries: Sequence[str]) -> list[Path]:
    paths = [stage / "bin" / binary_name(value) for value in binaries]
    if not all(path.is_file() for path in paths):
        raise SystemExit(f"staged prerelease binaries are incomplete under {stage}")
    return paths


def staged_wheels(stage: Path, manifest: dict[str, Any]) -> list[Path]:
    expected = sum(1 for distribution in python_distributions(manifest) if distribution.get("wheels"))
    wheels = sorted((stage / "wheels").glob("*.whl")) if expected else []
    if len(wheels) != expected:
        raise SystemExit(f"staged prerelease wheels are incomplete under {stage}")
    return wheels


def stage_release(
    tag: str,
    version: str,
    manifest: dict[str, Any],
    names: set[str],
    stage: Path,
    downloads: Path,
    *,
    public_url: str | None,
) -> None:
    """Download, checksum-verify and atomically stage the host binaries and wheels."""
    config = manifest["prerelease"]
    binaries = prerelease_binaries(config)
    digests = checksums(
        download_asset(tag, "checksums.txt", downloads, public_url=public_url).read_text(encoding="utf-8")
    )

    def checked(name: str) -> Path:
        expected = digests.get(name)
        if expected is None:
            raise SystemExit(f"checksums.txt has no sha256 for {name}")
        path = download_asset(tag, name, downloads, public_url=public_url)
        verify_checksum(path, expected)
        return path

    wheels = host_wheel_names(names, manifest, version)
    staging = stage.with_name(f".{stage.name}.download")
    shutil.rmtree(staging, ignore_errors=True)
    staging.mkdir(mode=0o700, parents=True)
    try:
        if binaries:
            archive = checked(archive_name(manifest, version, host_target(manifest)))
            extracted = staging / "extract"
            extracted.mkdir()
            safe_extract(archive, extracted)
            roots = [path for path in extracted.iterdir() if path.is_dir()]
            if len(roots) != 1:
                raise SystemExit("prerelease archive has an unexpected layout")
            root = roots[0]
            staged_binaries(root, binaries)
        else:
            root = staging / "root"
            root.mkdir()
        if wheels:
            wheel_dir = root / "wheels"
            wheel_dir.mkdir()
            for name in wheels:
                shutil.copy2(checked(name), wheel_dir / name)
        stage.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        shutil.rmtree(stage, ignore_errors=True)
        os.replace(root, stage)
    finally:
        shutil.rmtree(staging, ignore_errors=True)


def selector_directory(config: dict[str, Any]) -> Path:
    selectors = config.get("selector_dir")
    if not isinstance(selectors, dict):
        raise SystemExit("[prerelease].selector_dir must be a table")
    keys = {"Darwin": "darwin", "Linux": "linux", "Windows": "windows"}
    try:
        raw = selectors[keys[platform.system()]]
    except KeyError as error:
        raise SystemExit(f"[prerelease].selector_dir lacks {platform.system()} support") from error
    if not isinstance(raw, str) or not raw:
        raise SystemExit("prerelease selector directory must be a non-empty string")
    return Path(os.path.expandvars(raw)).expanduser()


def repoint_selector(paths: Sequence[Path], config: dict[str, Any]) -> None:
    if not paths:
        return
    selector = selector_directory(config)
    selector.mkdir(parents=True, exist_ok=True)
    for source in paths:
        target = selector / source.name
        if platform.system() == "Windows":
            shutil.copy2(source, target)
            continue
        if target.is_symlink() or target.exists():
            target.unlink()
        target.symlink_to(source)


def shell_quote(value: str) -> str:
    return subprocess.list2cmdline([value]) if os.name == "nt" else shlex.quote(value)


def replace_command(
    template: str, version: str, stage: Path, config: dict[str, Any], wheels: Sequence[Path] = ()
) -> str:
    """Expand the manifest command placeholders in one pass.

    {version} and {stage_dir} are substituted verbatim; {python}, {selector_dir}
    and {wheels} (the staged host wheels, space separated) are shell-quoted.
    """
    values: dict[str, Callable[[], str]] = {
        "version": lambda: version,
        "stage_dir": lambda: str(stage),
        "python": lambda: shell_quote(sys.executable),
        "selector_dir": lambda: shell_quote(str(selector_directory(config))),
        "wheels": lambda: " ".join(shell_quote(str(path)) for path in wheels),
    }
    return re.sub(
        r"\{(version|stage_dir|python|selector_dir|wheels)\}",
        lambda found: values[found.group(1)](),
        template,
    )


def verify_install(config: dict[str, Any], version: str, stage: Path, wheels: Sequence[Path] = ()) -> None:
    shell(replace_command(required_string(config, "post_install"), version, stage, config, wheels))
    verified = shell(replace_command(required_string(config, "verify"), version, stage, config, wheels), capture=True)
    if version not in f"{verified.stdout}\n{verified.stderr}":
        raise SystemExit(f"verify command did not report {version}")


def install(manifest: dict[str, Any], requested: str) -> tuple[str, Path]:
    config = manifest["prerelease"]
    assert isinstance(config, dict)
    binaries = prerelease_binaries(config)
    if not binaries and not any(distribution.get("wheels") for distribution in python_distributions(manifest)):
        raise SystemExit("[prerelease] installs nothing: declare binaries or python_distributions wheels")
    version, release, public_url = resolve_release(manifest, config, requested)
    root = Path(os.path.expandvars(required_string(config, "install_root"))).expanduser()
    stage = root / f"v{version}"
    if not stage.is_dir():
        names = release_asset_names(release)
        require_release_assets(release, manifest, version)
        tag = f"{required_string(config, 'tag_prefix')}{version}"
        with tempfile.TemporaryDirectory(prefix="prerelease-") as directory:
            stage_release(tag, version, manifest, names, stage, Path(directory), public_url=public_url)
    paths = staged_binaries(stage, binaries)
    wheels = staged_wheels(stage, manifest)
    repoint_selector(paths, config)
    verify_install(config, version, stage, wheels)
    return version, stage


def require_publish_preconditions(config: dict[str, Any]) -> None:
    branch = command(["git", "branch", "--show-current"], capture=True).stdout.strip()
    if not branch or branch in required_string_list(config, "protected_branches"):
        raise SystemExit("prerelease publishing refuses a protected or detached branch")
    command(["git", "diff", "--quiet"])
    command(["gh", "auth", "status"])


def wait_for_archive(tag: str, source_sha: str) -> None:
    started_at = time.monotonic()
    deadline = started_at + ARCHIVE_WAIT_SECONDS
    while time.monotonic() < deadline:
        elapsed_minutes = (time.monotonic() - started_at) / 60
        print(
            f"waiting for {ARCHIVE_WORKFLOW} ({elapsed_minutes:.1f} min elapsed)",
            flush=True,
        )
        runs = gh_json(["run", "list", "--workflow", ARCHIVE_WORKFLOW, "--branch", tag, "--limit", "20", "--json", "status,conclusion,headSha"])
        if isinstance(runs, list):
            run = next(
                (
                    candidate
                    for candidate in runs
                    if isinstance(candidate, dict)
                    and candidate.get("headSha") == source_sha
                ),
                None,
            )
            if run is not None and run.get("status") == "completed":
                if run.get("conclusion") == "success":
                    return
                raise SystemExit(f"{ARCHIVE_WORKFLOW} failed for {tag}")
        remaining = deadline - time.monotonic()
        if remaining > 0:
            time.sleep(min(ARCHIVE_POLL_SECONDS, remaining))
    elapsed_minutes = (time.monotonic() - started_at) / 60
    raise SystemExit(
        f"timed out waiting for {ARCHIVE_WORKFLOW} for {tag} after {elapsed_minutes:.1f} minutes"
    )


def publish(manifest: dict[str, Any]) -> tuple[str, str]:
    config = manifest["prerelease"]
    assert isinstance(config, dict)
    require_publish_preconditions(config)
    result = command([sys.executable, required_string(config, "tag_script")], capture=True)
    tag = next((line.split()[3] for line in result.stdout.splitlines() if line.startswith("created and pushed ")), "")
    prefix = required_string(config, "tag_prefix")
    if not tag.startswith(prefix):
        raise SystemExit("prerelease tag script did not report a prerelease tag")
    version = parse_version(tag.removeprefix(prefix))
    source_sha = command(["git", "rev-list", "-n", "1", tag], capture=True).stdout.strip()
    if not source_sha:
        raise SystemExit(f"cannot resolve the commit for {tag}")
    wait_for_archive(tag, source_sha)
    release = release_for_tag(tag)
    required = require_release_assets(release, manifest, version)
    with tempfile.TemporaryDirectory(prefix="prerelease-verify-") as directory:
        checksums_file = download_asset(tag, "checksums.txt", Path(directory))
        digest_by_name = checksums(checksums_file.read_text(encoding="utf-8"))
        uncovered = release_asset_names(release) - {"checksums.txt"} - set(digest_by_name)
        if uncovered:
            raise SystemExit(f"checksums.txt does not cover Release assets: {', '.join(sorted(uncovered))}")
        for asset in sorted(required):
            expected = digest_by_name.get(asset)
            if expected is None:
                raise SystemExit(f"checksums.txt has no sha256 for {asset}")
            verify_checksum(download_asset(tag, asset, Path(directory)), expected)
    url = release.get("url")
    if not isinstance(url, str):
        raise SystemExit("GitHub prerelease response has no URL")
    return tag, url


def asset_plan(manifest: dict[str, Any]) -> str:
    """Describe, without network access, the asset set a prerelease must carry."""
    archives = len(release_targets(manifest)) if release_binaries(manifest) else 0
    distributions = python_distributions(manifest)
    wheels = sum(len(distribution.get("wheels", [])) for distribution in distributions)
    sdists = sum(1 for distribution in distributions if distribution.get("sdist"))
    return (
        f"would verify {archives} binary archive(s), {wheels} wheel(s), "
        f"{sdists} sdist(s), and checksums.txt"
    )


def main(argv: Sequence[str] | None = None) -> int:
    require_supported_python()
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--publish",
        "--create",
        dest="publish",
        action="store_true",
        help="publish a new manifest-selected prerelease (--create is a compatibility alias)",
    )
    mode.add_argument(
        "--install",
        nargs="?",
        const="latest",
        metavar="X.Y.Z",
        help="install X.Y.Z, or the latest matching GitHub prerelease when omitted",
    )
    parser.add_argument("--manifest", type=Path, default=Path("release/publish-artifacts.toml"))
    parser.add_argument("--dry-run", action="store_true", help="print the plan without network calls")
    parser.add_argument("--authorized", action="store_true", help="confirm written operator authorization to publish")
    args = parser.parse_args(argv)
    manifest = read_manifest(args.manifest)
    config = prerelease_manifest(args.manifest)
    if args.publish:
        if args.dry_run:
            plan = command(
                [sys.executable, required_string(config, "tag_script"), "--dry-run"],
                capture=True,
            )
            print(plan.stdout, end="" if plan.stdout.endswith("\n") else "\n")
            print(f"would wait for {ARCHIVE_WORKFLOW}, then verify Release assets and checksums")
            print(asset_plan(manifest))
            return 0
        if not args.authorized:
            raise SystemExit("--publish requires written operator authorization")
        _tag, url = publish(manifest)
        print(f"Release URL: {url}")
        return 0
    requested = args.install or "latest"
    if requested != "latest":
        parse_version(requested)
    if args.dry_run:
        print(f"would install GitHub prerelease {requested} without network access")
        return 0
    version, stage = install(manifest, requested)
    print(f"installed prerelease {version} at {stage}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
