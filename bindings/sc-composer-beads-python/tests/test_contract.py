from __future__ import annotations

import builtins
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
GRAPH_FIXTURE_ROOT = FIXTURE_ROOT / "graph"


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
    assert beads.BeadOperation.PREVIEW_ATTACH == "preview_attach"
    assert beads.BeadOperation.ATTACH == "attach"
    assert {
        "BEADS_GRAPH_PARENT_NOT_FOUND": beads.BEADS_GRAPH_PARENT_NOT_FOUND,
        "BEADS_GRAPH_ID_INVALID": beads.BEADS_GRAPH_ID_INVALID,
        "BEADS_GRAPH_SCOPE_MISMATCH": beads.BEADS_GRAPH_SCOPE_MISMATCH,
        "BEADS_GRAPH_FORMULA_UNSUPPORTED": beads.BEADS_GRAPH_FORMULA_UNSUPPORTED,
        "BEADS_GRAPH_RELATION_INVALID": beads.BEADS_GRAPH_RELATION_INVALID,
        "BEADS_GRAPH_CONFLICT": beads.BEADS_GRAPH_CONFLICT,
        "BEADS_GRAPH_EDGE_CONFLICT": beads.BEADS_GRAPH_EDGE_CONFLICT,
        "BEADS_GRAPH_EDGE_MISSING": beads.BEADS_GRAPH_EDGE_MISSING,
        "BEADS_GRAPH_READ_FAILED": beads.BEADS_GRAPH_READ_FAILED,
        "BEADS_GRAPH_APPLY_FAILED": beads.BEADS_GRAPH_APPLY_FAILED,
    } == {
        "BEADS_GRAPH_PARENT_NOT_FOUND": "BEADS_GRAPH_PARENT_NOT_FOUND",
        "BEADS_GRAPH_ID_INVALID": "BEADS_GRAPH_ID_INVALID",
        "BEADS_GRAPH_SCOPE_MISMATCH": "BEADS_GRAPH_SCOPE_MISMATCH",
        "BEADS_GRAPH_FORMULA_UNSUPPORTED": "BEADS_GRAPH_FORMULA_UNSUPPORTED",
        "BEADS_GRAPH_RELATION_INVALID": "BEADS_GRAPH_RELATION_INVALID",
        "BEADS_GRAPH_CONFLICT": "BEADS_GRAPH_CONFLICT",
        "BEADS_GRAPH_EDGE_CONFLICT": "BEADS_GRAPH_EDGE_CONFLICT",
        "BEADS_GRAPH_EDGE_MISSING": "BEADS_GRAPH_EDGE_MISSING",
        "BEADS_GRAPH_READ_FAILED": "BEADS_GRAPH_READ_FAILED",
        "BEADS_GRAPH_APPLY_FAILED": "BEADS_GRAPH_APPLY_FAILED",
    }


@pytest.mark.parametrize(
    ("name", "pour_mode", "outcome_code"),
    [
        ("receipt-graph-pour.json", "graph", None),
        ("receipt-registry.json", "registry", None),
        ("receipt-conflict.json", "graph", beads.BEADS_GRAPH_CONFLICT),
        ("receipt-edge-missing.json", "graph", beads.BEADS_GRAPH_EDGE_MISSING),
    ],
)
def test_graph_receipt_fixtures_remain_json_contracts(
    name: str, pour_mode: str, outcome_code: str | None
) -> None:
    fixture = json.loads((GRAPH_FIXTURE_ROOT / name).read_text(encoding="utf-8"))
    receipt = beads.BeadComposeReceipt.from_json(json.dumps(fixture))

    assert receipt.to_json() == fixture
    assert receipt.pour_mode == pour_mode
    assert receipt.graph == fixture.get("graph")
    if outcome_code is not None:
        assert receipt.outcome.code == outcome_code


def test_receipt_decode_errors_have_a_receipt_code_and_stage() -> None:
    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeReceipt.from_json("{}")

    assert raised.value.code == "BEADS_RECEIPT_DESERIALIZATION_FAILED"
    assert raised.value.stage == "receipt"
    assert raised.value.message.startswith("failed to decode receipt:")


@pytest.mark.parametrize("name", ["render", "validate", "preview_pour", "pour", "preview_attach", "attach"])
def test_operation_names_preserve_the_wire_contract(tmp_path: Path, name: str) -> None:
    request = beads.BeadComposeRequest(
        tmp_path, tmp_path / "template.j2", tmp_path / "output.toml", {}, operation=name
    )
    fixture = json.loads((GRAPH_FIXTURE_ROOT / "receipt-registry.json").read_text(encoding="utf-8"))
    fixture["operation"] = name

    receipt = beads.BeadComposeReceipt.from_json(json.dumps(fixture))

    assert request.operation == getattr(beads.BeadOperation, name.upper()) == name
    assert receipt.operation == receipt.to_json()["operation"] == name


