from os import PathLike
from typing import Any, Literal


class ScComposeError(Exception):
    message: str
    code: str | None
    recovery_hints: list[str]
    diagnostic_kind: str | None
    diagnostic_message: str | None
    diagnostic_occurrence: int | None


class ScRenderError(ScComposeError): ...


class ScValidationError(ScComposeError): ...


class ScResolveError(ScComposeError): ...


class ScIncludeError(ScComposeError): ...


class ScConfigError(ScComposeError): ...


class RuntimeKind:
    CLAUDE: str
    CODEX: str
    GEMINI: str
    OPENCODE: str


class ProfileKind:
    AGENT: str
    COMMAND: str
    SKILL: str


class UnknownVariablePolicy:
    ERROR: str
    WARN: str
    IGNORE: str


class VariableSource:
    EXPLICIT_INPUT: str
    ENVIRONMENT: str
    BUILTIN: str
    TEMPLATE_INPUT_DEFAULT: str
    FRONTMATTER_DEFAULT: str
    INCLUDED_DEFAULT: str


class DiagnosticSeverity:
    ERROR: str
    WARNING: str
    INFO: str


class DiagnosticCode:
    ERR_RESOLVE_NOT_FOUND: str
    ERR_RESOLVE_AMBIGUOUS: str
    ERR_INCLUDE_ESCAPE: str
    ERR_INCLUDE_NOT_FOUND: str
    ERR_INCLUDE_CYCLE: str
    ERR_INCLUDE_DEPTH: str
    ERR_VAL_OBJECT_SHAPE: str
    ERR_VAL_NESTED_ARRAY_UNSUPPORTED: str
    ERR_VAL_DUPLICATE: str
    WARN_VAL_CONFLICTING_DEFAULT_SECTIONS: str
    ERR_VAL_EMPTY: str
    ERR_VAL_MISSING_FRONTMATTER: str
    ERR_VAL_MISSING_REQUIRED: str
    ERR_VAL_MISSING_NESTED_FIELD: str
    ERR_VAL_SHAPE_MISMATCH: str
    ERR_VAL_UNDECLARED_TOKEN: str
    ERR_VAL_EXTRA_INPUT: str
    INFO_VAL_DEFAULT_USED: str
    ERR_RENDER_STDIN_DOUBLE_READ: str
    ERR_RENDER_WRITE: str
    ERR_CONFIG_READONLY: str
    ERR_CONFIG_MODE: str
    ERR_CONFIG_PARSE: str
    ERR_CONFIG_VARFILE: str
    ERR_CONFIG_PACK_NOT_FOUND: str
    ERR_CONFIG_PACK_NOT_RENDERABLE: str
    ERR_CONFIG_TEMPLATE_EXISTS: str
    ERR_EXTRACT_INVALID_REQUEST: str
    ERR_EXTRACT_MALFORMED: str
    ERR_EXTRACT_UNSUPPORTED: str
    ERR_EXTRACT_AMBIGUOUS: str
    ERR_EXTRACT_FORMAT_UNSUPPORTED: str
    ERR_EXTRACT_JSON_MALFORMED: str
    ERR_EXTRACT_JSON_DUPLICATE_KEY: str
    ERR_EXTRACT_JSON_PATH_MISSING: str
    ERR_EXTRACT_JSON_SHAPE_MISMATCH: str
    ERR_EXTRACT_JSON_VALUE_UNSUPPORTED: str
    ERR_EXTRACT_JSON_AMBIGUOUS: str
    ERR_EXTRACT_YAML_MALFORMED: str
    ERR_EXTRACT_YAML_DUPLICATE_KEY: str
    ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED: str
    ERR_EXTRACT_YAML_DOCUMENT_STREAM: str
    ERR_EXTRACT_YAML_PATH_MISSING: str
    ERR_EXTRACT_YAML_SHAPE_MISMATCH: str
    ERR_EXTRACT_YAML_VALUE_UNSUPPORTED: str
    ERR_EXTRACT_YAML_AMBIGUOUS: str
    ERR_EXTRACT_TOML_MALFORMED: str
    ERR_EXTRACT_TOML_DUPLICATE_KEY: str
    ERR_EXTRACT_TOML_PATH_MISSING: str
    ERR_EXTRACT_TOML_SHAPE_MISMATCH: str
    ERR_EXTRACT_TOML_VALUE_UNSUPPORTED: str
    ERR_EXTRACT_TOML_AMBIGUOUS: str
    WARN_EXTRACT_NOT_OBSERVED: str
    WARN_EXTRACT_LOW_CONFIDENCE: str


class VariableName:
    def __init__(self, value: str) -> None: ...
    def __str__(self) -> str: ...


class ProfileName:
    def __init__(self, value: str) -> None: ...
    def __str__(self) -> str: ...


