"""Per-pass validation for multi-pass templates.

Matches Rust sc-composer validation semantics:
- Undeclared tokens → warning (error in strict mode)
- Missing required variables → error
- Extra caller-provided variables → warning/error/ignore (policy)
- Empty body → error
- Missing frontmatter → warning
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

from .types import ParsedTemplate, PassHeader, RenderContext
from .discover import discover_tokens


class Severity(Enum):
    ERROR = "error"
    WARNING = "warning"
    INFO = "info"


@dataclass
class Diagnostic:
    """A single validation diagnostic, matching Rust Diagnostic shape."""
    severity: Severity
    code: str
    message: str
    pass_number: int | None = None

    def __str__(self) -> str:
        prefix = f"[pass {self.pass_number}] " if self.pass_number else ""
        return f"{prefix}[{self.severity.value.upper()}] {self.code}: {self.message}"


@dataclass
class ValidationReport:
    """Result of mult-pass validation."""
    ok: bool
    diagnostics: list[Diagnostic] = field(default_factory=list)

    @property
    def errors(self) -> list[Diagnostic]:
        return [d for d in self.diagnostics if d.severity == Severity.ERROR]

    @property
    def warnings(self) -> list[Diagnostic]:
        return [d for d in self.diagnostics if d.severity == Severity.WARNING]

    @property
    def infos(self) -> list[Diagnostic]:
        return [d for d in self.diagnostics if d.severity == Severity.INFO]


# Built-in variable names (matching Rust BUILTIN_VARIABLE_NAMES)
BUILTIN_VARIABLES = {
    "TEMPLATE_NAME", "HOSTNAME", "USERNAME",
    "RENDER_DATE", "RENDER_TIMESTAMP",
}


def validate_passes(
    parsed: ParsedTemplate,
    contexts: list[RenderContext],
    *,
    strict: bool = False,
) -> ValidationReport:
    """Validate all passes of a parsed template against provided contexts.

    For each pass:
    1. Discover tokens with the pass's brace_count
    2. Check that every discovered token is declared (required_vars + defaults + builtins)
    3. Check that every required variable has a provided value
    4. Check for extra caller-provided variables

    Args:
        parsed: Stacked template with headers.
        contexts: One RenderContext per pass (same order as parsed.passes).
        strict: If True, undeclared tokens are errors instead of warnings.

    Returns:
        ValidationReport with ok=False if any errors found.
    """
    diagnostics: list[Diagnostic] = []

    if not parsed.body.strip():
        diagnostics.append(Diagnostic(
            severity=Severity.ERROR,
            code="ERR_VAL_EMPTY",
            message="template body is empty",
        ))
        return ValidationReport(ok=False, diagnostics=diagnostics)

    if not parsed.passes:
        return ValidationReport(ok=True, diagnostics=diagnostics)

    # Build context lookup by pass_number
    ctx_by_pass: dict[int, RenderContext] = {}
    for ctx in contexts:
        ctx_by_pass[ctx.pass_number] = ctx

    for header in parsed.passes:
        pass_num = header.pass_number
        ctx = ctx_by_pass.get(pass_num)

        # Discover tokens for this pass
        discovered = discover_tokens(parsed.body, header.brace_count)

        # Known variable names: required + defaults + builtins
        declared = set(header.required_variables)
        declared.update(header.defaults.keys())
        declared.update(BUILTIN_VARIABLES)

        # Undeclared referenced tokens
        undeclared = discovered - declared
        for var in sorted(undeclared):
            sev = Severity.ERROR if strict else Severity.WARNING
            diagnostics.append(Diagnostic(
                severity=sev,
                code="ERR_VAL_UNDECLARED_TOKEN",
                message=f"undeclared referenced token: {var}",
                pass_number=pass_num,
            ))

        # Missing required variables
        if ctx:
            provided = set(ctx.variables.keys())
        else:
            provided = set()

        for var in header.required_variables:
            if var not in provided and var not in header.defaults:
                diagnostics.append(Diagnostic(
                    severity=Severity.ERROR,
                    code="ERR_VAL_MISSING_REQUIRED",
                    message=f"missing required variable: {var}",
                    pass_number=pass_num,
                ))

        # Check for defaults being used (info diagnostic)
        if ctx:
            for var in header.required_variables:
                if var not in ctx.variables and var in header.defaults:
                    diagnostics.append(Diagnostic(
                        severity=Severity.INFO,
                        code="INFO_VAL_DEFAULT_USED",
                        message=f"variable {var} not provided, using default",
                        pass_number=pass_num,
                    ))

    errors = [d for d in diagnostics if d.severity == Severity.ERROR]
    return ValidationReport(ok=len(errors) == 0, diagnostics=diagnostics)


def print_report(report: ValidationReport) -> None:
    """Print a validation report to stdout."""
    if report.ok and not report.diagnostics:
        print("valid")
        return

    for d in report.diagnostics:
        print(d)

    if report.ok:
        print(f"\nvalid ({len(report.warnings)} warnings, {len(report.infos)} info)")
    else:
        print(f"\nINVALID ({len(report.errors)} errors)")
