# ADR-0008: ParsedTemplate Compatibility and Prototype Reference Authority

## Status

Accepted

## Context

Phase D changed the parsed template shape from one optional frontmatter block
to stacked passes. At the time this ADR was written, the pre-Phase-D Rust code
in `crates/sc-composer/src/frontmatter/model.rs` defined:

- a private `frontmatter: Option<Frontmatter>` field on `ParsedTemplate`
- a public `frontmatter() -> Option<&Frontmatter>` accessor

The current model keeps the compatibility accessor while storing the full
stacked-pass shape in its private `passes: Vec<Frontmatter>` field.

That means the real compatibility question is not public struct-field access;
it is the accessor semantics when a template contains multiple headers.

Separately, the previously untracked `prototype/multipass/` directory is the
only existing, tested implementation of the full multi-pass behavior. The user
and team-lead have directed that it must be committed into the plan PR and
treated as the reference implementation for D.1-D.4 behavior.

## Decision

- The committed `prototype/multipass/` directory is the Phase D reference
  implementation and reviewable behavior oracle for parser, discovery,
  renderer, template-init, and verify semantics.
- The Rust port should follow that prototype behavior closely rather than
  re-deriving semantics from sprint prose alone.
- DD-007 is defined around public accessor behavior, not public struct-field
  access.
- The compatibility path for `ParsedTemplate::frontmatter()` is:
  - single-header templates preserve current semantics
  - stacked-header templates define the accessor as returning the first
    (outermost) pass for backward-compatible callers, while the new `passes`
    surface carries the full multi-pass shape
- Writer/converter paths that collapse a template back to an effectively
  single-pass file omit `pass: 1` so the output remains compatible with the
  shipped `1.2.x` format.

## Consequences

- Reviewers can verify Phase D decisions against committed code and tests.
- Sprint docs must not claim that public field access is the breaking-change
  vector, because the field is already private.
- The Rust implementation must document the accessor semantic extension
  explicitly when D.1 lands.