@pytest.mark.parametrize("name", ["render", "validate", "resolve_active_registry", "preview_pour", "pour", "preview_attach", "attach"])
@pytest.mark.parametrize("kind", ["succeeded", "skipped", "failed"])
def test_stage_names_and_outcomes_preserve_the_wire_contract(name: str, kind: str) -> None:
    fixture = json.loads((GRAPH_FIXTURE_ROOT / "receipt-registry.json").read_text(encoding="utf-8"))
    fixture["stages"] = [{
        "stage": name, "argv": [], "exit_status": None, "elapsed_ms": 0,
        "stdout_excerpt": "", "stderr_excerpt": "",
        "outcome": {kind: {"code": "test-code"}} if kind == "failed" else kind,
    }]

    receipt = beads.BeadComposeReceipt.from_json(json.dumps(fixture))

    assert receipt.stages[0].stage == getattr(beads.BeadStage, name.upper()) == name
    assert receipt.stages[0].outcome.kind == kind
    assert receipt.stages[0].outcome.code == ("test-code" if kind == "failed" else None)
    assert receipt.to_json() == fixture


@pytest.mark.parametrize("kind", ["succeeded", "refused", "failed"])
def test_receipt_outcome_names_preserve_the_wire_contract(kind: str) -> None:
    fixture = json.loads((GRAPH_FIXTURE_ROOT / "receipt-registry.json").read_text(encoding="utf-8"))
    fixture["outcome"] = kind if kind == "succeeded" else {kind: {"code": "test-code"}}

    receipt = beads.BeadComposeReceipt.from_json(json.dumps(fixture))

    assert receipt.outcome.kind == kind
    assert receipt.outcome.code == (None if kind == "succeeded" else "test-code")
    assert receipt.to_json() == fixture


@pytest.mark.parametrize("name", ["", "unknown", "preview-attach", "Render"])
def test_unknown_operation_preserves_the_request_error(tmp_path: Path, name: str) -> None:
    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(
            tmp_path, tmp_path / "template.j2", tmp_path / "output.toml", {}, operation=name
        )

    assert raised.value.code == "BEADS_REQUEST_DESERIALIZATION_FAILED"
    assert raised.value.stage == "request"
    assert raised.value.message == "operation must be render, validate, preview_pour, pour, preview_attach, or attach"


@pytest.mark.parametrize("failure", ["import", "loads"])
def test_graph_receipt_conversion_failures_raise_bead_compose_error(
    monkeypatch: pytest.MonkeyPatch, failure: str
) -> None:
    wire = (GRAPH_FIXTURE_ROOT / "receipt-graph-pour.json").read_text(encoding="utf-8")
    message = f"injected graph JSON {failure} failure"
    if failure == "import":
        original_import = builtins.__import__

        def fail_json_import(name: str, *args: object, **kwargs: object) -> object:
            if name == "json":
                raise ImportError(message)
            return original_import(name, *args, **kwargs)

        monkeypatch.setattr(builtins, "__import__", fail_json_import)
    else:
        def fail_json_loads(*args: object, **kwargs: object) -> object:
            raise ValueError(message)

        monkeypatch.setattr(json, "loads", fail_json_loads)

    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeReceipt.from_json(wire)

    assert raised.value.code == "BEADS_REQUEST_DESERIALIZATION_FAILED"
    assert raised.value.stage == "request"
    assert message in raised.value.message


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


def test_execute_preserves_the_request_operation(tmp_path: Path) -> None:
    executable, _trace = _write_fake_bd(tmp_path)

    receipt = beads.execute(_request(tmp_path, executable, operation="preview_pour"))

    assert receipt.operation == "preview_pour"
    assert receipt.stages[-1].stage == "preview_pour"


def test_compose_variables_reject_unsupported_python_values(tmp_path: Path) -> None:
    executable, _trace = _write_fake_bd(tmp_path)

    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(
            tmp_path,
            tmp_path / "template.toml.j2",
            tmp_path / "output.toml",
            {"unsupported": {1, 2, 3}},
            bd_executable=executable,
        )

    assert raised.value.code == "BEADS_REQUEST_DESERIALIZATION_FAILED"
    assert raised.value.stage == "request"


def test_compose_variables_reject_non_string_object_keys(tmp_path: Path) -> None:
    executable, _trace = _write_fake_bd(tmp_path)

    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(
            tmp_path,
            tmp_path / "template.toml.j2",
            tmp_path / "output.toml",
            {"nested": {1: "value"}},
            bd_executable=executable,
        )

    assert raised.value.code == "BEADS_REQUEST_DESERIALIZATION_FAILED"
    assert raised.value.stage == "request"
    assert raised.value.message == "compose_variables object keys must be strings"


