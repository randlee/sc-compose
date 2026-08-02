---
id: H.4
title: XML Mixed-Content Extraction
status: planned
branch: sprint/h-4-xml-mixed-content
worktree: ../sc-compose-worktrees/sprint/h-4-xml-mixed-content
target: develop
---

# Sprint H.4 — XML Mixed-Content Extraction

## Goal

- Implement the H.1-approved representation for a variable whose XML value
  spans text nodes and child elements.
- Extend the scalar XML contract narrowly without weakening structural
  matching, ambiguity handling, or XML safety.

## Hard Dependencies

- H.1 explicitly accepts mixed-content extraction and defines its canonical
  value and provenance representation.
- Phase-G XML behavior and H.2/H.3 JSON behavior remain green.

## Exact Targets

- `crates/sc-composer/src/extract/xml.rs`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `bindings/python/tests/test_smoke.py` only for shared-surface parity cases
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `docs/requirements.md` and `docs/architecture.md` only for the accepted
  contract correction

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H4-D1 — Match the approved mixed-content template shape deterministically
  and return the exact H.1-defined rendered string representation.
- H4-D2 — Preserve child-element order, entity handling, whitespace policy,
  occurrence paths, and ambiguity diagnostics.
- H4-D3 — Reject unsupported mixed structures, loops, branches, and malformed
  XML without fabricating a block value.
- H4-D4 — Add Rust, Python, and CLI regression fixtures using realistic
  description/workflow/reference payloads.

## Required Work

- Replace positional text/element rejection only for the H.1-approved shape;
  do not introduce arbitrary DOM serialization.
- Define and test behavior for empty children, nested children, CDATA,
  entities, repeated variables, and static text around the variable.
- Keep the Phase-G scalar path behavior byte-compatible for existing inputs.

## Explicit Code Samples

```rust
// The selected representation is normative after H.1 acceptance.
pub enum XmlExtractionSource {
    TextNode,
    MixedContent { child_path: Vec<XmlPathSegment> },
}
```

The implementation may choose a different concrete shape only if it preserves
the H.1 report serialization and provenance rules.

## This Sprint Does Not Close

- JSON, YAML, or TOML extraction.
- Dirty prefixes before the XML root; that is H.5.
- Jinja loops, conditionals, typed recovery, or unknown-template discovery.

## Acceptance Criteria

- Every accepted mixed-content example returns the canonical value and
  provenance, and every rejected shape returns a stable diagnostic.
- Existing scalar XML fixtures pass unchanged.
- The same mixed-content outcome is visible through Rust, Python, and CLI
  surfaces without adapter-specific matching logic.
- Security and size behavior for serialized child content is bounded by the
  H.1 policy.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
