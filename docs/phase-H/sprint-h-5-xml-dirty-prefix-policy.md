---
id: H.5
title: XML Dirty-Prefix Policy
status: planned
branch: sprint/h-5-xml-dirty-prefix-policy
worktree: ../sc-compose-worktrees/sprint/h-5-xml-dirty-prefix-policy
target: develop
---

# Sprint H.5 — XML Dirty-Prefix Policy

## Goal

- Implement the H.1-approved, narrowly bounded policy for rendered XML with a
  permitted non-XML preamble.
- Improve real ATM payload tolerance without converting malformed or hostile
  input into an unreviewable success.

## Hard Dependencies

- H.1 defines and accepts the preamble grammar, limits, diagnostics, and root
  selection policy.
- Phase-G and H.4 XML matching behavior remain green.

## Exact Targets

- `crates/sc-composer/src/extract/xml.rs`
- `crates/sc-composer/src/extract/error.rs`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `bindings/python/tests/test_smoke.py`
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `docs/requirements.md` and `docs/architecture.md` only for accepted policy
  wording

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H5-D1 — Add a bounded preamble scanner implementing the exact H.1 grammar,
  rather than generic first-tag recovery.
- H5-D2 — Preserve the discarded-prefix evidence and expose the approved
  warning/diagnostic without corrupting normal report output.
- H5-D3 — Continue to reject malformed XML, multiple roots, deceptive tags,
  oversized prefixes, and disallowed encodings according to policy.
- H5-D4 — Add cross-surface fixtures for accepted ATM headers, whitespace,
  comments/declarations, malformed prefixes, and hostile near-misses.

## Required Work

- Keep prefix handling in the pure in-memory extraction layer; CLI file reads
  remain CLI-owned.
- Ensure the same input cannot select different roots across Rust, Python, and
  CLI surfaces.
- Document whether callers may opt out and how the original input is retained
  for evidence.

## Explicit Code Samples

```rust
pub struct XmlInputMetadata {
    pub discarded_preamble: Option<String>,
    pub root_offset: usize,
}
```

The concrete metadata shape must follow H.1 and must not hide discarded input
from a caller who needs to audit the extraction.

## This Sprint Does Not Close

- Generic malformed-XML recovery or multiple-root recovery.
- JSON, YAML, or TOML extraction.
- Unknown-template identification or arbitrary text scraping.

## Acceptance Criteria

- Approved dirty-prefix fixtures succeed with deterministic metadata and
  diagnostics.
- Genuine malformed XML, multiple roots, and hostile near-misses remain
  fail-closed with stable error categories.
- Existing clean XML reports are unchanged byte-for-byte where the contract
  requires it.
- Rust, Python, and CLI surfaces agree on accepted and rejected preambles.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
