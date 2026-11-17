#!/usr/bin/env python3
"""Consumer-owned native tool setup and build hooks for the publication kit."""
from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
LLVM_MINGW_TAG = "20261104"
LLVM_MINGW_SHA256 = "7d7f8387a8706639b0b47a629d6e23f8b3ef9acce67e9832b8cd4c5bb2816eab"


def run(argv):
    return subprocess.run(argv, cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE).stdout.strip()


def append_env(name, text):
    path = os.environ.get(name)
    if not path:
        raise ValueError(f"{name} is required for native setup in Actions")
    with Path(path).open("a", encoding="utf-8") as output:
        output.write(text + "\n")


def native_target(target):
    contract = tomllib.loads((ROOT / "bindings/sc-sha-go/native/targets.toml").read_text())
    matches = [row for row in contract["targets"] if row["rust_target"] == target]
    if len(matches) != 1:
        raise ValueError("target is not in the native Go contract")
    return matches[0]


def prepare(target):
    row = native_target(target)
    if target == "aarch64-pc-windows-gnullvm":
        name = f"llvm-mingw-{LLVM_MINGW_TAG}-ucrt-aarch64"
        url = f"https://github.com/mstorsjo/llvm-mingw/releases/download/{LLVM_MINGW_TAG}/{name}.zip"
        archive = Path(os.environ["RUNNER_TEMP"]) / (name + ".zip")
        with urllib.request.urlopen(url, timeout=120) as response:
            archive.write_bytes(response.read())
        if hashlib.sha256(archive.read_bytes()).hexdigest() != LLVM_MINGW_SHA256:
            raise ValueError("LLVM-MinGW download digest mismatch")
        tools = Path(os.environ["RUNNER_TEMP"]) / "llvm-mingw"
        with zipfile.ZipFile(archive) as payload:
            payload.extractall(tools)
        append_env("GITHUB_PATH", str(tools / name / "bin"))
        append_env("GITHUB_ENV", "CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER=aarch64-w64-mingw32-clang")
        append_env("GITHUB_ENV", "CC=aarch64-w64-mingw32-clang")
    elif row["goos"] == "windows":
        tools = Path(r"C:\Program Files\LLVM\bin")
        if not (tools / "llvm-readobj.exe").is_file():
            run(["choco", "install", "llvm", "-y", "--no-progress"])
        append_env("GITHUB_PATH", str(tools))
    elif row["goos"] == "darwin":
        probe = subprocess.run(["brew", "--prefix", "llvm"], check=False, text=True, capture_output=True)
        if probe.returncode:
            run(["brew", "install", "llvm"])
        append_env("GITHUB_PATH", str(Path(run(["brew", "--prefix", "llvm"])) / "bin"))
    elif not shutil.which("llvm-objcopy"):
        run(["sudo", "apt-get", "update"])
        run(["sudo", "apt-get", "install", "-y", "llvm"])


def build(target):
    row = native_target(target)
    run(["cargo", "build", "--release", "--locked", "-j", "2", "-p", "sc-sha-go", "--target", target])
    library = ROOT / "target" / target / "release" / row["library"]
    run([sys.executable, "bindings/sc-sha-go/clean_archive.py", str(library), target])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["prepare", "build"])
    parser.add_argument("target")
    args = parser.parse_args()
    try:
        (prepare if args.command == "prepare" else build)(args.target)
        return 0
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"Native Go hook failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
