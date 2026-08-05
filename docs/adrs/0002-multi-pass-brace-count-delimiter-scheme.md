# ADR-0002: Multi-Pass Brace-Count Delimiter Scheme

## Status

Accepted

## Context

Phase D introduces stacked multi-pass rendering where one template body is
rendered multiple times from outermost pass to innermost pass. The prototype
reference implementation in `prototype/multipass/renderer.py` and
`prototype/multipass/types.py` already encodes a delimiter rule:

- pass 1 uses `{{ ... }}`
- pass 2 uses `{{{ ... }}}`
- pass 3 uses `{{{{ ... }}}}`

The production Rust design needs one stable rule for delimiter construction so
parsing, discovery, rendering, verify, and template-init all agree.

## Decision

- Pass `N` uses `N + 1` braces for variable delimiters.
- Statement delimiters remain `{% ... %}` across all passes.
- The delimiter rule is computed from pass number, not inferred from filename.
- The production Rust implementation should follow the prototype behavior
  rather than re-deriving a different delimiter scheme.

## Consequences

- `PassConfig`, `Frontmatter`, parser, renderer, token discovery, and
  template-init all share one delimiter rule.
- Multi-pass examples and tests can express pass behavior mechanically from
  pass number.
- Reviewers can verify Phase D behavior against the committed prototype and
  ADR instead of plan prose alone.
