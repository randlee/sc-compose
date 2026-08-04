---
id: J.4
title: Frontmatter Parser and Normalizer Split
phase: J
status: complete
branch: sprint/j-4-frontmatter-parser-split
worktree: ../sc-compose-worktrees/sprint/j-4-frontmatter-parser-split
target: integrate/phase-j
---

# Sprint J.4 — Frontmatter Parser and Normalizer Split

## Purpose

Reduce `crates/sc-composer/src/frontmatter.rs`'s hot-spot risk (Repowise
score 3.45, issue #212) by separating the raw YAML schema, delimiter
scanner, stacked-header parser, section normalization
(`required`/`default`/`input_defaults`/`metadata`), diagnostic construction,
and explicit-pass validation into internal modules, while retaining the
existing `Frontmatter`, `ParsedTemplate`, and `parse_template_document`
public API unchanged. This is the highest fan-out sprint in Phase J:
`Frontmatter`/`ParsedTemplate` are consumed by `include.rs`, `composer.rs`,
`discovery.rs`, `validation.rs`, `frontmatter_init.rs`, every format
extraction adapter, and `sc-compose` command code.

## Dependencies and exact targets

- `crates/sc-composer/src/frontmatter.rs:1-368` (public model types, raw YAML schema,
  delimiter scanner, stacked-header parser, `normalize_frontmatter` and its
  default-section/variable-name/value conversion, diagnostic construction,
  explicit-pass validation);
- the public API surface: `Frontmatter`, `ParsedTemplate`,
  `parse_template_document` — must not change signature, field visibility,
  or `lib.rs` export path;
- J.2 and J.3's characterization coverage of `validation.rs`, since
  `Frontmatter`/`ParsedTemplate` are validation's primary inputs and this
  sprint must not regress that seam indirectly.

Depends on J.2 and J.3 (both must land first, with their characterization
suites passing) — this sprint must not start until that coverage exists.

## Deliverables

- Split `frontmatter.rs` internally into: a model module (raw YAML schema
  types), a parser module (delimiter/stacked-header scanning), and a
  normalizer module (default-section merging, variable-name/value
  conversion, explicit-pass validation, diagnostic construction) — exact
  module names are comp's implementation call as long as `lib.rs` exports
  are preserved.
- Preserve `Frontmatter`, `ParsedTemplate`, and `parse_template_document` as
  the sole public entry points; every consumer (`include.rs`, `composer.rs`,
  `discovery.rs`, `validation.rs`, `frontmatter_init.rs`, all extraction
  adapters, `sc-compose` command code) requires no call-site changes.
- Add characterization tests, before moving any code, for: delimiter
  variants (`---`, BOM-prefixed, stacked passes), default-section precedence
  (`default` vs `input_defaults`), duplicate explicit-pass declarations, and
  malformed YAML — covering current (pre-move) behavior exactly.
- Run the full cross-surface validation this sprint's fan-out requires: the
  complete extraction adapter suite
  (`crates/sc-composer/tests/extract_integration.rs` and per-format CLI
  tests) plus the full `validation.rs` suite (moved by J.3), unchanged.

## Planned internal seam

The split is private and keeps the existing `frontmatter` module as the
ownership boundary:

```rust
mod model;       // Frontmatter, ParsedTemplate, and raw YAML shapes
mod parser;      // delimiter and stacked-header scanning
mod normalizer;  // section conversion, diagnostics, and pass validation

pub use model::{Frontmatter, ParsedTemplate};
pub use parser::parse_template_document;
```

The exact layout may differ, but consumers continue to import only the
existing `sc_composer` exports and no extraction adapter is edited.

## Acceptance criteria

- `Frontmatter`, `ParsedTemplate`, and `parse_template_document`'s public
  signatures and `lib.rs` export paths are unchanged.
- Every consumer crate/module (`include.rs`, `composer.rs`, `discovery.rs`,
  `validation.rs`, `frontmatter_init.rs`, all `extract/*` adapters,
  `sc-compose` command code, `bindings/python`) compiles and passes its
  existing tests with zero call-site changes.
- Delimiter, stacked-pass, default-precedence, duplicate-explicit-pass, and
  malformed-YAML behavior is unchanged for every existing fixture.
- The full extraction adapter suite and the full (J.3-relocated)
  `validation.rs` suite pass unchanged.
- NLOC evidence (baseline `8eb239e` → integration tip `3703035`) uses
  nonblank, non-comment lines before the first `#[cfg(test)]` marker for
  production and counts test lines separately: `frontmatter.rs` is 298
  production / 113 test NLOC before the split; `frontmatter/` is 310
  production / 125 moved+added test NLOC after the split, with
  `normalizer.rs` as the largest production module at 116 NLOC.

## Required validation

Use the [Phase J authoritative validation
checklist](phase-J-plan.md#authoritative-validation-checklist), including
the additional J.4-specific requirement to re-run the full extraction
adapter test suite unchanged, given `Frontmatter`/`ParsedTemplate`'s role as
extraction input.

## Removal path

If any consumer (extraction adapters in particular) shows a behavior change,
revert to the single-module `frontmatter.rs` and keep only the added
characterization tests. Do not partially land a split with broken
cross-surface behavior, since this module's fan-out makes partial breakage
the highest-blast-radius failure mode in Phase J.

## Out of scope

- any change to the public `Frontmatter`/`ParsedTemplate`/
  `parse_template_document` API shape;
- any change to YAML frontmatter semantics as observed by consumers (this is
  a structural split only);
- `crates/sc-composer/src/discovery.rs` or `crates/sc-composer/src/extract/*`
  (excluded from Phase J entirely — this sprint's tests exercise the
  extraction adapters as consumers but do not modify them).
