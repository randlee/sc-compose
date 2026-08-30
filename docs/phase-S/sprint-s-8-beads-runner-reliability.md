---
id: S.8
title: Beads Runner Reliability
status: complete
branch: sprint/s-8-beads-runner-reliability
worktree: ../sc-compose-worktrees/sprint/s-8-beads-runner-reliability
target: sprint/s-7-path-normalization-contract
---

# Sprint S.8 — Beads Runner Reliability

## Goal

Make the bounded-output process runner easier to reason about and test after
its recent defect history, while preserving its host-neutral request/receipt
contract and platform-specific containment strategy. This closes S-T8.

## Hard Dependencies

- S.7 is this branch's required `gh stack` parent. There is no functional code
  dependency on S.1–S.7; the parent keeps this final PR incremental.
- **Phase-close dependency:** S.1 through S.7 must be reviewed and CI-green
  before the S.8-top stack can merge atomically at phase close.
- The existing `process-wrap` containment contract remains approved; a new
  process library or OS-containment policy requires ADR review.

## Exact Targets

- `crates/sc-composer-beads/src/runner.rs`
- `crates/sc-composer-beads/tests/runner_process_tree.rs`
- `docs/architecture.md`
- `docs/phase-S/sprint-s-8-beads-runner-reliability.md`

## Deliverables

- Private runner-state seams for capture completion, cap breach, contained
  termination, child-status collection, and reader join behavior.
- Deterministic unit tests for normal non-zero exits, stream cap breach,
  reader failure, and output completion ordering.
- Real supported-platform proof that Unix process groups and Windows Job
  Objects terminate contained descendant trees on cap breach; no shell.
- A documented unsupported-platform fallback that remains direct-child only.

## Required Work

- Preserve `CommandSpec`, `ProcessOutput`, `ProcessRunner`, stable error
  markers, Beads stage receipt mapping, and direct argv execution.
- Do not add Beads source/database dependencies, make the runner CLI-aware, or
  expose foreign-language bindings from `sc-composer-beads`.
- Follow `CLAUDE.md` Rule 11: only already-approved `process-wrap` is allowed;
  the crate remains host-neutral with no CLI, adapter, ATM, or Beads source
  dependency.
- **Production-ready closure:** every listed capture/containment state and its
  committed supported-platform proof must land in this sprint; partial
  platform coverage does not close S-T8.

## Explicit Code Samples

```rust
// Private lifecycle state; public runner contract remains unchanged.
enum CaptureState {
    Waiting,
    OutputLimitExceeded,
    Completed,
}
```

The exact private representation may differ, but tests must prove the same
lifecycle boundaries independently.

## This Sprint Does Not Close

- New Beads CLI commands, formula semantics, real persistent pours, or a
  request/receipt schema revision.
- An unreviewed replacement for `process-wrap` or a cross-platform process API
  change requiring an ADR.

## Acceptance Criteria

- [x] Normal exit status remains exactly the child's status.
- [x] Cap breach returns the existing typed output-limit marker only after
  contained-child termination and capture-reader cleanup.
- [x] A descendant retaining an output pipe cannot hang supported Unix or
  Windows runner paths.
- [x] Runner unit and real pinned-`bd` integration tests pass on Linux, macOS,
  and Windows CI.
- [x] No shell, Beads source/database, CLI, adapter, or ATM dependency enters
  `sc-composer-beads`.

## gh-stack Workflow

```bash
# The phase plan added this branch directly on top of S.7.
git config rerere.enabled true
git config remote.pushDefault origin
git add crates/sc-composer-beads/src/runner.rs crates/sc-composer-beads/tests/runner_process_tree.rs docs/architecture.md docs/phase-S/sprint-s-8-beads-runner-reliability.md
git commit -m "refactor(beads): isolate bounded runner lifecycle"
gh stack submit --auto
gh pr ready <sprint-s-8-pr-number>
gh stack view --json
# The phase plan owns the sole final `gh stack merge` command.
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-composer-beads --all-targets --all-features -- -D warnings`
- `cargo test -p sc-composer-beads`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
