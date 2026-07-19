# ADR-0005: Exact-Match Delimiter Scanning Across Passes

## Status

Accepted

## Context

The prototype discovery logic in `prototype/multipass/discover.py` and the
render protection logic in `prototype/multipass/renderer.py` both rely on one
core rule: a delimiter for pass `N` must not match inside the longer delimiter
for pass `N + 1`.

Without that rule:

- pass-1 discovery would incorrectly match `{{` inside `{{{`
- pass-1 rendering could consume pass-2 placeholders early
- multi-pass validation would report the wrong variables per pass

## Decision

- Delimiter scanning is exact-match only.
- When scanning for `"{".repeat(N)`, the implementation must reject matches
  whose next character is also `{`.
- The symmetric rule applies to close delimiters.
- Before rendering pass `N`, higher-brace placeholders are wrapped in
  `{% raw %}...{% endraw %}` exactly as shown in the prototype.
- Statement delimiters `{% ... %}` remain unchanged across passes.

## Consequences

- Token discovery and rendering share one correctness rule.
- Mixed-brace templates behave predictably in both validation and rendering.
- The Rust port has a concrete behavioral oracle in the committed prototype.
