from os import PathLike
from typing import Any, Mapping

BEADS_SCHEMA_V1: str


class BeadComposeError(Exception):
    code: str
    stage: str | None
    message: str


class BeadOperation:
    RENDER: str
    VALIDATE: str
    PREVIEW_POUR: str
    POUR: str


class PourAuthorization:
    CREATE_PERSISTENT_BEADS: str


class BeadStage:
    RENDER: str
    VALIDATE: str
    RESOLVE_ACTIVE_REGISTRY: str
    PREVIEW_POUR: str
    POUR: str


class BeadStageOutcome:
    kind: str
    code: str | None


class BeadOutcome:
    kind: str
    code: str | None


class BeadStageReceipt:
    stage: str
    argv: list[str]
    exit_status: int | None
    elapsed_ms: int
    stdout_excerpt: str
    stderr_excerpt: str
    outcome: BeadStageOutcome


class BeadComposeReceipt:
    schema: str
    operation: str
    rendered_formula: str
    stages: list[BeadStageReceipt]
    outcome: BeadOutcome


class BeadComposeRequest:
    def __init__(
        self,
        working_directory: str | PathLike[str],
        template: str | PathLike[str],
        rendered_formula: str | PathLike[str],
        compose_variables: Mapping[str, Any],
        *,
        operation: str = "render",
        formula_name: str | None = None,
        bead_variables: Mapping[str, str] | None = None,
        bd_executable: str | PathLike[str] | None = None,
        pour_authorization: str | None = None,
        schema: str = BEADS_SCHEMA_V1,
    ) -> None: ...

    schema: str
    operation: str
    working_directory: str
    template: str
    rendered_formula: str
    compose_variables: Mapping[str, Any]
    formula_name: str | None
    bead_variables: Mapping[str, str]
    bd_executable: str | None
    pour_authorization: str | None


def execute(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def render(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def validate(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def preview_pour(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def pour(request: BeadComposeRequest) -> BeadComposeReceipt: ...
