from __future__ import annotations

import json
import os
import shutil
import stat
import subprocess
from pathlib import Path

import pytest

import sc_composer_beads as beads


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPOSITORY_ROOT / "crates" / "sc-composer-beads" / "tests" / "fixtures" / "beads"


def _write_fake_bd(root: Path) -> tuple[Path, Path]:
    """Create the closed fake Beads runner used for cross-surface receipts."""
    trace = root / "bd-trace.txt"
    active_registry = root / ".beads"
    if os.name == "nt":
        executable = root / "fake-bd.cmd"
        registry = active_registry.as_posix()
        executable.write_text(
            "@echo off\r\n"
            "setlocal\r\n"
            'set "stage=%~1"\r\n'
            f'echo %stage%>>"{trace}"\r\n'
            'if /I "%stage%"=="where" (\r\n'
            f'  echo {{"path":"{registry}"}}\r\n'
            "  exit /b 0\r\n"
            ")\r\n"
            "exit /b 0\r\n",
            encoding="utf-8",
            newline="",
        )
    else:
        executable = root / "fake-bd"
        executable.write_text(
            "#!/bin/sh\n"
            f"printf '%s\\n' \"$1\" >> '{trace}'\n"
            'if [ "$1" = "where" ]; then\n'
            f"  printf '%s\\n' '{{\"path\":\"{active_registry}\"}}'\n"
            "fi\n",
            encoding="utf-8",
        )
        executable.chmod(executable.stat().st_mode | stat.S_IXUSR)
    return executable, trace


def _request(root: Path, executable: Path, *, operation: str = "validate") -> beads.BeadComposeRequest:
    templates = root / "templates"
    templates.mkdir(exist_ok=True)
    template = templates / "toml-workflow.formula.toml.j2"
    shutil.copy2(FIXTURE_ROOT / template.name, template)
    output = root / ".beads" / "formulas" / "toml-workflow.formula.toml"
    output.parent.mkdir(parents=True, exist_ok=True)
    return beads.BeadComposeRequest(
        root,
        template,
        output,
        {
            "project": {"name": "sc-compose", "notes": "Python contract fixture"},
            "reviewers": [{"id": "ada", "name": "Ada"}],
        },
        operation=operation,
        formula_name="toml-workflow",
        bead_variables={"release_name": "1.5.0"},
        bd_executable=executable,
    )


def test_import_surface_exposes_versioned_beads_contract() -> None:
    assert beads.BEADS_SCHEMA_V1 == "sc-compose/beads/v1"
    assert beads.BeadOperation.VALIDATE == "validate"
    assert beads.PourAuthorization.CREATE_PERSISTENT_BEADS == "CreatePersistentBeads"


def test_validate_and_preview_preserve_stage_receipts(tmp_path: Path) -> None:
    executable, trace = _write_fake_bd(tmp_path)

    validated = beads.validate(_request(tmp_path, executable))
    previewed = beads.preview_pour(_request(tmp_path, executable))

    assert validated.operation == "validate"
    assert [stage.stage for stage in validated.stages] == ["render", "validate"]
    assert validated.stages[-1].argv[1] == "cook"
    assert validated.outcome.kind == "succeeded"
    assert [stage.stage for stage in previewed.stages] == [
        "render",
        "validate",
        "resolve_active_registry",
        "preview_pour",
    ]
    assert previewed.stages[-1].argv[1:3] == ["mol", "pour"]
    assert previewed.stages[-1].argv[-2:] == ["--var", "release_name=1.5.0"]
    assert trace.read_text(encoding="utf-8").splitlines() == ["cook", "cook", "where", "mol"]


def test_python_and_cli_preview_receipts_have_the_same_stages(tmp_path: Path) -> None:
    executable, _trace = _write_fake_bd(tmp_path)
    request = _request(tmp_path, executable, operation="preview_pour")
    request_path = tmp_path / "request.json"
    request_path.write_text(
        json.dumps(
            {
                "schema": request.schema,
                "operation": request.operation,
                "working_directory": request.working_directory,
                "template": request.template,
                "rendered_formula": request.rendered_formula,
                "compose_variables": request.compose_variables,
                "formula_name": request.formula_name,
                "bead_variables": request.bead_variables,
                "bd_executable": request.bd_executable,
                "pour_authorization": request.pour_authorization,
            }
        ),
        encoding="utf-8",
    )

    python_receipt = beads.preview_pour(request)
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "sc-compose",
            "--",
            "bead",
            "preview-pour",
            "--request",
            str(request_path),
            "--json",
        ],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    cli_receipt = json.loads(completed.stdout)["payload"]
    assert cli_receipt["operation"] == python_receipt.operation
    assert [stage["stage"] for stage in cli_receipt["stages"]] == [
        stage.stage for stage in python_receipt.stages
    ]
    assert cli_receipt["outcome"] == python_receipt.outcome.kind


def test_pour_refuses_before_starting_the_runner(tmp_path: Path) -> None:
    executable, trace = _write_fake_bd(tmp_path)

    with pytest.raises(beads.BeadComposeError) as raised:
        beads.pour(_request(tmp_path, executable, operation="pour"))

    assert raised.value.code == "BEADS_POUR_AUTH_REQUIRED"
    assert not trace.exists()


@pytest.mark.skipif(
    "BD_EXECUTABLE" not in os.environ,
    reason="the pinned Beads executable is configured by the CI wheel job",
)
def test_installed_wheel_runs_the_pinned_beads_fixture(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Exercise the installed wheel with the same canonical fixture as R.1/R.2."""
    executable = Path(os.environ["BD_EXECUTABLE"])
    monkeypatch.setenv("BEADS_NO_DAEMON", "1")
    subprocess.run([executable, "init"], cwd=tmp_path, check=True, capture_output=True)

    receipt = beads.preview_pour(_request(tmp_path, executable, operation="preview_pour"))

    assert receipt.outcome.kind == "succeeded"
    assert [stage.stage for stage in receipt.stages] == [
        "render",
        "validate",
        "resolve_active_registry",
        "preview_pour",
    ]
