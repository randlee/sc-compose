---
id: FIX-O15
title: Public classified directive-inspection API for sc-composer
status: assigned
phase: Phase O
sprint_doc: docs/project-plan.md
---

# FIX-O15: Public classified directive-inspection API for sc-composer

## Context

atm-core (team-lead@atm-dev) is upgrading `atm-template-sc-compose` from
`sc-composer = 1.3.0` to `1.4.x`. They carry a temporary local stub
(`ScComposeTemplateComposer.inspections` / `from_fixture_inspections` in
`crates/atm-template-sc-compose/src/lib.rs`) standing in for real
directive-inspection output, tracked internally as
`AN1-FIXTURE-STUB-REPLACEMENT-001`. `sc-sha` 1.4.0 already covers their
content-hash need; the remaining gap is that `sc-composer` keeps native
directive parsing private and exposes no public API for classified
include/import/from-import span inspection. Filed as
[randlee/sc-compose#445](https://github.com/randlee/sc-compose/issues/445).
User decision (2026-08-13): pull this into the 1.4.1 release rather than
deferring it to a later release, since 1.4.1 is already the fix-forward
release atm-core is pinning to.

## Deliverable

Expose a public, read-only API on `sc-composer` that returns classified
spans for a parsed template's:

- `include` directives
- `import` directives
- `from ... import ...` directives

Each span should carry at minimum: directive kind, source byte/line range,
and the resolved target path/module expression as parsed (not yet
resolved against the filesystem). Do not leak internal parser types
(AST nodes, token structs) across the public boundary -- return a small,
stable, purpose-built public type (e.g. `DirectiveSpan` /
`ClassifiedDirective`) per `sc-composer`'s existing pattern of narrow
public surfaces (see how `template_scanner.rs`'s existing scan results
are exposed, if applicable, for precedent).

## Out of scope

- Resolving/following the directive targets (path confinement, existence
  checks) -- this is inspection only, not resolution.
- Any change to `bindings/python` or `sc-compose` CLI surfaces, unless a
  minimal export is needed for consistency -- confirm with team-lead
  before touching those crates.
- Redesigning `atm-core`'s stub itself -- that is their crate, not ours.

## Acceptance criteria

- New public function/type on `sc-composer`'s public surface (documented
  in `crates/sc-composer/src/lib.rs`'s public exports) returning classified
  spans for include/import/from-import directives for a given parsed
  template.
- Unit tests in `sc-composer` covering: a template with all three directive
  kinds, nested/mixed directives, a template with none (empty result, not
  an error), and at least one malformed-directive case (confirm it surfaces
  a diagnostic rather than panicking).
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --workspace` all clean.
- Update `docs/architecture.md` and `CHANGELOG.md` (`[1.4.1]` entry) to
  document the new public API.
- Reconcile the returned shape against atm-core's fixture-based stub
  input/output shape once team-lead relays it (see references) --- flag
  to team-lead if the shapes are incompatible before finalizing, don't
  silently diverge.

## References

- https://github.com/randlee/sc-compose/issues/445
- `docs/phase-O/evidence/o5-release-corpus.md` (1.4.1 release waiver context)
- `CLAUDE.md` Boundary Rules 1-2 (`sc-composer` must remain a pure library)
