#!/usr/bin/env python3
"""Build, verify and publish manifest-declared native Go module bundles."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile


class GoReleaseError(ValueError):
    pass


def safe_path(value, label):
    if (not isinstance(value, str) or not value or "\\" in value or
            PurePosixPath(value).is_absolute() or any(part in {".", ".."} for part in value.split("/"))):
        raise GoReleaseError(f"{label} must be a repository-relative path without traversal")
    return value


def configuration(manifest):
    rows = manifest.get("go_modules", [])
    if not isinstance(rows, list):
        raise GoReleaseError("go_modules must be an array")
    modules, prefixes, sources = set(), set(), set()
    required = {"module", "source", "artifact_prefix", "targets"}
    optional = {"prepare", "build", "stage", "consumer_test", "rust_toolchain"}
    for row in rows:
        if not isinstance(row, dict) or not required <= set(row) or set(row) - required - optional:
            raise GoReleaseError("Go module requires module, source, artifact_prefix and targets; unknown fields refused")
        module = row["module"]
        if not isinstance(module, str) or not re.fullmatch(r"[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+", module):
            raise GoReleaseError("invalid Go module path")
        source = safe_path(row["source"], "Go source")
        prefix = row["artifact_prefix"]
        if not isinstance(prefix, str) or not re.fullmatch(r"[A-Za-z0-9_-]+", prefix):
            raise GoReleaseError("invalid Go artifact prefix")
        if module in modules or prefix in prefixes or source in sources:
            raise GoReleaseError("Go modules, sources and artifact prefixes must be unique")
        modules.add(module); prefixes.add(prefix); sources.add(source)
        if "consumer_test" in row:
            safe_path(row["consumer_test"], "Go consumer test")
        if "rust_toolchain" in row and (not isinstance(row["rust_toolchain"], str) or not re.fullmatch(r"[A-Za-z0-9_.-]+", row["rust_toolchain"])):
            raise GoReleaseError("invalid Go native Rust toolchain")
        for hook in ("prepare", "build", "stage"):
            if hook in row and (not isinstance(row[hook], list) or not row[hook] or
                                any(not isinstance(arg, str) or not arg or "\0" in arg for arg in row[hook])):
                raise GoReleaseError(f"Go {hook} must be a nonempty argv array")
        if not isinstance(row["targets"], list) or not row["targets"]:
            raise GoReleaseError("Go targets must be a nonempty array")
        targets, coordinates = set(), set()
        for target in row["targets"]:
            if not isinstance(target, dict) or set(target) != {"target", "os", "goos", "goarch"}:
                raise GoReleaseError("Go target requires exactly target, os, goos and goarch")
            if any(not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9_.-]+", value) for value in target.values()):
                raise GoReleaseError("unsafe Go target identifier")
            if target["goos"] not in {"linux", "darwin", "windows"} or target["goarch"] not in {"amd64", "arm64"}:
                raise GoReleaseError("unsupported native Go coordinate")
            if target["target"] in targets or (target["goos"], target["goarch"]) in coordinates:
                raise GoReleaseError("duplicate Go native target or coordinate")
            targets.add(target["target"]); coordinates.add((target["goos"], target["goarch"]))
    return rows


def matrix(manifest):
    return {"include": [{**row, **target, "artifact": f"{row['artifact_prefix']}-{target['target']}"}
                         for row in configuration(manifest) for target in row["targets"]]}


def print_asset_patterns(manifest):
    for row in matrix(manifest)["include"]:
        print("^" + re.escape(row["artifact"]) + r"\.zip$")


def execute(argv, *, cwd=None, env=None):
    return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                          check=True, timeout=900).stdout.strip()


def hook(row, name, values, root, run=execute):
    if name in row:
        try:
            args = [arg.format_map(values) for arg in row[name]]
        except (KeyError, ValueError) as error:
            raise GoReleaseError(f"invalid Go {name} placeholder: {error}") from error
        output = run(args, cwd=root)
        if output:
            print(output, file=sys.stderr)


def contained(root, value):
    path = (root / value).resolve()
    if not path.is_relative_to(root):
        raise GoReleaseError("Go path escapes the consumer repository")
    return path


def module_name(path):
    matches = re.findall(r"^module\s+(\S+)\s*$", (path / "go.mod").read_text(), re.MULTILINE)
    if len(matches) != 1:
        raise GoReleaseError("Go module must declare exactly one module path")
    return matches[0]


def verify_tree(path, row, root, run=execute):
    if module_name(path) != row["module"]:
        raise GoReleaseError("staged Go module path differs from declared module")
    result = run(["go", "test", "./..."], cwd=path)
    if result:
        print(result, file=sys.stderr)
    if "consumer_test" in row:
        with tempfile.TemporaryDirectory(prefix="go-independent-consumer-") as temp:
            consumer = Path(temp) / "consumer"
            shutil.copytree(contained(root, row["consumer_test"]), consumer)
            run(["go", "mod", "edit", "-replace", f"{row['module']}={path}"], cwd=consumer)
            run(["go", "mod", "tidy"], cwd=consumer)
            run(["go", "test", "./..."], cwd=consumer)
            run(["go", "run", "."], cwd=consumer)


def build(manifest, artifact, version, root, output, *, prepare_only=False, run=execute):
    validate_version(version)
    rows = [row for row in matrix(manifest)["include"] if row["artifact"] == artifact]
    if len(rows) != 1:
        raise GoReleaseError("Go artifact must identify exactly one declared target")
    row = rows[0]
    root = root.resolve()
    source = contained(root, row["source"])
    if module_name(source) != row["module"]:
        raise GoReleaseError("source go.mod differs from declared module path")
    values = {**row, "version": version.removeprefix("v"), "root": str(root), "source": str(source)}
    if prepare_only:
        hook(row, "prepare", values, root, run)
        return
    coordinate = run(["go", "env", "GOOS", "GOARCH"], cwd=root).splitlines()
    if coordinate != [row["goos"], row["goarch"]]:
        raise GoReleaseError("Go bundle must be tested on its declared native coordinate")
    hook(row, "build", values, root, run)
    output.mkdir(parents=True, exist_ok=True)
    archive = output / (artifact + ".zip")
    with tempfile.TemporaryDirectory(prefix="go-module-bundle-") as temp:
        stage = Path(temp) / "module"
        values["stage"] = str(stage)
        if "stage" in row:
            hook(row, "stage", values, root, run)
        else:
            shutil.copytree(source, stage, ignore=shutil.ignore_patterns(".git"))
        verify_tree(stage, row, root, run)
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
            for path in sorted(stage.rglob("*")):
                if path.is_symlink():
                    raise GoReleaseError("Go bundle may not contain symlinks")
                if path.is_file():
                    bundle.write(path, path.relative_to(stage).as_posix())
        unpacked = Path(temp) / "unpacked"
        with zipfile.ZipFile(archive) as bundle:
            bundle.extractall(unpacked)
        verify_tree(unpacked, row, root, run)
    return archive


def verify_bundles(manifest, directory, *, mixed=False):
    rows = matrix(manifest)["include"]
    expected = {row["artifact"] + ".zip": row for row in rows}
    # The Go artifact download uses its own directory, so extras and nested
    # duplicate names cannot be hidden by merging uploads into one directory.
    found = list(directory.rglob("*.zip")) if directory.exists() else []
    if mixed:
        prefixes = tuple(row["artifact_prefix"] + "-" for row in rows)
        found = [path for path in found if path.name.startswith(prefixes)]
    if len(found) != len(expected) or {path.name for path in found} != set(expected):
        raise GoReleaseError("Go bundles do not match the exact declared target set")
    for path in found:
        if not path.is_file() or path.stat().st_size == 0:
            raise GoReleaseError("Go bundle is empty or missing")
        with zipfile.ZipFile(path) as bundle:
            names = bundle.namelist()
            if (not names or "go.mod" not in names or len(names) != len(set(names)) or
                    any(PurePosixPath(name).is_absolute() or ".." in PurePosixPath(name).parts for name in names) or
                    bundle.testzip() is not None):
                raise GoReleaseError("Go bundle is invalid")
            module = re.findall(r"^module\s+(\S+)\s*$", bundle.read("go.mod").decode("utf-8"), re.MULTILINE)
            if module != [expected[path.name]["module"]]:
                raise GoReleaseError("Go bundle module path differs from declaration")
    return sorted(found)


def validate_version(version):
    if not isinstance(version, str) or not re.fullmatch(r"v?\d+\.\d+\.\d+", version):
        raise GoReleaseError("Go release version must be X.Y.Z or vX.Y.Z")


def ensure_tag(tag, commit, *, run=execute):
    """Create an absent tag once, reuse an exact match, refuse any conflict."""
    if not re.fullmatch(r"[0-9a-f]{40}", commit) or not re.fullmatch(r"[A-Za-z0-9_./-]+", tag):
        raise GoReleaseError("tag publication requires a safe tag and exact commit")
    remote = run(["git", "ls-remote", "origin", f"refs/tags/{tag}", f"refs/tags/{tag}^{{}}"])
    entries = [line.split() for line in remote.splitlines()]
    if any(len(parts) != 2 for parts in entries):
        raise GoReleaseError("invalid remote tag evidence")
    refs = {parts[1]: parts[0] for parts in entries}
    if refs:
        actual = refs.get(f"refs/tags/{tag}^{{}}", refs.get(f"refs/tags/{tag}"))
        if actual != commit:
            raise GoReleaseError(f"tag {tag} already exists at another commit")
        return "reused"
    # Push an exact commit ref without changing or trusting a local tag.
    run(["git", "push", "origin", f"{commit}:refs/tags/{tag}"])
    return "created"


def publish(manifest, directory, version, commit, *, run=execute):
    validate_version(version)
    rows = configuration(manifest)
    if not rows:
        return
    verify_bundles(manifest, directory, mixed=True)
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise GoReleaseError("Go publication requires an exact release commit")
    for row in rows:
        ensure_tag(f"{row['source']}/v{version.removeprefix('v')}", commit, run=run)
        run(["go", "list", "-m", f"{row['module']}@v{version.removeprefix('v')}"],
            env=dict(os.environ, GOPROXY="https://proxy.golang.org", GONOPROXY="none", GOPRIVATE=""))


def read_manifest(path, source_ref=None):
    if source_ref:
        if Path(path).is_absolute() or ":" in path or "\\" in path or ".." in PurePosixPath(path).parts:
            raise GoReleaseError("--source-ref requires a repository-relative manifest path")
        commit = execute(["git", "rev-parse", "--verify", source_ref + "^{commit}"])
        entry = execute(["git", "ls-tree", "--name-only", commit, "--", path])
        if not entry:
            return {}
        return tomllib.loads(execute(["git", "show", f"{commit}:{path}"]))
    return tomllib.loads(Path(path).read_text())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["plan", "prepare", "build", "verify", "publish", "release-tag"])
    parser.add_argument("--manifest", default="release/publish-artifacts.toml")
    parser.add_argument("--source-ref", help="read-only committed manifest lookup for plan")
    parser.add_argument("--artifact")
    parser.add_argument("--version")
    parser.add_argument("--commit")
    parser.add_argument("--directory", type=Path, default=Path("go-bundles"))
    args = parser.parse_args()
    try:
        if args.source_ref and args.command != "plan":
            raise GoReleaseError("--source-ref is only supported for read-only plan")
        manifest = read_manifest(args.manifest, args.source_ref)
        rows = configuration(manifest)
        if args.command == "plan":
            print(json.dumps({"enabled": bool(rows), "matrix": matrix(manifest)}, separators=(",", ":")))
        elif args.command in {"prepare", "build"}:
            build(manifest, args.artifact, args.version, Path.cwd(), args.directory,
                  prepare_only=args.command == "prepare")
        elif args.command == "verify":
            verify_bundles(manifest, args.directory)
        elif args.command == "publish":
            publish(manifest, args.directory, args.version, args.commit)
        elif rows:
            validate_version(args.version)
            verify_bundles(manifest, args.directory)
            ensure_tag("v" + args.version.removeprefix("v"), args.commit)
        return 0
    except (ValueError, OSError, KeyError, TypeError, zipfile.BadZipFile, subprocess.SubprocessError) as error:
        print(f"Go release failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
