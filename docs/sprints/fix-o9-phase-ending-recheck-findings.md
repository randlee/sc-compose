---
id: FIX-O9
title: Close QA-PHASE-O-ENDING-RECHECK findings (O5-015 citation, error-registry gap, RBP structural fixes)
status: in-progress
branch: fix/phase-o-outstanding-findings
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/phase-o-outstanding-findings
target: integrate/phase-o
---

## Root Cause

quality-mgr's fresh, full `phase_ending_review` of `integrate/phase-o` at head
`51031d2` (QA-PHASE-O-ENDING-RECHECK, verdict FAIL) found zero Blocking
findings but 5 Important + 2 Minor findings, all independently confirmed via
direct source inspection:

1. **Important** — `docs/phase-O/o5-task-checklist.md:55-67,82-84`: O5-015's
   closing rationale and mirrored "Final verification" bullet cite PR
   #430/#431 `gh pr checks` 12/12 as evidence the production-scoped
   `template-contracts` CI gate is green. That gate did not exist before PR
   #434 (`fix/o6-ci-template-contracts-gate`) — PR #430/#431 predate it and
   could not have exercised an assertion that did not yet exist.
2. **Important** — `docs/error-code-registry.md`: missing a canonical table
   row for `ERR_JSON_MODE_INCLUDE_CONFLICT`, despite the registry's own
   documented rule that new stable `ERR_*` codes require a registry update.
   The code is registered in code
   (`crates/sc-composer/src/diagnostics/schema.rs:77,226,354`) but absent from
   the doc.
3. **Important (RBP-001)** — `crates/sc-composer/src/frontmatter/parser.rs:25`:
   `parse_template_document` discards the underlying `serde_yaml::Error` via
   `map_err(|_error| ConfigError::new(...))` instead of
   `ConfigError::with_source(error)` (already available, already used
   elsewhere at `crates/sc-composer/src/error/config.rs:34`).
4. **Important (RBP-001)** — `bindings/python/src/enums.rs:93-243`:
   `PyDiagnosticCode` is missing the two diagnostic codes introduced this
   phase (`ERR_JSON_MODE_INCLUDE_CONFLICT`, `WARN_LINT_REDUNDANT_FILTER_CHAIN`)
   against the canonical Rust-side registry.
5. **Important (RBP-004)** — `crates/sc-composer/src/template_scanner.rs:13`:
   `next_jinja_variable_expression` returns an unnamed positional
   `Option<(usize, usize, usize)>` tuple across a crate boundary, destructured
   differently at each of its two call sites.
6. **Minor, non-blocking** —
   `crates/sc-composer/tests/fixtures/reverse-extract/json-atm-payload.json.j2`
   / `json-malformed.json.j2`: legacy-mode relabeling (commit `01e3883`) has no
   test proving its stated purpose (`WARN_JSON_LEGACY_ESCAPE_MODE` firing
   instead of `ERR_JSON_MODE_CONTRACT`).
7. **Minor, non-blocking** — `tests/fixtures/sc-lint/bootstrap/Cargo.lock`:
   recurring uncommitted local drift, unrelated to any reviewed commit; a known
   local build/lint-tool artifact, not phase content — confirmed by
   rust-qa-agent across multiple prior reviews. No fix required; do not commit
   any change to this file.

## Fix Design

- Correct the O5-015 rationale and mirrored bullet in
  `docs/phase-O/o5-task-checklist.md` to cite PR #434 (which introduced the
  enforcing gate) rather than PR #430/#431.
- Add a canonical row for `ERR_JSON_MODE_INCLUDE_CONFLICT` to
  `docs/error-code-registry.md`, matching the format of existing rows.
- Change `crates/sc-composer/src/frontmatter/parser.rs:25` to use
  `ConfigError::with_source(error)`, preserving the cause chain.
- Add `ErrJsonModeIncludeConflict` and `WarnLintRedundantFilterChain` (or the
  exact naming convention already used by sibling variants) to
  `bindings/python/src/enums.rs`'s `PyDiagnosticCode`, keeping it a complete
  mirror of the Rust-side registry.
- Replace `next_jinja_variable_expression`'s unnamed
  `Option<(usize, usize, usize)>` return type in
  `crates/sc-composer/src/template_scanner.rs` with a small named struct
  (e.g. `JinjaVariableExpressionSpan { start, name_end, end }` — field names
  at the fixer's discretion, but must be named, not positional), updating both
  call sites' destructuring accordingly.
- Optionally, in the same PR: add a regression test proving
  `json-atm-payload.json.j2` legacy-mode relabeling fires
  `WARN_JSON_LEGACY_ESCAPE_MODE` rather than `ERR_JSON_MODE_CONTRACT` (Minor
  finding #6). Do not touch `tests/fixtures/sc-lint/bootstrap/Cargo.lock`
  (Minor finding #7 — known benign local artifact, not phase content).

## Required Changes / Tests

- `docs/phase-O/o5-task-checklist.md`: O5-015 rationale + mirrored bullet
  corrected to cite PR #434.
- `docs/error-code-registry.md`: new row for `ERR_JSON_MODE_INCLUDE_CONFLICT`.
- `crates/sc-composer/src/frontmatter/parser.rs`: `ConfigError::with_source`
  fix + regression test confirming the parse-error cause chain is preserved.
- `bindings/python/src/enums.rs`: two new `PyDiagnosticCode` variants +
  parity test/assertion if one already exists for this enum.
- `crates/sc-composer/src/template_scanner.rs`: named-struct return type,
  both call sites updated (`crates/sc-compose/src/commands/template_lint.rs`,
  `crates/sc-composer/src/validation/diagnostics.rs`).
- Optional: new test for legacy-mode relabeling fixtures.

## Out of Scope

- Any further scanner/architecture rework beyond the named-struct return type.
- `tests/fixtures/sc-lint/bootstrap/Cargo.lock` — leave untouched.

## Acceptance Criteria

- All 5 Important findings from QA-PHASE-O-ENDING-RECHECK closed with direct
  file:line evidence.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --workspace`, `git diff --check` all clean.
- No unrelated files touched (confirm via `git diff --stat` against
  `origin/integrate/phase-o`).

## References

- QA-PHASE-O-ENDING-RECHECK verdict (FAIL), Findings 1-7.
- `docs/sprints/fix-434-ci-template-contracts-gate.md`.

## Priority

High — blocks `integrate/phase-o` → `develop` merge readiness per
quality-mgr's hard 100%-deliverable-completion gate. Zero Blocking findings;
all 5 required fixes are small and mechanical per quality-mgr's own
assessment.
