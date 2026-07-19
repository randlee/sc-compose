# ADR-0006: Longest-Match-First Template-Init Replacement

## Status

Accepted

## Context

`prototype/multipass/template_init.py` already implements a longest-match-first
replacement strategy across all passes before generating stacked headers. That
behavior prevents substring collisions such as replacing `wyvern` before
`/home/wyvern/worktrees/wyvern`.

Phase D needs the same rule documented formally so the Rust implementation
does not regress into order-dependent partial substitutions.

## Decision

- `template-init` collects every requested replacement across all passes.
- Replacements are sorted by concrete value length descending before
  substitution.
- The production implementation should preserve that prototype behavior.

## Consequences

- Multi-pass conversion remains deterministic even when one value is a
  substring of another.
- Template-init test cases can assert output shape without depending on input
  declaration order.
- The Rust implementation has a straightforward, prototype-backed algorithm to
  port.
