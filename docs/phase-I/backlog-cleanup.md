---
id: phase-I-backlog
title: Phase I Non-Blocking Backlog Cleanup
phase: I
status: complete
branch: chore/phase-i-backlog-cleanup
worktree: ../sc-compose-worktrees/chore/phase-i-backlog-cleanup
target: develop
---

# Phase I Backlog Cleanup

Three non-blocking items surfaced during Phase I's final exit-gate
`phase_ending_review` (PASS at 12/12, 100%, ATM `01KZ5KJ4NNNYA357EAMYGQZJFC`)
that were explicitly deferred as backlog rather than gating Phase I closure.
None of these affect Phase I's completeness; they are cleanup only.

## Deliverables

1. **Stale status wording** — `docs/project-plan.md:559`'s Phase I status
   intro bullet still reads "planned follow-on work..." even though every
   I.1-I.6 sub-bullet now reads complete/accepted. Update the intro bullet to
   reflect that Phase I is complete. Note: Phase G's status bullet has the
   same stale pattern (per quality-mgr's ATM-QA-006 finding) — leave Phase G
   untouched unless explicitly asked; this deliverable is Phase I's bullet
   only.
2. **Duplicate `line_column()` helper** — `crates/sc-composer/src/extract/raw.rs:109`
   and `crates/sc-composer/src/extract/xml_prefix.rs:99` each independently
   reimplement an identical byte-offset-to-line/column helper (flagged by
   simplification-reviewer during the Phase I exit-gate review). Hoist the
   shared implementation into `crates/sc-composer/src/extract/mod.rs` and have
   both call sites use it. No behavior change — this is a pure dedup.
3. **Undocumented pre-Phase-I error codes** — `docs/error-code-registry.md`
   is missing entries for four error codes that predate Phase I and are
   already shipped: `ERR_EXTRACT_XML_ELEMENT_MISMATCH`,
   `ERR_EXTRACT_XML_ATTRIBUTE_MISMATCH`, `ERR_EXTRACT_XML_STATIC_MISMATCH`,
   and `ERR_EXTRACT_XML_NAMESPACE_UNSUPPORTED` (flagged by arch-qa as
   ARCH-001). Add registry rows for all four, matching the existing table's
   format and the style used for the Phase I `ERR_EXTRACT_XML_*` rows added
   in PR #222. Locate each code's actual trigger condition in source
   (`crates/sc-composer/src/extract/`) before writing the description — do
   not guess from the name alone.

## Acceptance criteria

- `docs/project-plan.md`'s Phase I status intro bullet no longer says
  "planned follow-on work..."; Phase G's bullet is untouched.
- `raw.rs` and `xml_prefix.rs` both call one shared `line_column()` (or
  equivalently named) helper in `extract/mod.rs`; no duplicate implementation
  remains; existing tests for both call sites still pass unchanged.
- All four pre-existing error codes appear in `docs/error-code-registry.md`
  with accurate, source-verified descriptions.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --
  -D warnings`, `cargo test --workspace`, and the Python pytest suite all
  pass.
- No behavior change to any extraction, validation, or rendering path — this
  is a docs/dedup-only cleanup.

## References

- Phase I exit-gate final PASS: ATM `01KZ5KJ4NNNYA357EAMYGQZJFC`
- Phase I merge-forward to develop: PR #224 (`57c4f71`)
- Error-code registry format precedent: PR #222 (`e38e389`)
