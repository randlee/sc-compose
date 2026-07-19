# ADR-0004: YAML `pass` Field Is the Authoritative Pass Selector

## Status

Accepted

## Context

Prototype parsing in `prototype/multipass/parser.py` treats the YAML `pass`
field as the source of truth for each header's pass number and defaults the
field to `1` when absent or invalid. Filename suffixes such as `.2.j2` are
helpful human signals, but the current design does not rely on them for
correctness.

## Decision

- The YAML `pass` field is authoritative for pass selection.
- Missing or invalid `pass` values normalize to pass 1.
- Filename conventions remain optional UX aids and do not override header
  semantics.

## Consequences

- Template semantics travel with the file contents, not naming conventions.
- Parser and validation logic can reject or normalize bad header data without
  coupling correctness to resolver naming rules.
- The Phase D CLI can accept arbitrary template paths without inventing a
  second pass-number source of truth.