class ConfiningRoot:
    def __init__(self, path: str | PathLike[str]) -> None: ...
    def confine(self, candidate: str | PathLike[str]) -> str: ...
    def __str__(self) -> str: ...


class ResolverPolicy:
    def __repr__(self) -> str: ...


class ComposeMode:
    @staticmethod
    def file(template_path: str | PathLike[str]) -> ComposeMode: ...
    @staticmethod
    def profile(kind: str, name: str | ProfileName) -> ComposeMode: ...
    @property
    def template_path(self) -> str | None: ...
    @property
    def kind(self) -> str | None: ...
    @property
    def name(self) -> str | None: ...


class ComposePolicy:
    def __init__(
        self,
        strict_undeclared_variables: bool = False,
        unknown_variable_policy: str = "ignore",
        max_include_depth: int = 32,
        allowed_roots: list[str | PathLike[str] | ConfiningRoot] | None = None,
        passes: list[PassConfig] | None = None,
    ) -> None: ...
    @property
    def strict_undeclared_variables(self) -> bool: ...
    @property
    def unknown_variable_policy(self) -> str: ...
    @property
    def max_include_depth(self) -> int: ...
    @property
    def allowed_roots(self) -> list[str]: ...
    @property
    def resolver_policy(self) -> ResolverPolicy: ...
    @property
    def passes(self) -> list[PassConfig]: ...


class PassConfig:
    def __init__(
        self,
        pass_number: int,
        required_variables: list[str | VariableName] | None = None,
        defaults: dict[str, Any] | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> None: ...
    @property
    def pass_number(self) -> int: ...
    @property
    def required_variables(self) -> list[VariableName]: ...
    @property
    def defaults(self) -> dict[str, Any]: ...
    @property
    def metadata(self) -> dict[str, Any]: ...


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
        runtime: str | None = None,
    ) -> None: ...
    @property
    def root(self) -> str: ...
    @property
    def runtime(self) -> str | None: ...
    @property
    def mode(self) -> ComposeMode: ...
    @property
    def policy(self) -> ComposePolicy: ...


class Diagnostic:
    @property
    def severity(self) -> str: ...
    @property
    def code(self) -> str: ...
    @property
    def message(self) -> str: ...
    @property
    def path(self) -> str | None: ...
    @property
    def line(self) -> int | None: ...
    @property
    def column(self) -> int | None: ...
    @property
    def include_chain(self) -> list[str]: ...


class ExtractionDiagnostic:
    @property
    def code(self) -> str: ...
    @property
    def kind(self) -> str: ...
    @property
    def message(self) -> str: ...
    @property
    def occurrence(self) -> int | None: ...


class ExtractionSource:
    @property
    def kind(self) -> str: ...
    @property
    def name(self) -> str | None: ...


class ExtractionPathSegment:
    @property
    def kind(self) -> str: ...
    @property
    def name(self) -> str: ...
    @property
    def ordinal(self) -> int | None: ...


class ExtractionOccurrence:
    @property
    def variable(self) -> str: ...
    @property
    def path(self) -> list[ExtractionPathSegment]: ...
    @property
    def source(self) -> ExtractionSource: ...
    @property
    def rendered_text(self) -> str | None: ...


class ExtractionReport:
    @property
    def values(self) -> dict[str, str]: ...
    @property
    def occurrences(self) -> list[ExtractionOccurrence]: ...
    @property
    def confidence(self) -> float: ...
    @property
    def diagnostics(self) -> list[ExtractionDiagnostic]: ...


class ResolveResult:
    @property
    def resolved_path(self) -> str: ...
    @property
    def attempted_paths(self) -> list[str]: ...
    @property
    def ambiguity_candidates(self) -> list[str]: ...


class ComposeResult:
    @property
    def rendered_text(self) -> str: ...
    @property
    def resolved_files(self) -> list[str]: ...
    @property
    def resolve_result(self) -> ResolveResult: ...
    @property
    def warnings(self) -> list[Diagnostic]: ...
    @property
    def variable_sources(self) -> dict[str, str]: ...


class ValidationReport:
    @property
    def ok(self) -> bool: ...
    @property
    def warnings(self) -> list[Diagnostic]: ...
    @property
    def errors(self) -> list[Diagnostic]: ...
    @property
    def resolve_result(self) -> ResolveResult: ...


class VerifyResult:
    @property
    def clean(self) -> bool: ...
    @property
    def resolved_template_path(self) -> str: ...
    @property
    def deployed_path(self) -> str: ...
    @property
    def rendered_text(self) -> str: ...
    @property
    def deployed_text(self) -> str: ...
    @property
    def diff(self) -> str | None: ...
    @property
    def warnings(self) -> list[Diagnostic]: ...


