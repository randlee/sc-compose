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


def test_archive_verifier_declaration_survives_generated_workflows():
    import json
    import tomllib
    import yaml

    root = SCRIPT.parents[2]
    expected = ["python3", ".github/scripts/stage_examples_smoke.py", "{stage_dir}", "{target}", "{manifest}"]
    values = json.loads((root / "release/sc-publish-install.json").read_text())
    manifest = tomllib.loads((root / "release/publish-artifacts.toml").read_text())
    assert values["archive_verification"] == manifest["archive_verification"] == expected
    for name in ["release.yml", "prerelease-archive.yml"]:
        jobs = yaml.safe_load((root / ".github/workflows" / name).read_text())["jobs"]
        steps = jobs["build"]["steps"]
        package = next(step for step in steps if step.get("name", "").startswith("Package manifest-declared"))
        upload = next(step for step in steps if step.get("name") == "Upload artifact")
        assert steps.index(package) < steps.index(upload)
        assert not package.get("continue-on-error") and "if" not in upload
        assert package["run"].index("archive_verification.py") < package["run"].index("archive = Path(")
