---
id: sprint-I.5
title: Jinja Loop-Context Built-ins
phase: I
status: planned
branch: sprint/i-5-loop-context-builtins
worktree: ../sc-compose-worktrees/sprint/i-5-loop-context-builtins
target: develop
---

# Sprint I.5 — Jinja Loop-Context Built-ins

## Purpose

Close GitHub issue #167. Make strict undeclared-variable discovery aware of
Jinja's loop context without turning arbitrary names into implicit globals.

## Dependencies and exact targets

- I.1 accepted built-in list, scope, and shadowing rules;
- token discovery and loop-scope code in
  `crates/sc-composer/src/discovery.rs`, integrated by
  `crates/sc-composer/src/validation.rs`;
- strict validation and CLI diagnostic tests;
- Python behavior if validation is surfaced through the binding.

## Deliverables

- Extend the existing statement-aware scope tracker so loop-context names are
  ignored only inside the corresponding active `for` scope.
- Recognize the I.1-approved scalar attributes and `loop.cycle(...)` form;
  preserve discovery of iterable expressions, filters, caller variables, and
  loop-body variables.
- Handle nested loops with independent scopes and bindings that shadow outer
  names.
- Keep a user-declared/reference `loop` outside a `for` scope visible to normal
  undeclared-token policy.
- Add strict-mode regression tests for the #167 `loop.last` reproduction,
  every approved built-in, nested loops, shadowing, outside-loop use, and
  unsupported lookalikes.

## Acceptance criteria

- The issue #167 template no longer fails strict validation solely because of
  `loop.last`, `loop.index`, or another approved loop-context name.
- An undeclared `loop` outside a loop remains a warning/error according to the
  active policy; it is not globally injected.
- `item`, iterable expressions, filters, and nested-loop variables retain
  their existing discovery behavior.
- The scanner does not accept arbitrary `loop.anything` unless I.1 explicitly
  includes it; unsupported names remain discoverable or produce the approved
  diagnostic.
- Existing multi-pass brace-count and strict/default validation suites pass.

## Required validation

Use the [authoritative Phase I validation
checklist](phase-I-plan.md#authoritative-validation-checklist). The focused
evidence must include both pass and fail examples with the full
template text, frontmatter declarations, policy, and resulting diagnostics.

## Removal path

If scope-aware built-ins fail QA, revert only the loop-context exemption and
its tests. Restore ordinary token discovery rather than broadening the global
implicit-variable set.

## Out of scope

- general Jinja AST execution or full built-in discovery;
- loop reconstruction in reverse extraction;
- changing `VariableName` grammar globally;
- making `loop` an implicit variable outside a `for` scope.
