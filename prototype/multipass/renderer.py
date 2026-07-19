"""Multi-pass template renderer using native sc_compose bindings.

Renders stacked-header templates one pass at a time using sc_compose.Renderer
with configurable delimiter syntax per pass.

Pass N uses {N+1} braces: pass 1 → {{ }}, pass 2 → {{{ }}}, etc.
Block delimiters {% %} remain unchanged across all passes.

Uses sc_compose maturin bindings for rendering (replaces jinja2.Environment).
"""

from __future__ import annotations

import sc_compose

from .types import ParsedTemplate, RenderContext


def _protect_higher_braces(text: str, brace_count: int) -> str:
    """Wrap higher-brace-count variables in {% raw %} to prevent the native
    renderer from matching {N} inside {N+1}.

    E.g., when brace_count=2 (pass 1), escape {{{ }}} so {{ }} matching
    doesn't accidentally match inside {{{.
    """
    higher_brace = brace_count + 1
    open_delim = "{" * higher_brace
    close_delim = "}" * higher_brace

    if open_delim not in text:
        return text

    # Find each {N+1}...{N+1} block and wrap in raw
    result: list[str] = []
    cursor = 0
    while True:
        idx = text.find(open_delim, cursor)
        if idx == -1:
            result.append(text[cursor:])
            break
        end_idx = text.find(close_delim, idx + higher_brace)
        if end_idx == -1:
            result.append(text[cursor:])
            break
        # Include text before match, wrap the matched block
        result.append(text[cursor:idx])
        result.append("{% raw %}")
        result.append(text[idx : end_idx + higher_brace])
        result.append("{% endraw %}")
        cursor = end_idx + higher_brace

    return "".join(result)


def render_pass(
    parsed: ParsedTemplate,
    ctx: RenderContext,
) -> tuple[str, ParsedTemplate]:
    """Render a single pass, consuming its header.

    Uses sc_compose.Renderer.with_delimiters() for native rendering.

    Returns:
        (rendered_body, remaining_template)
        remaining_template has one fewer header. If this was the last pass,
        remaining_template has passes=[] and body=rendered_output (no headers).
    """
    if not parsed.passes:
        raise ValueError("no passes remaining")

    header = parsed.passes[0]
    if header.pass_number != ctx.pass_number:
        raise ValueError(
            f"header declares pass {header.pass_number} but context is "
            f"pass {ctx.pass_number}"
        )

    # Build native renderer with custom delimiters
    brace_count = header.brace_count
    open_delim = "{" * brace_count
    close_delim = "}" * brace_count
    renderer = sc_compose.Renderer.with_delimiters(open_delim, close_delim)

    # Merge defaults + provided variables (provided wins)
    render_vars = dict(header.defaults)
    render_vars.update(ctx.variables)

    # Protect higher-brace-count variables from being parsed.
    # When brace_count=2 (pass 1), {{{ }}} must not be matched as {{.
    # When brace_count=3 (pass 2), {{{{ }}}} must not be matched as {{{.
    body = _protect_higher_braces(parsed.body, brace_count)

    # Render the body using native sc_compose renderer
    rendered_body = renderer.render(body, render_vars)

    # Remaining template: strip consumed header
    remaining_passes = parsed.passes[1:]

    if remaining_passes:
        remaining = ParsedTemplate(passes=remaining_passes, body=rendered_body)
    else:
        remaining = ParsedTemplate(passes=[], body=rendered_body)

    return rendered_body, remaining


def render_all(
    parsed: ParsedTemplate,
    contexts: list[RenderContext],
) -> str:
    """Render all passes in sequence using native sc_compose renderer.

    Args:
        parsed: Stacked template to render.
        contexts: One RenderContext per pass, in order (outermost first).

    Returns:
        Fully rendered output text (no headers, no templates remaining).
    """
    if len(contexts) != len(parsed.passes):
        raise ValueError(
            f"expected {len(parsed.passes)} contexts, got {len(contexts)}"
        )

    current = parsed
    current_text = ""

    for ctx in contexts:
        if current.passes[0].pass_number != ctx.pass_number:
            raise ValueError(
                f"context pass {ctx.pass_number} doesn't match "
                f"header pass {current.passes[0].pass_number}"
            )
        current_text, current = render_pass(current, ctx)

    return current_text


def _reconstruct(passes: list, body: str) -> str:
    """Reconstruct template text from headers + body."""
    import yaml

    lines: list[str] = []
    for header in passes:
        data: dict = {}
        if header.pass_number > 1:
            data["pass"] = header.pass_number
        if header.required_variables:
            data["required_variables"] = header.required_variables
        if header.defaults:
            data["defaults"] = header.defaults
        if header.metadata:
            data["metadata"] = header.metadata

        lines.append("---")
        if data:
            lines.append(yaml.dump(data, default_flow_style=False).rstrip())
        lines.append("---")

    lines.append(body)
    return "\n".join(lines) + "\n"