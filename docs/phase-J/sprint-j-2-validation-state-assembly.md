---
id: sprint-J.2
title: Validation State and Context Assembly
phase: J
status: planned
branch: sprint/j-2-validation-state-assembly
worktree: ../sc-compose-worktrees/sprint/j-2-validation-state-assembly
target: integrate/phase-j
---

# Sprint J.2 — Validation State and Context Assembly

## Purpose

Reduce `crates/sc-composer/src/validation.rs`'s hot-spot risk (Repowise score
2.37, issue #212) by extracting `ValidationState` assembly — frontmatter and
default merging, per-pass token discovery maps, required-path origins,
variable-source precedence, and built-in environment/date injection — into a
dedicated state module, without changing validation behavior. This is the
highest-risk sprint in Phase J: I.5 (loop-context built-ins) recently changed
this exact state/discovery boundary, and `ValidationState` is directly
consumed by `composer.rs`.

## Dependencies and exact targets

- `crates/sc-composer/src/validation.rs:46-57` (`ValidationState` struct);
- `crates/sc-composer/src/validation.rs:525-769` (state assembly: frontmatter/
  default merging, per-pass discovery maps, required origins, precedence,
  built-in injection);
- the coupling to `discovery::discover_all_pass_tokens` — this sprint may
  move *where* that call happens but must not alter discovery semantics;
- `composer.rs`'s consumption of `ValidationState` — must not require a
  call-site or shape change.

Depends on J.1 only for phase sequencing (no code dependency); must land and
have its characterization suite passing before J.3 begins.

## Deliverables

- Freeze a `ValidationState` shape contract (documented field-by-field: what
  each field means, who populates it, what invariants hold) *before* moving
  any code — this is a decomposition safety requirement, not documentation
  afterthought.
- Move `ValidationState` construction, per-pass discovery-map assembly,
  required-path origin tracking, variable-source precedence resolution, and
  built-in environment/date injection into a new state module (e.g.
  `validation/state.rs`), reachable only through crate-private or private
  APIs — no new public surface.
- Preserve `composer.rs`'s existing consumption of `ValidationState`
  unchanged in shape and behavior.
- Do not alter `discovery::discover_all_pass_tokens` semantics; this sprint
  relocates the caller, not the callee.
- Add characterization tests for `ValidationState` assembly covering: I.5
  loop-context discovery output, default-merge precedence, required-path
  origin attribution, and built-in injection — captured against the current
  (pre-move) behavior before any code moves.

## Acceptance criteria

- Every existing validation diagnostic (code, severity, message, order,
  location, include-chain attribution) is unchanged for the full existing
  fixture set.
- The full I.5 loop-context regression suite
  (`validation::tests::strict_mode_accepts_approved_loop_context_builtins`
  and siblings) passes unchanged, run both before and after the move.
- `discovery.rs` is not modified by this sprint.
- `composer.rs` requires no call-site changes.

## Required validation

Use the [Phase J authoritative validation
checklist](phase-J-plan.md#authoritative-validation-checklist), including
the additional J.2-specific requirement to re-run the full I.5 loop-context
regression suite unchanged.

## Removal path

If any characterization test fails post-move, or if `composer.rs` integration
breaks, revert to the single-module `validation.rs` state assembly and keep
only the frozen state-shape contract and added characterization tests as
documentation for a future retry.

## Out of scope

- any change to `discovery.rs` semantics or its public surface;
- `validate_expanded`'s diagnostic-policy and required-path collector logic
  (owned by J.3, which depends on this sprint's frozen state contract);
- `crates/sc-composer/src/extract/*` (excluded from Phase J entirely).
