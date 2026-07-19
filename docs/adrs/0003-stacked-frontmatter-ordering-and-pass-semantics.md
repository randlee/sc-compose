# ADR-0003: Stacked Frontmatter Ordering and Pass Semantics

## Status

Accepted

## Context

The committed prototype in `prototype/multipass/parser.py` parses one or more
stacked YAML frontmatter documents before the template body. Those headers are
retained in the same outer-to-inner order they appear in the file and drive
the order of rendering contexts consumed by `render_all()`.

Phase D needs that ordering and shape documented as architecture, not only as
prototype code or sprint examples.

## Decision

- A parsed multi-pass template is represented as `ParsedTemplate { passes,
  body }`, where `passes` is ordered outer-to-inner.
- Empty headers are valid and normalize to pass 1 with empty declarations.
- `---` and `...` are both accepted as YAML document terminators.
- Rendering consumes headers in the same order they were parsed: the first
  header is rendered first, and each rendered body is fed into the next pass.

## Consequences

- Parser, renderer, template-init, and verify share a consistent pass order.
- Multi-pass validation can compare contexts to headers by position and
  declared pass number.
- Documentation can state a concrete compatibility shape for the Rust port.