class NamedTemplateAsset:
    def __init__(self, template_name: str, template_text: str) -> None: ...
    @property
    def template_name(self) -> str: ...
    @property
    def template_text(self) -> str: ...


class LoadedTemplateRequest:
    def __init__(
        self,
        template_name: str,
        template_text: str,
        context: dict[str, Any],
        supporting_templates: list[NamedTemplateAsset] | None = None,
    ) -> None: ...


class RenderedArtifact:
    @property
    def rendered(self) -> str: ...
    @property
    def template_name(self) -> str: ...


class Frontmatter:
    @property
    def pass_number(self) -> int: ...
    @property
    def required_variables(self) -> list[VariableName]: ...
    @property
    def defaults(self) -> dict[str, Any]: ...
    @property
    def metadata(self) -> dict[str, Any]: ...
    @property
    def diagnostics(self) -> list[Diagnostic]: ...


class ParsedTemplate:
    @property
    def frontmatter(self) -> Frontmatter | None: ...
    @property
    def passes(self) -> list[Frontmatter]: ...
    @property
    def body(self) -> str: ...


class ExpandedTemplate:
    @property
    def text(self) -> str: ...
    @property
    def resolved_files(self) -> list[str]: ...
    @property
    # Legacy compatibility view: returns only the outermost frontmatter block
    # for each file. Use `frontmatter_passes` for the complete multi-pass data.
    def frontmatters(self) -> list[tuple[str, Frontmatter | None]]: ...
    @property
    def frontmatter_passes(self) -> list[tuple[str, list[Frontmatter]]]: ...
    @property
    def include_chains(self) -> dict[str, list[str]]: ...


class FrontmatterInitResult:
    @property
    def target_path(self) -> str: ...
    @property
    def frontmatter_text(self) -> str: ...
    @property
    def discovered_variables(self) -> list[VariableName]: ...
    @property
    def changed(self) -> bool: ...
    @property
    def would_change(self) -> bool: ...


class InitResult:
    @property
    def prompts_dir(self) -> str: ...
    @property
    def gitignore_updated(self) -> bool: ...
    @property
    def scanned_templates(self) -> list[str]: ...
    @property
    def recommendations(self) -> list[Diagnostic]: ...
    @property
    def validation_passed(self) -> bool: ...


class Renderer:
    def __init__(self) -> None: ...
    @classmethod
    def with_delimiters(cls, open: str, close: str) -> Renderer: ...
    def render(self, template: str, context: dict[str, Any]) -> str: ...
    def render_named(self, name: str, template: str, context: dict[str, Any]) -> str: ...


BUILTIN_VARIABLE_NAMES: list[str]


def compose(request: ComposeRequest) -> ComposeResult: ...
def compose_file(request: ComposeRequest) -> ComposeResult: ...
def validate(request: ComposeRequest) -> ValidationReport: ...
def verify(
    request: ComposeRequest,
    deployed_path: str | PathLike[str],
) -> VerifyResult: ...
def resolve_template_path(request: ComposeRequest) -> ResolveResult: ...
def resolve_profile(request: ComposeRequest) -> ResolveResult: ...
def render_template(template: str, context: dict[str, Any]) -> str: ...
def extract_variables(
    template: str,
    rendered: str,
    *,
    format: Literal["xml", "json", "yaml", "toml"] = "xml",
    include: list[str] | None = None,
    exclude: list[str] | None = None,
) -> ExtractionReport: ...


XmlPathSegment = ExtractionPathSegment
def render_loaded_template(request: LoadedTemplateRequest) -> RenderedArtifact: ...
def parse_template_document(input: str) -> ParsedTemplate: ...
# Expects fully resolved per-pass contexts and applies each pass header's
# frontmatter defaults beneath caller-supplied values. Use compose() for
# request/policy resolution, validation, and variable-source tracking.
def render_all(
    parsed: ParsedTemplate,
    contexts: list[tuple[int, dict[str, Any]]],
) -> str: ...
def expand_includes(
    template_path: str | PathLike[str],
    root: str | PathLike[str],
    policy: ComposePolicy | None = None,
) -> ExpandedTemplate: ...
def frontmatter_init(
    path: str | PathLike[str],
    force: bool = False,
    dry_run: bool = False,
) -> FrontmatterInitResult: ...
def init_workspace(root: str | PathLike[str], dry_run: bool = False) -> InitResult: ...
def validate_input_value(value: Any) -> None: ...
def input_value_from_yaml(input: str) -> Any: ...
def to_forward_slash(path: str | PathLike[str]) -> str: ...
def discover_tokens(text: str) -> list[VariableName]: ...
def discover_tokens_with_brace_count(
    text: str, brace_count: int
) -> list[VariableName]: ...
def discover_all_pass_tokens(parsed: ParsedTemplate) -> dict[int, list[VariableName]]: ...
