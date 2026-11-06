#!/usr/bin/env python3
"""Verify retained install-matrix evidence for one release candidate (FR-25, ADR-0025).

The verifier is fail-closed and offline. It reads only:

- the canonical contracts: ``release/publish-artifacts.toml`` (``[project]``,
  ``[[release_targets]]``, ``[[python_distributions]]``) and the Go native
  module config plus its ``native/targets.toml``, parsed by the existing
  ``.github/scripts/go_native_module.py`` helpers;
- the trusted run, as GitHub returns it: the workflow run object
  (``GET /repos/{repo}/actions/runs/{id}``) and its artifact listing
  (``GET /repos/{repo}/actions/runs/{id}/artifacts``);
- that run's ``checksums.txt`` (``sha256sum`` output, as release.yml writes it);
- the publisher staging directory holding the files to publish.

It requires 30 cells: five artifact families (Rust CLI, the sc-compose,
sc-sha and sc-composer-beads wheels, the sc-sha-go native module) on six
OS/architecture coordinates (macOS, Windows, Linux on x64 and arm64). A
coordinate counts once however many toolchain variants a contract declares
for it (Windows x64 msvc and gnu are one coordinate), and a supplemental
variant never stands in for a missing coordinate.

Every check fails closed: a wrong, missing, stale or expired run, a wrong
candidate head or version, a missing, extra or changed file, and a digest
mismatch are all errors. The clock and the trust inputs (repository, workflow
path, candidate head and version) are explicit arguments, so the check is
deterministic for fixed inputs. This verifies evidence only; it does not
build, install, stage or publish anything.

Exit status: 0 when every check passes, 1 when evidence is rejected, 2 when
an input cannot be read.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import zipfile
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / ".github" / "scripts"))

from go_native_module import (  # noqa: E402 - canonical contract parsers
    CONFIG_FIELDS,
    ContractError,
    _read_toml,
    _release_targets,
    _safe_relative,
    _target_entries,
)

OS_NAMES = ("linux", "macos", "windows")
ARCH_NAMES = ("x64", "arm64")
REQUIRED_COORDINATES = tuple((os_name, arch) for os_name in OS_NAMES for arch in ARCH_NAMES)
RUST_FAMILY = "rust-cli"
GO_FAMILY = "sc-sha-go"
WHEEL_FAMILIES = {"sc-compose": "sc-compose-wheel", "sc-sha": "sc-sha-wheel", "sc-composer-beads": "sc-composer-beads-wheel"}
FAMILIES = (RUST_FAMILY, *WHEEL_FAMILIES.values(), GO_FAMILY)
REQUIRED_CELLS = len(FAMILIES) * len(REQUIRED_COORDINATES)
DEFAULT_WORKFLOW = ".github/workflows/release-preflight.yml"
DISPATCH_EVENT = "workflow_dispatch"  # FR-25.4: the full matrix runs only on manual dispatch
WHEEL_ABI = "abi3"
WHEEL_PYTHON = "cp39"
WINDOWS_ARM64_PYTHON = "cp311"  # FR-25.2: the one recorded platform exception
CHECKSUM_LINE = re.compile(r"^([0-9a-f]{64}) [ *](\S(?:.*\S)?)$")
SEMVER = re.compile(r"^(\d+\.\d+\.\d+)(?:-(alpha|beta|rc)\.?(\d+))?$")
PEP440_PRE = {"alpha": "a", "beta": "b", "rc": "rc"}


class EvidenceInputError(ValueError):
    """An input file is missing or unreadable (exit 2, never a pass)."""


@dataclass
class Report:
    errors: list[dict[str, str]] = field(default_factory=list)
    cells: dict[tuple[str, str, str], list[str]] = field(default_factory=dict)

    def fail(self, code: str, message: str) -> None:
        self.errors.append({"code": code, "message": message})

    def cover(self, family: str, coordinate: tuple[str, str], name: str) -> None:
        self.cells.setdefault((family, *coordinate), []).append(name)


@dataclass(frozen=True)
class Contracts:
    archive_prefix: str
    rust_targets: dict[str, dict[str, str]]
    wheel_names: dict[str, str]
    go_prefix: str
    go_targets: dict[str, tuple[str, str]]


# --------------------------------------------------------------------------- #
# canonical contracts
# --------------------------------------------------------------------------- #
def rust_coordinate(triple: str) -> tuple[str, str] | None:
    arch = {"x86_64": "x64", "aarch64": "arm64"}.get(triple.split("-", 1)[0])
    if "-linux-" in triple:
        os_name = "linux"
    elif triple.endswith("-apple-darwin"):
        os_name = "macos"
    elif "-windows-" in triple:
        os_name = "windows"
    else:
        os_name = None
    return (os_name, arch) if os_name and arch else None


def go_coordinate(goos: str, goarch: str) -> tuple[str, str] | None:
    os_name = {"linux": "linux", "darwin": "macos", "windows": "windows"}.get(goos)
    arch = {"amd64": "x64", "arm64": "arm64"}.get(goarch)
    return (os_name, arch) if os_name and arch else None


def wheel_coordinates(platform_tag: str) -> set[tuple[str, str]]:
    found: set[tuple[str, str]] = set()
    for tag in platform_tag.split("."):
        if tag == "win_amd64":
            found.add(("windows", "x64"))
        elif tag == "win_arm64":
            found.add(("windows", "arm64"))
        elif re.fullmatch(r"(?:many|musl)linux_\d+_\d+_(x86_64|aarch64)|manylinux\d+_(x86_64|aarch64)", tag):
            found.add(("linux", "x64" if tag.endswith("x86_64") else "arm64"))
        elif match := re.fullmatch(r"macosx_\d+_\d+_(x86_64|arm64|universal2)", tag):
            arches = ARCH_NAMES if match[1] == "universal2" else ("x64" if match[1] == "x86_64" else "arm64",)
            found.update(("macos", arch) for arch in arches)
    return found


def load_contracts(manifest_path: Path, go_config_path: Path) -> Contracts:
    """Read the canonical contracts; ContractError when they cannot be read."""
    manifest = _read_toml(manifest_path, "release artifact manifest")
    project = manifest.get("project")
    if not isinstance(project, dict) or not isinstance(project.get("archive_prefix"), str) or not project["archive_prefix"]:
        raise ContractError("release artifact manifest must define [project].archive_prefix")
    wheel_names: dict[str, str] = {}
    for distribution in manifest.get("python_distributions") or []:
        name = distribution.get("name") if isinstance(distribution, dict) else None
        if name in WHEEL_FAMILIES:
            wheel_names[name] = WHEEL_FAMILIES[name]
    go_config = _read_toml(go_config_path, "go native module config")
    if set(go_config) != CONFIG_FIELDS or go_config.get("schema_version") != 1:
        raise ContractError("go native module config must contain exactly the v1 schema fields")
    source = go_config_path.resolve().parent.parent / _safe_relative(str(go_config.get("source", "")), "source")
    _, go_entries = _target_entries(source)
    return Contracts(
        archive_prefix=project["archive_prefix"],
        rust_targets=_release_targets(manifest_path),
        wheel_names=wheel_names,
        go_prefix=str(go_config["artifact_prefix"]),
        go_targets={entry["target"]: (entry["goos"], entry["goarch"]) for entry in go_entries},
    )


def declared_coordinates(contracts: Contracts) -> dict[str, set[tuple[str, str]]]:
    declared = {family: set() for family in FAMILIES}
    declared[RUST_FAMILY] = {c for t in contracts.rust_targets for c in [rust_coordinate(t)] if c}
    declared[GO_FAMILY] = {c for goos, goarch in contracts.go_targets.values() for c in [go_coordinate(goos, goarch)] if c}
    for family in contracts.wheel_names.values():
        declared[family] = set(REQUIRED_COORDINATES)  # wheel platforms are fixed by FR-25.1, not the legacy runner list
    return declared


# --------------------------------------------------------------------------- #
# evidence
# --------------------------------------------------------------------------- #
def parse_time(value: object) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    return parsed if parsed.tzinfo else None


def pep440(version: str) -> str | None:
    match = SEMVER.fullmatch(version)
    if not match:
        return None
    return match[1] + (f"{PEP440_PRE[match[2]]}{match[3]}" if match[2] else "")


def check_run(report: Report, run: Any, artifacts: Any, *, repository: str, workflow: str, head: str, now: datetime) -> None:
    if not isinstance(run, dict) or not run.get("id"):
        report.fail("RUN_MISSING", "no trusted workflow run was supplied")
        return
    run_id = run["id"]
    full_name = (run.get("repository") or {}).get("full_name") if isinstance(run.get("repository"), dict) else None
    if full_name != repository:
        report.fail("RUN_WRONG", f"run {run_id} belongs to {full_name!r}, not {repository!r}")
    if str(run.get("path", "")).split("@", 1)[0] != workflow:
        report.fail("RUN_WRONG", f"run {run_id} is workflow {run.get('path')!r}, not {workflow!r}")
    if run.get("event") != DISPATCH_EVENT:
        report.fail("RUN_WRONG", f"run {run_id} was triggered by {run.get('event')!r}; the matrix runs only on {DISPATCH_EVENT}")
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        report.fail("RUN_WRONG", f"run {run_id} is {run.get('status')}/{run.get('conclusion')}, not completed/success")
    if run.get("head_sha") != head:
        report.fail("RUN_STALE", f"run {run_id} tested {run.get('head_sha')!r}, not candidate {head!r}")
    finished = parse_time(run.get("updated_at"))
    if finished is None or finished > now:
        report.fail("RUN_WRONG", f"run {run_id} has no valid completion time not later than the check time")
    listing = artifacts.get("artifacts") if isinstance(artifacts, dict) else None
    if not isinstance(listing, list) or not listing:
        report.fail("EVIDENCE_EXPIRED", f"run {run_id} has no retained artifacts")
        return
    for artifact in listing:
        name = artifact.get("name") if isinstance(artifact, dict) else None
        owner = artifact.get("workflow_run") if isinstance(artifact, dict) else None
        if not isinstance(owner, dict) or owner.get("id") != run_id or owner.get("head_sha") != head:
            report.fail("ARTIFACT_WRONG_RUN", f"artifact {name!r} was not produced by run {run_id} at the candidate head")
        expires = parse_time(artifact.get("expires_at")) if isinstance(artifact, dict) else None
        if not isinstance(artifact, dict) or artifact.get("expired") is not False or expires is None or expires <= now:
            report.fail("EVIDENCE_EXPIRED", f"artifact {name!r} of run {run_id} is expired or has no valid expiry")


def parse_checksums(report: Report, text: str) -> dict[str, str]:
    digests: dict[str, str] = {}
    for number, line in enumerate(text.splitlines(), start=1):
        if not line.strip():
            continue
        match = CHECKSUM_LINE.fullmatch(line)
        if not match or "/" in match[2] or "\\" in match[2]:
            report.fail("CHECKSUMS_INVALID", f"checksums.txt line {number} is not '<sha256>  <file name>'")
            continue
        if match[2] in digests:
            report.fail("CHECKSUMS_INVALID", f"checksums.txt lists {match[2]} more than once")
        digests[match[2]] = match[1]
    if not digests:
        report.fail("CHECKSUMS_INVALID", "checksums.txt lists no files")
    return digests


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def go_module_version(path: Path) -> str | None:
    try:
        with zipfile.ZipFile(path) as archive:
            members = [n for n in archive.namelist() if n == "VERSION" or (n.endswith("/VERSION") and n.count("/") == 1)]
            if len(members) != 1:
                return None
            return archive.read(members[0]).decode("utf-8").strip()
    except (OSError, zipfile.BadZipFile, UnicodeDecodeError):
        return None


def classify(report: Report, contracts: Contracts, name: str, path: Path, version: str) -> None:
    """Map one trusted, staged file to its cell(s); version and tag errors are recorded."""
    wheel_version = pep440(version)
    for target, entry in contracts.rust_targets.items():
        if name == f"{contracts.archive_prefix}_{version}_{target}.{entry['archive']}":
            report.cover(RUST_FAMILY, rust_coordinate(target) or ("?", "?"), name)
            return
        if name.startswith(f"{contracts.archive_prefix}_") and name.endswith(f"_{target}.{entry['archive']}"):
            report.fail("VERSION_MISMATCH", f"{name} is not version {version}")
            return
    for target, (goos, goarch) in contracts.go_targets.items():
        if name == f"{contracts.go_prefix}-{target}.zip":
            found = go_module_version(path)
            if found != version:
                report.fail("VERSION_MISMATCH", f"{name} carries VERSION {found!r}, not {version}")
                return
            report.cover(GO_FAMILY, go_coordinate(goos, goarch) or ("?", "?"), name)
            return
    for dist, family in contracts.wheel_names.items():
        stem = dist.replace("-", "_")
        if name.endswith(".whl") and name.startswith(f"{stem}-"):
            parts = name[: -len(".whl")].split("-")
            if len(parts) != 5:
                report.fail("WHEEL_TAG_INVALID", f"{name} is not a {{name}}-{{version}}-{{python}}-{{abi}}-{{platform}} wheel")
                return
            _, found_version, python_tag, abi_tag, platform_tag = parts
            if found_version != wheel_version:
                report.fail("VERSION_MISMATCH", f"{name} is not version {wheel_version}")
                return
            coordinates = wheel_coordinates(platform_tag)
            allowed = {WHEEL_PYTHON, WINDOWS_ARM64_PYTHON} if coordinates == {("windows", "arm64")} else {WHEEL_PYTHON}
            if not coordinates or abi_tag != WHEEL_ABI or python_tag not in allowed:
                report.fail("WHEEL_TAG_INVALID", f"{name} is not a {WHEEL_PYTHON}-{WHEEL_ABI} wheel for a required platform")
                return
            for coordinate in coordinates:
                report.cover(family, coordinate, name)
            return
        if re.fullmatch(rf"{re.escape(stem)}-(.+)\.tar\.gz", name) or re.fullmatch(rf"{re.escape(dist)}-(.+)\.tar\.gz", name):
            if not name.endswith(f"-{wheel_version}.tar.gz"):
                report.fail("VERSION_MISMATCH", f"{name} is not version {wheel_version}")
            return  # an sdist is a release file, never a platform cell
    report.fail("FILE_UNRECOGNIZED", f"{name} matches no artifact family in the canonical contracts")


def verify(
    *,
    contracts: Contracts,
    run: Any,
    artifacts: Any,
    checksums_text: str,
    staging: Path,
    repository: str,
    workflow: str,
    head: str,
    version: str,
    now: datetime,
) -> dict[str, Any]:
    """Return the verification result; ``ok`` is true only when nothing failed."""
    report = Report()
    if not re.fullmatch(r"[0-9a-f]{40}", head):
        report.fail("CANDIDATE_INVALID", f"candidate head {head!r} is not a full commit SHA")
    if pep440(version) is None:
        report.fail("CANDIDATE_INVALID", f"candidate version {version!r} is not X.Y.Z or X.Y.Z-(alpha|beta|rc).N")
    check_run(report, run, artifacts, repository=repository, workflow=workflow, head=head, now=now)
    trusted = parse_checksums(report, checksums_text)
    staged = {p.name: p for p in sorted(staging.iterdir()) if p.is_file()} if staging.is_dir() else {}
    if not staging.is_dir():
        report.fail("FILE_MISSING", f"staging directory {staging} does not exist")
    for name in sorted(set(staged) - set(trusted)):
        report.fail("FILE_UNEXPECTED", f"staged {name} is not in the run's checksums (rebuilt or added after the gate)")
    for name, expected in sorted(trusted.items()):
        if name not in staged:
            report.fail("FILE_MISSING", f"{name} from the run's checksums is not staged")
            continue
        actual = sha256(staged[name])
        if actual != expected:
            report.fail("DIGEST_MISMATCH", f"staged {name} is {actual}, the run recorded {expected}")
            continue
        if pep440(version) is not None:
            classify(report, contracts, name, staged[name], version)
    declared = declared_coordinates(contracts)
    for family in FAMILIES:
        for coordinate in REQUIRED_COORDINATES:
            if coordinate not in declared[family]:
                report.fail("CONTRACT_MISSING_COORDINATE", f"{family} declares no {coordinate[0]}/{coordinate[1]} target")
            elif (family, *coordinate) not in report.cells:
                report.fail("CELL_MISSING", f"no verified {family} file for {coordinate[0]}/{coordinate[1]}")
            elif len(report.cells[(family, *coordinate)]) > 1 and family != RUST_FAMILY:
                report.fail("CELL_DUPLICATE", f"{family} {coordinate[0]}/{coordinate[1]} has several files: {report.cells[(family, *coordinate)]}")
    for target in contracts.rust_targets:
        name = f"{contracts.archive_prefix}_{version}_{target}.{contracts.rust_targets[target]['archive']}"
        if name not in trusted:  # a declared supplemental toolchain is still a release file
            report.fail("FILE_MISSING", f"declared release target {target} has no {name} in the run's checksums")
    verified = sum(1 for family in FAMILIES for c in REQUIRED_COORDINATES if (family, *c) in report.cells)
    return {
        "ok": not report.errors,
        "candidate": {"head_sha": head, "version": version},
        "run_id": run.get("id") if isinstance(run, dict) else None,
        "required_cells": REQUIRED_CELLS,
        "verified_cells": verified,
        "cells": [{"family": f, "os": o, "arch": a, "files": sorted(files)} for (f, o, a), files in sorted(report.cells.items())],
        "errors": report.errors,
    }


# --------------------------------------------------------------------------- #
# command line
# --------------------------------------------------------------------------- #
def read_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceInputError(f"cannot read {label} {path}: {error}") from error


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("--manifest", type=Path, default=Path("release/publish-artifacts.toml"))
    parser.add_argument("--go-config", type=Path, default=Path("release/go-native-module.toml"))
    parser.add_argument("--run", type=Path, required=True, help="workflow run JSON (GET .../actions/runs/{id})")
    parser.add_argument("--artifacts", type=Path, required=True, help="run artifact listing JSON (GET .../actions/runs/{id}/artifacts)")
    parser.add_argument("--checksums", type=Path, required=True, help="the run's checksums.txt (sha256sum format)")
    parser.add_argument("--staging", type=Path, required=True, help="publisher staging directory")
    parser.add_argument("--repository", required=True, help="trusted OWNER/REPO the run must belong to")
    parser.add_argument("--workflow", default=DEFAULT_WORKFLOW, help="trusted workflow path the run must come from")
    parser.add_argument("--candidate-sha", required=True, help="full commit SHA of the release candidate")
    parser.add_argument("--version", required=True, help="release version, X.Y.Z or X.Y.Z-rc.N")
    parser.add_argument("--now", help="check time, ISO 8601 with offset (default: the current UTC time)")
    args = parser.parse_args(argv)
    try:
        now = parse_time(args.now) if args.now else datetime.now(timezone.utc)
        if now is None:
            raise EvidenceInputError("--now must be ISO 8601 with a UTC offset")
        try:
            checksums_text = args.checksums.read_text(encoding="utf-8")
        except OSError as error:
            raise EvidenceInputError(f"cannot read checksums {args.checksums}: {error}") from error
        result = verify(
            contracts=load_contracts(args.manifest, args.go_config),
            run=read_json(args.run, "run"),
            artifacts=read_json(args.artifacts, "artifact listing"),
            checksums_text=checksums_text,
            staging=args.staging,
            repository=args.repository,
            workflow=args.workflow,
            head=args.candidate_sha,
            version=args.version,
            now=now,
        )
    except (EvidenceInputError, ContractError) as error:
        print(json.dumps({"ok": False, "errors": [{"code": "INPUT_UNREADABLE", "message": str(error)}]}))
        return 2
    print(json.dumps(result, indent=2))
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
