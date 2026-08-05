"""Native sc_compose binding integration for multi-pass prototype.

Uses the real sc_compose maturin bindings (built from bindings/python/).
Provides typed wrappers that match the prototype's expected interface.

Key bindings used:
- sc_compose.Renderer.with_delimiters(open, close) → renderer
- sc_compose.parse_template_document(text) → ParsedTemplate (single frontmatter)
- sc_compose.discover_tokens(text) → list[VariableName]
- sc_compose.validate(request) → ValidationReport
- sc_compose.compose(request) → ComposeResult
- sc_compose.frontmatter_init(path, force, dry_run) → FrontmatterInitResult
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

import sc_compose

# ── Re-export native types with Python-friendly wrappers ──────────────────


def render_template_native(
    template: str,
    context: dict[str, Any],
    *,
    open_delim: str = "{{",
    close_delim: str = "}}",
) -> str:
    """Render a template using native sc_compose.Renderer.

    This is the KEY function that replaces jinja2.Environment().
    """
    renderer = sc_compose.Renderer.with_delimiters(open_delim, close_delim)
    return renderer.render(template, context)


def parse_template_document_native(text: str) -> sc_compose.ParsedTemplate:
    """Parse a template document using native sc_compose parser.

    Returns a native ParsedTemplate with optional single frontmatter.
    For multi-pass stacked headers, use the prototype's parser directly.
    """
    return sc_compose.parse_template_document(text)


def compose_native(request: dict) -> ComposeResult:
    """Call sc_compose.compose() with a ComposeRequest."""
    req = sc_compose.ComposeRequest(**request)
    result = sc_compose.compose(req)
    return ComposeResult(
        rendered_text=result.rendered_text,
        resolved_files=list(result.resolved_files),
        warnings=[dict(w) for w in result.warnings] if hasattr(result, 'warnings') else [],
    )


def validate_native(request: dict) -> ValidationReport:
    """Call sc_compose.validate() with a ComposeRequest."""
    req = sc_compose.ComposeRequest(**request)
    result = sc_compose.validate(req)
    return ValidationReport(
        ok=result.ok,
        warnings=[dict(w) for w in result.warnings] if hasattr(result, 'warnings') else [],
        errors=[dict(e) for e in result.errors] if hasattr(result, 'errors') else [],
    )


def frontmatter_init_native(
    path: str, *, force: bool = False, dry_run: bool = False
) -> FrontmatterInitResult:
    """Call sc_compose.frontmatter_init()."""
    result = sc_compose.frontmatter_init(path, force=force, dry_run=dry_run)
    return FrontmatterInitResult(
        target_path=result.target_path,
        frontmatter_text=result.frontmatter_text,
        discovered_variables=[str(v) for v in result.discovered_variables],
        changed=result.changed,
        would_change=result.would_change,
    )


def discover_tokens_native(text: str) -> list[str]:
    """Call sc_compose.discover_tokens() and return string names."""
    tokens = sc_compose.discover_tokens(text)
    return [str(t) for t in tokens]


# ── Python-compatible wrapper types ───────────────────────────────────────


@dataclass
class Frontmatter:
    """Mirrors Rust Frontmatter struct (Python-compatible)."""

    pass_number: int = 1
    required_variables: list[str] = field(default_factory=list)
    defaults: dict[str, Any] = field(default_factory=dict)


@dataclass
class ParsedTemplate:
    """Mirrors Rust ParsedTemplate struct (Python-compatible)."""

    passes: list[Frontmatter] = field(default_factory=list)
    body: str = ""


@dataclass
class ComposeResult:
    """Mirrors Rust ComposeResult struct."""

    rendered_text: str = ""
    resolved_files: list[str] = field(default_factory=list)
    warnings: list[dict] = field(default_factory=list)


@dataclass
class ValidationReport:
    """Mirrors Rust ValidationReport struct."""

    ok: bool = True
    warnings: list[dict] = field(default_factory=list)
    errors: list[dict] = field(default_factory=list)


@dataclass
class FrontmatterInitResult:
    """Mirrors Rust FrontmatterInitResult struct."""

    target_path: str = ""
    frontmatter_text: str = ""
    discovered_variables: list[str] = field(default_factory=list)
    changed: bool = False
    would_change: bool = False


# ── Multi-pass renderer using native sc_compose ───────────────────────────


def render_pass_with_sc_compose(
    parsed: ParsedTemplate,
    pass_vars: dict[str, Any],
) -> tuple[str, ParsedTemplate]:
    """Render one pass using native sc_compose.Renderer.with_delimiters().

    Uses the real maturin bindings — no more jinja2.Environment().

    Args:
        parsed: Current template state with remaining passes.
        pass_vars: Variables for the current pass.

    Returns:
        (rendered_body, remaining_template)
    """
    if not parsed.passes:
        raise ValueError("no passes remaining")

    header = parsed.passes[0]
    brace_count = header.pass_number + 1
    open_delim = "{" * brace_count
    close_delim = "}" * brace_count

    # Use native sc_compose.Renderer — the single line that replaces ~15
    # lines of jinja2 setup.
    rendered = render_template_native(
        parsed.body,
        {**header.defaults, **pass_vars},
        open_delim=open_delim,
        close_delim=close_delim,
    )

    remaining_passes = parsed.passes[1:]
    remaining = ParsedTemplate(passes=remaining_passes, body=rendered)
    return rendered, remaining


def render_all_with_sc_compose(
    parsed: ParsedTemplate,
    pass_contexts: list[dict[str, Any]],
) -> str:
    """Render all passes using native sc_compose.Renderer."""
    current = parsed
    output = ""
    for ctx_vars in pass_contexts:
        output, current = render_pass_with_sc_compose(current, ctx_vars)
    return output


# ── Migration guide: what changed ─────────────────────────────────────────

MIGRATION_TABLE = """
┌─────────────────────────┬──────────────────────────────────────┬──────────────────────────────────────┐
│ Module                  │ Current (pure Python)                │ After C.1 (sc_compose bindings)      │
├─────────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
│ multipass/renderer.py   │ jinja2.Environment(                  │ sc_compose.Renderer.with_delimiters( │
│                         │   variable_start_string="{{{", ...)  │   "{{{", "}}}")                      │
│                         │ ~20 lines of env setup               │ 1 line                              │
├─────────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
│ multipass/discover.py   │ Pure Python regex scanner            │ sc_compose.discover_tokens() for     │
│                         │ for all brace counts                 │ double-brace; Python fallback for    │
│                         │                                      │ triple+ braces                       │
├─────────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
│ multipass/parser.py     │ _parse_header() with yaml.safe_load  │ sc_compose.parse_template_document() │
│                         │ manual ---...--- loop                │ for single-pass; custom for multi    │
├─────────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
│ multipass/validate.py   │ custom Diagnostic + check logic      │ sc_compose.validate(request)         │
│                         │ ~100 lines                           │ returns ValidationReport             │
├─────────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
│ multipass/template_init │ str.replace() + yaml.dump headers    │ sc_compose.frontmatter_init(         │
│                         │ ~80 lines                            │   path, force, dry_run)              │
├─────────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
│ multipass/verify.py     │ render_all() + difflib.unified_diff  │ sc_compose.compose() + diff          │
└─────────────────────────┴──────────────────────────────────────┴──────────────────────────────────────┘

Status (v1.2.0):
  ✅ Renderer.with_delimiters() — DONE, works for all brace counts
  ✅ discover_tokens() — DONE, handles double-brace natively
  ✅ parse_template_document() — DONE, single frontmatter only
  ✅ compose() / validate() / frontmatter_init() — DONE
  ⚠️  Multi-pass stacked headers — still Python-driven (no Rust support yet)
  ⚠️  discover_tokens for triple+ braces — Python fallback (native only does {{ }})
"""  # noqa: E501