def test_attach_request_fields_reach_the_rust_graph_plan(tmp_path: Path) -> None:
    cooked = {
        "schema_version": 1,
        "formula": "release",
        "type": "workflow",
        "steps": [
            {"id": "build", "title": "Build"},
            {"id": "verify", "title": "Verify"},
        ],
    }
    responses = {
        "cook": cooked,
        "show": [{"id": "proj-100"}, {"id": "proj-200"}],
        "create": {},
    }
    if os.name == "nt":
        executable = tmp_path / "fake-graph-bd.cmd"
        executable.write_text(
            "@echo off\r\n"
            + "".join(
                f'if /I "%~1"=="{command}" (echo {json.dumps(response)} & exit /b 0)\r\n'
                for command, response in responses.items()
            )
            + "exit /b 1\r\n",
            encoding="utf-8",
            newline="",
        )
    else:
        executable = tmp_path / "fake-graph-bd"
        executable.write_text(
            '#!/bin/sh\ncase "$1" in\n'
            + "".join(
                f"{command}) printf '%s\\n' '{json.dumps(response)}';;\n"
                for command, response in responses.items()
            )
            + "*) exit 1;;\nesac\n",
            encoding="utf-8",
        )
        executable.chmod(executable.stat().st_mode | stat.S_IXUSR)
    template = tmp_path / "release.formula.json.j2"
    template.write_text(json.dumps(cooked), encoding="utf-8")
    request = beads.BeadComposeRequest(
        tmp_path,
        template,
        tmp_path / "release.formula.json",
        {},
        operation="preview_attach",
        parent="proj-100",
        ref="release_1",
        relations=[
            {"from": "step:verify", "to": "step:build", "type": "blocks"},
            {"from": "step:build", "to": "bead:proj-200", "type": "related"},
        ],
        bd_executable=executable,
    )

    receipt = beads.execute(request)

    assert receipt.operation == "preview_attach"
    assert receipt.outcome.kind == "succeeded"
    assert receipt.graph["parent"] == "proj-100"
    assert receipt.graph["ref"] == "release_1"
    assert receipt.graph["ids"] == {
        "build": "proj-100.release_1-build",
        "verify": "proj-100.release_1-verify",
    }
    edges = {(edge["from"], edge["to"], edge["type"]) for edge in receipt.graph["edges"]}
    assert ("proj-100.release_1-verify", "proj-100.release_1-build", "blocks") in edges
    assert ("proj-100.release_1-build", "proj-200", "related") in edges
    assert receipt.stages[-1].argv[1] == "create"
    assert "--dry-run" in receipt.stages[-1].argv


@pytest.mark.parametrize(
    ("parent", "reference"),
    [("invalid parent", "release_1"), ("proj-100", "invalid.ref")],
)
def test_attach_request_rejects_invalid_parent_and_ref(
    tmp_path: Path, parent: str, reference: str
) -> None:
    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(
            tmp_path,
            tmp_path / "template.toml.j2",
            tmp_path / "output.toml",
            {},
            operation="preview_attach",
            parent=parent,
            ref=reference,
        )

    assert raised.value.code == "BEADS_REQUEST_DESERIALIZATION_FAILED"
    assert raised.value.stage == "request"


@pytest.mark.parametrize(
    "relation, expected_code",
    [
        ({"from": "root", "to": "bead:parent", "type": "blocks"}, "BEADS_RELATION_ENDPOINT_INVALID"),
        ({"from": "step:build", "to": "", "type": "blocks"}, "BEADS_RELATION_ENDPOINT_INVALID"),
        ({"from": "step:build", "to": "bead:child", "type": "unknown"}, "BEADS_REQUEST_DESERIALIZATION_FAILED"),
    ],
)
def test_relations_reject_malformed_endpoints_and_unknown_types(
    tmp_path: Path, relation: dict[str, str], expected_code: str
) -> None:
    executable, _trace = _write_fake_bd(tmp_path)

    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(
            tmp_path,
            tmp_path / "template.toml.j2",
            tmp_path / "output.toml",
            {},
            relations=[relation],
            bd_executable=executable,
        )

    assert raised.value.code == expected_code
    assert raised.value.stage == "request"


@pytest.mark.parametrize("value", [float("nan"), float("inf"), -float("inf")])
def test_compose_variables_reject_non_finite_floats(tmp_path: Path, value: float) -> None:
    executable, _trace = _write_fake_bd(tmp_path)

    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(
            tmp_path,
            tmp_path / "template.toml.j2",
            tmp_path / "output.toml",
            {"value": value},
            bd_executable=executable,
        )

    assert raised.value.code == "BEADS_REQUEST_DESERIALIZATION_FAILED"
    assert raised.value.stage == "request"
    assert raised.value.message == "compose_variables floating-point values must be finite"


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


def test_invalid_authorization_uses_stable_error_code(tmp_path: Path) -> None:
    output = tmp_path / "output.formula.toml"
    with pytest.raises(beads.BeadComposeError) as caught:
        beads.BeadComposeRequest(
            tmp_path,
            tmp_path / "template.formula.toml.j2",
            output,
            {},
            operation="pour",
            formula_name="workflow",
            pour_authorization="invalid",
        )
    assert caught.value.code == "BEADS_POUR_AUTH_INVALID"
    assert caught.value.stage == "request"
    assert not output.exists()
