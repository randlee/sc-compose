#!/usr/bin/env python3
"""Select, commit and tag one immutable GitHub prerelease for sc-compose.

This is the consumer-owned `[prerelease].tag_script` required by the sc-publish
prerelease skill. It never takes a version argument. Policy:

* The workspace version (`Cargo.toml` `[workspace.package]`) is used as-is only
  when it has never been published: no `prerelease/vX.Y.Z` or production
  `vX.Y.Z` tag exists locally or on `origin`, and no declared crate is
  published at that version on its registry.
* Otherwise the PATCH number alone is incremented until a never-used version is
  found. Minor and major numbers are never changed, so a chosen production
  `1.8.0` can only become `1.8.N`, never `1.9.0`. The bump is applied to every
  lockstep version file and committed on the current non-protected branch.
* A tag is only ever created, never moved, deleted or overwritten, and no force
  push is used. Any failed lookup stops the run (fail closed).

`--dry-run` is local-only: it runs read-only git commands, makes no network
call and mutates nothing. Because it cannot see `origin` or the registries it
says so, and the real run re-checks both before changing anything.

A real run prints `created and pushed prerelease/vX.Y.Z from <commit>`, the
line the prerelease skill parses.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Callable, Sequence

MANIFEST = Path("release/publish-artifacts.toml")
WORKSPACE_TOML = Path("Cargo.toml")
PYPROJECTS = (
    Path("bindings/sc-sha-python/pyproject.toml"),
    Path("bindings/python/pyproject.toml"),
    Path("bindings/sc-composer-beads-python/pyproject.toml"),
)
CARGO_LOCK = Path("Cargo.lock")
SEMVER = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
MAX_PATCH_STEPS = 100


class TagError(SystemExit):
    """A fail-closed refusal with a stable code."""

    def __init__(self, code: str, detail: str) -> None:
        super().__init__(f"PRERELEASE_TAG.{code}: {detail}")


Runner = Callable[[Sequence[str]], str]
RegistryCheck = Callable[[str], bool]


def run_git(args: Sequence[str], cwd: Path) -> str:
    result = subprocess.run(
        ["git", *args], cwd=cwd, check=False, capture_output=True, text=True, timeout=120
    )
    if result.returncode != 0:
        raise TagError("GIT_FAILED", f"git {' '.join(args)}: {result.stderr.strip() or result.stdout.strip()}")
    return result.stdout


def parse_version(value: str) -> tuple[int, int, int]:
    match = SEMVER.match(value)
    if match is None:
        raise TagError("VERSION_INVALID", f"{value!r} is not a plain X.Y.Z release version")
    return int(match[1]), int(match[2]), int(match[3])


def workspace_version(root: Path) -> str:
    data = tomllib.loads((root / WORKSPACE_TOML).read_text(encoding="utf-8"))
    version = data.get("workspace", {}).get("package", {}).get("version")
    if not isinstance(version, str):
        raise TagError("VERSION_INVALID", "Cargo.toml has no [workspace.package].version")
    parse_version(version)
    return version


def prerelease_config(root: Path) -> dict[str, object]:
    manifest = tomllib.loads((root / MANIFEST).read_text(encoding="utf-8"))
    config = manifest.get("prerelease")
    if not isinstance(config, dict):
        raise TagError("MANIFEST_INVALID", f"{MANIFEST} has no [prerelease] table")
    prefix = config.get("tag_prefix")
    protected = config.get("protected_branches")
    if not isinstance(prefix, str) or not prefix:
        raise TagError("MANIFEST_INVALID", "[prerelease].tag_prefix must be a non-empty string")
    if not isinstance(protected, list) or not protected or not all(isinstance(b, str) for b in protected):
        raise TagError("MANIFEST_INVALID", "[prerelease].protected_branches must be a non-empty string list")
    return config


def check_branch(root: Path, protected: Sequence[str]) -> str:
    branch = run_git(["branch", "--show-current"], root).strip()
    if not branch:
        raise TagError("DETACHED_HEAD", "a prerelease needs an attached branch")
    if branch in protected:
        raise TagError("PROTECTED_BRANCH", f"{branch!r} is protected ({', '.join(protected)}); use a feature branch")
    if run_git(["status", "--porcelain", "--untracked-files=no"], root).strip():
        raise TagError("DIRTY_TREE", "tracked files have uncommitted changes")
    return branch


def local_tags(root: Path) -> set[str]:
    return set(run_git(["tag", "--list"], root).split())


def remote_tags(root: Path) -> set[str]:
    out = run_git(["ls-remote", "--tags", "--refs", "origin"], root)
    return {line.split("refs/tags/", 1)[1] for line in out.splitlines() if "refs/tags/" in line}


def bump_patch(version: str) -> str:
    major, minor, patch = parse_version(version)
    return f"{major}.{minor}.{patch + 1}"


def select_version(
    base: str,
    prefix: str,
    tags: set[str],
    registry_published: RegistryCheck | None,
) -> str:
    """Return the first never-used version at or above `base`, patch bumps only."""
    candidate = base
    for _ in range(MAX_PATCH_STEPS):
        used = {f"{prefix}{candidate}", f"v{candidate}"} & tags
        if not used and (registry_published is None or not registry_published(candidate)):
            return candidate
        candidate = bump_patch(candidate)
    raise TagError("NO_FREE_VERSION", f"no unused patch version within {MAX_PATCH_STEPS} steps of {base}")


def replace_exactly(path: Path, pattern: str, replacement: str, expected: int) -> None:
    text = path.read_text(encoding="utf-8")
    new, count = re.subn(pattern, replacement, text, flags=re.MULTILINE)
    if count != expected:
        raise TagError("LOCKSTEP_EDIT", f"{path}: expected {expected} version site(s), found {count}")
    path.write_text(new, encoding="utf-8")


def apply_patch_bump(root: Path, old: str, new: str) -> list[Path]:
    """Rewrite every lockstep version site from `old` to `new` and return the files."""
    old_re = re.escape(old)
    replace_exactly(root / WORKSPACE_TOML, rf'^version = "{old_re}"$', f'version = "{new}"', 1)
    workspace = tomllib.loads((root / WORKSPACE_TOML).read_text(encoding="utf-8"))
    pins = [
        name
        for name, spec in workspace.get("workspace", {}).get("dependencies", {}).items()
        if isinstance(spec, dict) and "path" in spec and spec.get("version") == old
    ]
    for name in pins:
        replace_exactly(
            root / WORKSPACE_TOML,
            rf'^({re.escape(name)} = \{{ path = "[^"]+", version = "){old_re}(" \}})$',
            rf"\g<1>{new}\g<2>",
            1,
        )
    for pyproject in PYPROJECTS:
        replace_exactly(root / pyproject, rf'^version = "{old_re}"$', f'version = "{new}"', 1)
    lock = (root / CARGO_LOCK).read_text(encoding="utf-8")
    blocks = lock.split("\n[[package]]\n")
    changed = 0
    for index, block in enumerate(blocks):
        if f'version = "{old}"' in block.split("\n") and "\nsource = " not in f"\n{block}":
            blocks[index] = block.replace(f'version = "{old}"', f'version = "{new}"', 1)
            changed += 1
    if changed == 0:
        raise TagError("LOCKSTEP_EDIT", f"{CARGO_LOCK}: no workspace package at {old}")
    (root / CARGO_LOCK).write_text("\n[[package]]\n".join(blocks), encoding="utf-8")
    return [WORKSPACE_TOML, *PYPROJECTS, CARGO_LOCK]


def registry_published_default(root: Path) -> RegistryCheck:
    """Network lookup of every declared crate at a version; used by real runs only."""
    sys.path.insert(0, str(root / ".github" / "scripts"))
    from release_manifest import check_version_publication  # noqa: PLC0415

    def published(version: str) -> bool:
        unexpected, preserved = check_version_publication(root / MANIFEST, version, "")
        return bool(unexpected or preserved)

    return published


def plan_lines(base: str, chosen: str, prefix: str, branch: str, remote_checked: bool) -> list[str]:
    lines = [f"branch: {branch} (attached, not protected, tracked tree clean)"]
    lines.append(f"workspace version: {base}")
    if chosen == base:
        lines.append(f"would create tag {prefix}{chosen}: version {base} is unused in the checked scope")
    else:
        lines.append(
            f"would PATCH-BUMP {base} -> {chosen} (patch only; major and minor stay) in Cargo.toml, "
            "the 3 pyproject.toml files and Cargo.lock, commit it on this branch, then tag "
            f"{prefix}{chosen}"
        )
        lines.append(
            f"note: production release version {base} is unchanged on protected branches; "
            f"do not merge the {chosen} bump commit into develop or main"
        )
    if not remote_checked:
        lines.append(
            "dry-run is local-only: origin tags and registry versions were NOT checked; "
            "the real run checks both first and refuses on any lookup failure or collision"
        )
    return lines


def main(
    argv: Sequence[str] | None = None,
    *,
    root: Path | None = None,
    registry_published: RegistryCheck | None = None,
    remote_tag_lister: Callable[[Path], set[str]] | None = None,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dry-run", action="store_true", help="print the plan; local-only, no network, no mutation")
    args = parser.parse_args(argv)
    root = (root or Path.cwd()).resolve()

    config = prerelease_config(root)
    prefix = str(config["tag_prefix"])
    branch = check_branch(root, list(config["protected_branches"]))  # type: ignore[arg-type]
    base = workspace_version(root)
    tags = local_tags(root)

    if args.dry_run:
        chosen = select_version(base, prefix, tags, None)
        print("\n".join(plan_lines(base, chosen, prefix, branch, remote_checked=False)))
        return 0

    tags |= (remote_tag_lister or remote_tags)(root)
    check = registry_published if registry_published is not None else registry_published_default(root)
    chosen = select_version(base, prefix, tags, check)
    tag = f"{prefix}{chosen}"
    if chosen != base:
        files = apply_patch_bump(root, base, chosen)
        run_git(["add", "--", *[str(path) for path in files]], root)
        run_git(["commit", "-m", f"chore(release): prerelease patch bump {base} to {chosen}"], root)
    run_git(["push", "origin", f"HEAD:refs/heads/{branch}"], root)
    commit = run_git(["rev-parse", "HEAD"], root).strip()
    run_git(["tag", "-a", tag, "-m", f"Prerelease {chosen}", commit], root)
    run_git(["push", "origin", f"refs/tags/{tag}"], root)
    print(f"created and pushed {tag} from {commit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
