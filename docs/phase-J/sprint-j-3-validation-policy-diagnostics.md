---
id: J.3
title: Validation Policy and Required-Path Diagnostics
phase: J
status: planned
branch: sprint/j-3-validation-policy-diagnostics
worktree: ../sc-compose-worktrees/sprint/j-3-validation-policy-diagnostics
target: integrate/phase-j
---

# Sprint J.3 — Validation Policy and Required-Path Diagnostics

## Purpose

Complete `crates/sc-composer/src/validation.rs`'s decomposition (Repowise
score 2.37, issue #212) by separating `validate_expanded`'s remaining
diagnostic-policy and required-path/location collector logic — empty-body
checks, frontmatter warnings, default-use diagnostics, required-path
diagnostics, undeclared-variable policy, extra-input policy, and their
ordering — from the orchestration function itself, without changing any
diagnostic behavior.

## Dependencies and exact targets

- `crates/sc-composer/src/validation.rs:59-523` (`validate_expanded` and its
  diagnostic collectors);
- the frozen `ValidationState` shape contract produced by J.2 — this sprint
  consumes that state, it does not re-derive it;
- diagnostic ordering, severity, code, location, and include-chain
  attribution as currently emitted — these are the behavior contract this
  sprint must preserve exactly.

Depends on J.2 (must land first, with its characterization suite passing).

## Deliverables

- Extract a diagnostics/policy layer (e.g. `validation/diagnostics.rs`) and a
  required-path/location layer (e.g. `validation/required_paths.rs`) from
  `validate_expanded`, covering: empty-body checks, frontmatter warnings,
  default-use diagnostics, required-path diagnostics (including nested-array
  traversal), undeclared-variable policy, and extra-input policy.
- Preserve `validate_expanded`'s existing signature and its role as the
  single orchestration entry point — the new layers are internal
  collaborators, not new public APIs.
- Preserve diagnostic ordering, severity, codes, locations, and include-chain
  attribution exactly as emitted today for every existing fixture.
- Inventory the ~40 existing `validation.rs` tests by contract (required
  paths, defaults, undeclared/extra inputs, built-ins, pass scopes) and move
  each test with the seam it characterizes, adding at least one regression
  through the public `sc_composer::validate()` entry point per contract group
  so a misrouted assertion after the split cannot pass silently.

## Planned internal seam

The two collaborators remain private and `validate_expanded` remains the only
orchestration entry point:

```rust
fn collect_policy_diagnostics(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
    resolved_path: &Path,
    state: &ValidationState,
) -> (Vec<Diagnostic>, Vec<Diagnostic>);

fn required_path_diagnostics(state: &ValidationState) -> Vec<Diagnostic>;

// validate_expanded keeps the existing signature and calls these helpers
// without changing diagnostic ordering or attribution.
```

## Acceptance criteria

- Every existing validation diagnostic (code, severity, message, order,
  location, include-chain attribution) is byte-for-byte unchanged for the
  full existing fixture set, verified by running the full pre-existing
  `validation.rs` test suite (moved, not deleted) unchanged.
- Nested-array required-path traversal behavior is unchanged.
- No new diagnostic code, severity level, or ordering rule is introduced as
  a side effect of the refactor.

## Required validation

Use the [Phase J authoritative validation
checklist](phase-J-plan.md#authoritative-validation-checklist). The focused
evidence must include the full moved-and-inventoried test suite passing,
grouped by contract, plus a diff review confirming no diagnostic output
changed for any fixture.

## Removal path

If moving a diagnostic collector changes ordering or output for any fixture,
revert that specific collector's move and keep it in `validate_expanded`
rather than force a passing-but-incorrect split.

## Out of scope

- any change to `ValidationState` assembly (owned by J.2, already landed);
- any change to diagnostic codes, severities, or message text — this is a
  structural split only;
- `crates/sc-composer/src/discovery.rs` or `crates/sc-composer/src/extract/*`
  (excluded from Phase J entirely).
