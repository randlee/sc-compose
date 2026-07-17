from os import PathLike
from typing import Any


class ScComposeError(Exception): ...


class ComposeMode:
    @staticmethod
    def file(template_path: str | PathLike[str]) -> ComposeMode: ...
    @property
    def template_path(self) -> str | None: ...


class ComposePolicy:
    def __init__(self, strict_undeclared_variables: bool = False) -> None: ...
    @property
    def strict_undeclared_variables(self) -> bool: ...


class ComposeRequest:
    def __init__(
        self,
        root: str | PathLike[str],
        mode: ComposeMode,
        vars_input: dict[str, Any] | None = None,
        vars_env: dict[str, Any] | None = None,
        vars_defaults: dict[str, Any] | None = None,
        guidance_block: str | None = None,
        user_prompt: str | None = None,
        policy: ComposePolicy | None = None,
    ) -> None: ...
    @property
    def root(self) -> str: ...
    @property
    def mode(self) -> ComposeMode: ...


class ComposeResult:
    @property
    def rendered_text(self) -> str: ...
    @property
    def resolved_files(self) -> list[str]: ...
    @property
    def warnings(self) -> list[str]: ...


def compose_file(request: ComposeRequest) -> ComposeResult: ...
