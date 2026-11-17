"""Controlled Homebrew installation state, including the hosted macOS failure."""
import importlib.util
from pathlib import Path
import subprocess

import pytest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("go_native_build", ROOT / "release/go-native-build.py")
hook = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(hook)


@pytest.mark.parametrize("installed", [False, True])
@pytest.mark.parametrize("target", ["aarch64-apple-darwin", "x86_64-apple-darwin"])
def test_prefix_success_does_not_prove_llvm_is_installed(tmp_path, monkeypatch, installed, target):
    prefix = tmp_path / "llvm"
    tools = prefix / "bin"
    tools.mkdir(parents=True)
    if installed:
        (tools / "llvm-objcopy").write_text("tool fixture")
    path_file = tmp_path / "github-path"
    monkeypatch.setenv("GITHUB_PATH", str(path_file))
    commands = []

    def probe(argv, **kwargs):
        if argv == ["brew", "list", "--versions", "llvm"]:
            return subprocess.CompletedProcess(argv, 0 if installed else 1, stdout="", stderr="")
        # Homebrew reports a formula's prefix even when no keg is installed.
        assert argv == ["brew", "--prefix", "llvm"]
        return subprocess.CompletedProcess(argv, 0, stdout=str(prefix), stderr="")

    def command(argv):
        commands.append(argv)
        if argv == ["brew", "install", "llvm"]:
            (tools / "llvm-objcopy").write_text("installed tool fixture")
            return ""
        assert argv == ["brew", "--prefix", "llvm"]
        return str(prefix)

    monkeypatch.setattr(hook.subprocess, "run", probe)
    monkeypatch.setattr(hook, "run", command)
    hook.prepare(target)
    assert (["brew", "install", "llvm"] in commands) is (not installed)
    assert path_file.read_text() == str(tools) + "\n"
    assert (tools / "llvm-objcopy").is_file()


def test_missing_llvm_executable_refuses_before_advertising_path(tmp_path, monkeypatch):
    path_file = tmp_path / "github-path"
    monkeypatch.setenv("GITHUB_PATH", str(path_file))
    monkeypatch.setattr(hook.subprocess, "run", lambda argv, **kwargs:
                        subprocess.CompletedProcess(argv, 0, stdout="installed", stderr=""))
    monkeypatch.setattr(hook, "run", lambda argv: str(tmp_path / "missing-llvm"))
    with pytest.raises(ValueError, match="missing llvm-objcopy"):
        hook.prepare("aarch64-apple-darwin")
    assert not path_file.exists()
