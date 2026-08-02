---
id: H.2
title: JSON Extraction Core
status: complete
branch: sprint/h-2-json-extraction-core
worktree: ../sc-compose-worktrees/sprint/h-2-json-extraction-core
target: develop
---

# Sprint H.2 — JSON Extraction Core

## Goal

- Implement the approved H.1 known-template JSON extraction contract in pure
  `sc-composer`.
- Reuse the generic extraction report and fail closed on ambiguous, malformed,
  unsupported, or policy-violating JSON cases.

## Hard Dependencies

- H.1 is complete and ADR-0012 is Accepted.
- Phase-G extraction contracts and tests remain green.

## Exact Targets

- `crates/sc-composer/src/extract/mod.rs`
- `crates/sc-composer/src/extract/raw_text.rs` for the shared matching seam
- `crates/sc-composer/src/extract/json.rs`
- `crates/sc-composer/src/extract/xml.rs` to delegate its format-neutral value
  matching to the shared core without changing XML structural behavior
- `crates/sc-composer/src/extract/error.rs`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `crates/sc-composer/tests/fixtures/reverse-extract/xml-regression-baseline.json`
- `crates/sc-composer/Cargo.toml` only if the approved parser requires an
  already-accepted dependency

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H2-D1 — Add the approved JSON format adapter behind `ExtractFormat::Json`
  without changing XML behavior.
- H2-D2 — Match known-template JSON structure with deterministic object-key and
  array-index provenance, as specified by H.1.
- H2-D3 — Implement approved null, duplicate-key, missing-path, ambiguity,
  malformed-input, and rendered-string policies with stable diagnostics.
- H2-D4 — Add unit and integration coverage for supported values, repeated
  paths, empty/null values, malformed JSON, unsupported template expressions,
  and every intentional H.1 boundary.
- H2-D5 — Extract the approved format-neutral delimiter scanning, template
  segment parsing, capture-boundary, and adjacent-variable ambiguity logic from
  the XML implementation into the shared raw-text core. Keep XML structural
  traversal, path/source provenance, and format-specific diagnostics in the XML
  adapter, and prove existing XML tests remain green while JSON delegates to
  the new seam.

### Internal checkpoint: XML parity before JSON policy work

H.2 has two separately reviewable risk boundaries. Before changing `xml.rs`,
the implementation must:

1. run the baseline XML extraction suite on the clean Phase-G input;
2. commit `xml-regression-baseline.json` containing the input identifiers and
   exact report values, occurrence paths/sources, diagnostics, and confidence;
3. extract the format-neutral matcher into `raw_text.rs` and make XML delegate
   to it;
4. run the XML suite and diff its reports against the frozen baseline; and
5. obtain an independent green review of that parity diff before beginning
   JSON-specific parser and policy work.

The completed parity review is recorded in
[`evidence/h-2-xml-parity-review.md`](evidence/h-2-xml-parity-review.md).

Any baseline difference is a parity failure until explicitly reviewed and
accepted as an intentional contract change. JSON tests cannot mask an
incomplete or regressed XML-core extraction checkpoint.

## Required Work

- Keep parsing and matching in `sc-composer`; no file I/O, CLI imports, Python
  imports, or ATM dependencies.
- Ensure one variable at multiple distinct JSON paths is ambiguous and never
  silently overwritten.
- Preserve include/exclude filtering and report confidence semantics.
- Keep dotted-expression handling aligned with the H.1 JSON decision rather
  than reusing the Phase-G XML call-site rule accidentally.
- Add a format-specific source/path model through the generic report aliases.
- Delegate placeholder/value matching to the shared raw-text matching core
  defined by H.1 and implemented by H.2; do not add an independent JSON text
  matcher.

## Explicit Code Samples

```rust
pub enum JsonPathSegment {
    ObjectKey { key: String },
    ArrayIndex { index: usize },
}

pub type JsonExtractionReport =
    ExtractionReport<JsonPathSegment, JsonExtractionSource>;
```

The final names may differ only if the H.1 contract and serialized report
shape remain unchanged.

## This Sprint Does Not Close

- Python or CLI JSON exposure; that is H.3.
- YAML or TOML extraction; those are H.4 and H.5.
- XML mixed-content extraction, XML dirty-prefix tolerance, or a
  customer-facing raw-text/best-effort mode; those are future-phase scope.
- JSON template identification, loops, branches, or typed-value recovery.

## Acceptance Criteria

- Rust callers can select JSON through the approved `ExtractFormat` contract
  and receive the same generic report guarantees as XML.
- Every accepted JSON failure policy has a stable diagnostic and regression
  test; no malformed or ambiguous case returns a fabricated value.
- The frozen XML regression baseline is captured before the `xml.rs` refactor,
  and a post-refactor parity test/diff proves identical XML report values,
  occurrence evidence, diagnostics, and confidence. Existing XML test
  assertions may only be added to, never modified, unless the reviewed parity
  record explicitly accepts the change.
- The XML-parity checkpoint is independently green before JSON-specific policy
  work is considered complete.
- The implementation has no production dependency on the prototype harness.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `git diff --check`
