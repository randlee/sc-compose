#!/usr/bin/env python3
"""Materialize the pinned, sc-lint-owned Python helpers for local ``just lint``.

CI obtains the same helper bundle through ``setup-sc-lint``. Keeping the
helpers generated avoids a second, drifting copy of sc-lint's Python checks in
each consumer repository while making the local ``just lint`` contract usable.

This is a temporary, fail-closed dependency on the pinned sc-lint source
archive: a network or upstream-tag failure prevents local lint from running.
``SC_LINT_SOURCE_ROOT`` supports an offline checked-out source tree. Long-term
packaging is tracked by sc-lint issue 86; this consumer must not vendor helpers.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import tarfile
import tempfile
from urllib.request import urlretrieve


SC_LINT_VERSION = "0.4.0"
REQUIRED_HELPERS = ("run_lint.py", "check_version_sync.py", "lint_common.py")


def _safe_extract(archive: tarfile.TarFile, destination: Path) -> None:
    destination = destination.resolve()
    for member in archive.getmembers():
        candidate = (destination / member.name).resolve()
        if not candidate.is_relative_to(destination):
            raise SystemExit(f"refusing unsafe sc-lint source archive member: {member.name}")
    archive.extractall(destination, filter="data")


def _helper_root(source_root: Path) -> Path:
    helpers = source_root / ".just"
    if not helpers.is_dir():
        raise SystemExit(f"sc-lint source is missing helper directory: {helpers}")
    missing = [name for name in REQUIRED_HELPERS if not (helpers / name).is_file()]
    if missing:
        raise SystemExit("sc-lint source is missing required helper(s): " + ", ".join(missing))
    return helpers


def _download_source(version: str) -> Path:
    url = f"https://github.com/randlee/sc-lint/archive/refs/tags/v{version}.tar.gz"
    temporary = Path(tempfile.mkdtemp(prefix="sc-lint-source-"))
    archive_path = temporary / "source.tar.gz"
    try:
        urlretrieve(url, archive_path)
        with tarfile.open(archive_path, "r:gz") as archive:
            _safe_extract(archive, temporary)
    except Exception:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    roots = [path for path in temporary.iterdir() if path.is_dir()]
    if len(roots) != 1:
        shutil.rmtree(temporary, ignore_errors=True)
        raise SystemExit(f"sc-lint source archive for v{version} has an unexpected layout")
    return roots[0]


def _runtime_is_current(destination: Path, version: str) -> bool:
    marker = destination / ".sc-lint-runtime-version"
    return (
        marker.is_file()
        and marker.read_text(encoding="utf-8").strip() == version
        and all((destination / name).is_file() for name in REQUIRED_HELPERS)
    )


def materialize(repo_root: Path, version: str, source_root: Path | None) -> None:
    destination = repo_root / ".just"
    if _runtime_is_current(destination, version):
        print(f"sc-lint helpers v{version} already materialized")
        return

    cleanup_root: Path | None = None
    if source_root is None:
        cleanup_root = _download_source(version)
        source_root = cleanup_root
    try:
        helpers = _helper_root(source_root)
        destination.mkdir(parents=True, exist_ok=True)
        for helper in helpers.glob("*.py"):
            shutil.copy2(helper, destination / helper.name)
        config = helpers / "lint-config.toml"
        if config.is_file():
            shutil.copy2(config, destination / config.name)
        (destination / ".sc-lint-runtime-version").write_text(f"{version}\n", encoding="utf-8")
    finally:
        if cleanup_root is not None:
            shutil.rmtree(cleanup_root.parent, ignore_errors=True)
    print(f"materialized sc-lint helpers v{version}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--version", default=os.environ.get("SC_LINT_VERSION", SC_LINT_VERSION))
    parser.add_argument(
        "--source-root",
        type=Path,
        default=Path(os.environ["SC_LINT_SOURCE_ROOT"])
        if "SC_LINT_SOURCE_ROOT" in os.environ
        else None,
    )
    args = parser.parse_args()
    materialize(args.root.resolve(), args.version, args.source_root.resolve() if args.source_root else None)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
