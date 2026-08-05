"""Brace-count aware token discovery using native sc_compose bindings.

Scans template text for Jinja2 variable references using configurable
brace-count delimiters.

Pass N uses {N+1} braces: pass 1 → {{ }}, pass 2 → {{{ }}}, pass 3 → {{{{ }}}}
Block delimiters {% %} remain unchanged across all passes.

For standard double-brace ({{ }}) discovery, delegates to the native
sc_compose.discover_tokens(). For non-standard brace counts (triple+ braces),
falls back to the pure-Python scanner since the native function only scans
for double-brace patterns.
"""

from __future__ import annotations

import re

import sc_compose

from .types import ParsedTemplate

# Keywords that appear inside Jinja2 expressions but are not variables
_JINJA_KEYWORDS: set[str] = {
    "if",
    "else",
    "elif",
    "endif",
    "for",
    "endfor",
    "in",
    "set",
    "true",
    "false",
    "none",
    "not",
    "and",
    "or",
    "block",
    "endblock",
    "macro",
    "endmacro",
    "filter",
    "endfilter",
    "is",
    "defined",
    "loop",
}


def discover_tokens(text: str, brace_count: int) -> set[str]:
    """Find all variable references in text with a given brace count.

    Uses native sc_compose.discover_tokens() for brace_count=2.
    Falls back to pure-Python scanner for other brace counts.

    Args:
        text: Template body text to scan.
        brace_count: Number of braces for variable delimiters.
                     brace_count=2 → {{ var }}, brace_count=3 → {{{ var }}}

    Returns:
        Set of variable names referenced in the text.
    """
    if brace_count < 2:
        raise ValueError(f"brace_count must be >= 2, got {brace_count}")

    # Use native binding for standard double-brace templates without
    # higher-brace-count delimiters (which would confuse the native scanner).
    if brace_count == 2:
        # Check for higher-brace-count delimiters in the text
        has_higher = False
        for bc in range(3, 6):  # check for {{{, {{{{, {{{{{ 
            if "{" * bc in text:
                has_higher = True
                break
        if not has_higher:
            native_tokens = sc_compose.discover_tokens(text)
            return {str(t) for t in native_tokens}

    # Fall back to pure-Python scanner for non-standard brace counts
    # or when higher-brace-count delimiters would confuse the native scanner.
    return _discover_tokens_py(text, brace_count)


def _discover_tokens_py(text: str, brace_count: int) -> set[str]:
    """Pure-Python token discovery for non-standard brace counts."""
    open_delim = "{" * brace_count
    close_delim = "}" * brace_count

    tokens: set[str] = set()
    scopes: list[LoopScope] = []
    cursor = text

    while True:
        # Find next delimiter: variable expression or statement block
        var_pos = _find_delim(cursor, "{" * brace_count)
        stmt_pos = cursor.find("{%")
        comment_pos = cursor.find("{#")

        # Skip comments
        if comment_pos != -1 and (var_pos == -1 or comment_pos < var_pos):
            comment_end = cursor.find("#}", comment_pos)
            if comment_end != -1:
                cursor = cursor[comment_end + 2 :]
                continue

        if var_pos == -1 and stmt_pos == -1:
            break

        if stmt_pos != -1 and (var_pos == -1 or stmt_pos < var_pos):
            # Statement block
            stmt_end = cursor.find("%}", stmt_pos)
            if stmt_end == -1:
                break
            expression = cursor[stmt_pos + 2 : stmt_end].strip()
            if expression.startswith("for "):
                scope = _parse_for_scope(expression, tokens)
                if scope:
                    scopes.append(scope)
            elif expression.startswith("endfor"):
                if scopes:
                    scopes.pop()
            else:
                _collect_identifiers(expression, scopes, tokens)
            cursor = cursor[stmt_end + 2 :]
        else:
            # Variable expression
            after_open = var_pos + brace_count
            var_end = _find_close_delim(cursor[after_open:], close_delim)
            if var_end == -1:
                break
            expression = cursor[after_open : after_open + var_end].strip()
            _collect_identifiers(expression, scopes, tokens)
            cursor = cursor[after_open + var_end + brace_count :]

    return tokens


def discover_all_tokens(parsed: ParsedTemplate) -> dict[int, set[str]]:
    """Discover tokens for each pass's brace count in a parsed template.

    Returns:
        Dict mapping pass_number → set of variable names found.
    """
    result: dict[int, set[str]] = {}
    for header in parsed.passes:
        result[header.pass_number] = discover_tokens(
            parsed.body, header.brace_count
        )
    return result


class LoopScope:
    """Tracks bound loop variable names so they aren't flagged as references."""

    def __init__(self, bound_names: set[str]):
        self.bound_names = bound_names


def _find_delim(text: str, delim: str) -> int:
    """Find delim in text where it is NOT followed by the same character.

    For brace_count=2: find `{{` that is NOT part of `{{{`.
    For brace_count=3: find `{{{` that is NOT part of `{{{{`.
    """
    pos = 0
    while True:
        idx = text.find(delim, pos)
        if idx == -1:
            return -1
        # Check if this is actually a longer delimiter (e.g., `{{{` containing `{{`)
        after = idx + len(delim)
        if after < len(text) and text[after] == delim[0]:
            # Skip past this match — it's part of a longer delimiter
            pos = after
            continue
        return idx


def _find_close_delim(text: str, delim: str) -> int:
    """Like _find_delim but for close delimiters.

    For brace_count=2: find `}}` that is NOT part of `}}}`.
    """
    pos = 0
    while True:
        idx = text.find(delim, pos)
        if idx == -1:
            return -1
        after = idx + len(delim)
        if after < len(text) and text[after] == delim[0]:
            pos = after
            continue
        return idx


_IDENTIFIER_RE = re.compile(r"[a-zA-Z_][a-zA-Z0-9_.-]*")


def _parse_for_scope(expression: str, tokens: set[str]) -> LoopScope | None:
    """Parse a {% for x in iterable %} statement, return bound names."""
    trimmed = expression.strip()
    remainder = trimmed.removeprefix("for ")
    if " in " not in remainder:
        return None
    binding, iterable = remainder.split(" in ", 1)
    _collect_identifiers(iterable, [], tokens)

    bound_names: set[str] = set()
    for candidate in binding.split(","):
        name = candidate.strip().strip("()").split(".")[0]
        if name:
            bound_names.add(name)
    return LoopScope(bound_names) if bound_names else None


def _collect_identifiers(
    expression: str, scopes: list[LoopScope], tokens: set[str]
) -> None:
    """Extract variable identifiers from a Jinja2 expression."""
    all_bound: set[str] = set()
    for scope in scopes:
        all_bound.update(scope.bound_names)

    for match in _IDENTIFIER_RE.finditer(expression):
        name = match.group(0)
        if name in _JINJA_KEYWORDS:
            continue
        # Skip loop-bound variables (e.g., 'item' in 'for item in items')
        root = name.split(".")[0]
        if root in all_bound:
            continue
        tokens.add(name)