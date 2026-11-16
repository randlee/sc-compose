"""Controlled checks for staged release smoke execution."""
import importlib.util
from pathlib import Path
import subprocess

import pytest

SCRIPT = Path(__file__).resolve().parents[1] / "stage_examples_smoke.py"
SPEC = importlib.util.spec_from_file_location("stage_examples_smoke", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_staged_smoke_runs_binary_and_requires_hello(tmp_path, monkeypatch):
    monkeypatch.setenv("SC_COMPOSE_DATA_DIR", "/unrelated")
    calls = []

    def run(command, **options):
        calls.append((command, options))
        return subprocess.CompletedProcess(command, 0, "hello\n", "")

    monkeypatch.setattr(MODULE.subprocess, "run", run)
    manifest = {"release_binaries": [{"name": "sc-compose", "bundled_paths": [{
        "stage_smoke_args": ["examples", "list"], "stage_smoke_line": "hello",
    }]}]}
    MODULE.run_checks(tmp_path, "x86_64-unknown-linux-gnu", manifest)
    command, options = calls[0]
    assert command == [str(tmp_path / "bin" / "sc-compose"), "examples", "list"]
    assert options["cwd"] == tmp_path
    assert "SC_COMPOSE_DATA_DIR" not in options["env"]
    assert options["timeout"] == 120

    def empty(command, **_options):
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr(MODULE.subprocess, "run", empty)
    with pytest.raises(RuntimeError, match="staged smoke check failed"):
        MODULE.run_checks(tmp_path, "x86_64-unknown-linux-gnu", manifest)
