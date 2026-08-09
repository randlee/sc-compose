---
id: FIX-354-355
title: "Phase L follow-up: sc_lint_*.rs RAII/duplication sweep + sprint-doc frontmatter sync"
status: complete
branch: fix/phase-l-followup-raii-frontmatter
worktree: ../sc-compose-worktrees/fix/phase-l-followup-raii-frontmatter
target: develop
---

## Goal

Close out the two non-blocking follow-ups raised during Phase L's
phase-ending review (PR #352, merged to `develop` at `1abe082`):

- Issue #354 — sweep the `sc_lint_*.rs` integration-test fixture/teardown
  duplication onto the shared `tests/support` module.
- Issue #355 — sync stale `status: planned`/`in-progress` frontmatter in
  `docs/phase-L/phase-L-plan.md` and the L.1, L.2, L.4-L.9 sprint docs to
  reflect actual completion.

## Hard Dependencies

None. Phase L (L.1-L.17) is fully merged to `develop`. This sprint is
independent of any other in-flight work.

## Scope: Issue #354 — test-suite RAII/duplication

Raised independently by simplification-reviewer, test-auditor, and
flaky-test-qa during the Phase L phase-ending review, and separately
confirmed by comp's lint-suppression audit. This exact duplication pattern
already caused one real defect mid-phase: the incomplete `PYTHON_TOOLS`
fixture list bug (QM-L14-002 / QM-L14-004), fixed at `e2ec06c`.

### Required Work

- Inventory every `TempFixture`/`copy_directory`/`parse_stdout`-equivalent
  helper reimplemented in `crates/sc-compose/tests/sc_lint_*.rs` (at minimum
  `sc_lint_ci.rs`, `sc_lint_clippy_xwin.rs`, `sc_lint_lint_ci.rs`,
  `sc_lint_lint_fast.rs`, `sc_lint_lint_full.rs`) instead of using
  `crates/sc-compose/tests/support/mod.rs`.
- Add the 3 shared helpers recommended by test-auditor to `tests/support`:
  - an RAII-safe fixture materializer (temp dir + guaranteed cleanup on
    drop, including on panic/assertion failure)
  - a `.just`-source locator
  - a Windows fake-cargo shim
- Replace the per-file duplicated implementations in the `sc_lint_*.rs`
  files with calls into the new shared helpers. Do not change test
  assertions or coverage — this is a pure de-duplication/hardening pass.
- Fix the non-RAII teardown flagged by flaky-test-qa in the 3 affected
  `sc_lint_*.rs` files (leaks/can panic-mask on assertion failure), and the
  `support::sc_compose()` per-call log-root directory leak.

### This Sprint Does Not Close

- Does not change production `sc_lint.rs` runner behavior.
- Does not add or remove any lint target/profile.
- Does not touch `deny.toml` or Cargo.toml lint policy (tracked separately
  in issue #353).

### Acceptance Criteria

- No `sc_lint_*.rs` test file reimplements fixture setup/teardown that
  already exists in `tests/support`.
- All fixture temp state is cleaned up via RAII (drop), including on
  assertion failure/panic — verified by an intentionally failing
  assertion in a scratch test run, not just the happy path.
- `support::sc_compose()` no longer leaks a log-root directory per call.
- `cargo test --workspace` passes on all 3 CI platforms with no new
  flakiness.

## Scope: Issue #355 — sprint-doc frontmatter sync

Raised by req-qa (PL-QA-002) during the same review; independently
confirmed true and non-blocking by quality-mgr.

### Required Work

- Update the `status:` frontmatter field to the repo's terminal status
  value (matching the convention used by closed-out phases, e.g. Phase K's
  sprint docs) in:
  - `docs/phase-L/phase-L-plan.md`
  - `docs/phase-L/sprint-l-1-sc-lint-bootstrap.md`
  - `docs/phase-L/sprint-l-2-sc-lint-just-report-integration.md`
  - `docs/phase-L/sprint-l-4-sc-portability.md`
  - `docs/phase-L/sprint-l-5-sc-runtime.md`
  - `docs/phase-L/sprint-l-6-line-counts.md`
  - `docs/phase-L/sprint-l-7-identity-literals.md`
  - `docs/phase-L/sprint-l-8-view-findings.md`
  - `docs/phase-L/sprint-l-9-check-native.md`
- Do not alter any other content in these docs — this is frontmatter-only.
- Verify the remaining L.3, L.10-L.17 sprint docs are already correctly
  marked complete (no change needed) rather than assuming.

### Acceptance Criteria

- No sprint doc under `docs/phase-L/` contradicts `docs/project-plan.md`'s
  Phase L status block (which asserts full completion as of `1abe082`).
- `git diff` for this half of the sprint touches only YAML frontmatter
  blocks, no body content.

## sc-lint Cleanup Routing

Run the applicable sc-lint documentation/configuration checks on the final
sprint commit. Fix minor documentation findings immediately. If a remaining
finding is out of scope for this sprint, file it as a new issue rather than
expanding scope here.

## Required Validation

- `cargo test --workspace` (all 3 CI platforms)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `git diff --stat` reviewed to confirm the frontmatter-only constraint for
  the issue #355 half of the diff
- `gh issue view 354 --json state` / `gh issue view 355 --json state` after
  merge, to close both issues with a reference to the merged PR
