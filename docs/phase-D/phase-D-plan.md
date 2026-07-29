---
id: phase-D
title: Recursive Inputs And Adversarial Validation
status: draft
branch: plan/phase-D
worktree: ../sc-compose-worktrees/plan/phase-D
---

# Phase D — Recursive Inputs And Adversarial Validation

## Objective

Extend the structured-input contract to support the finite recursive value
trees that `serde_json::Value` and Minijinja already represent, then add a
repeatable adversarial testing workflow that tries to break each rendering
boundary and preserves confirmed failures as regression tests.

This phase is split because runtime input support and reusable QA orchestration
have different closure types and ownership. D.1 owns production behavior. D.2
owns the skill, agents, contracts, and QA workflow that exercise that behavior.

## Sprint sequence

1. [Sprint D.1 — Recursive Structured Input Support](./sprint-d-1-recursive-structured-input.md)
   - remove the artificial nested-array validation restriction;
   - preserve top-level var-file and YAML string-key boundaries;
   - add JSON/YAML/default-source regression coverage.
2. [Sprint D.2 — Adversarial Fuzzing Workflow](./sprint-d-2-adversarial-fuzzing.md)
   - create the bounded multi-agent fuzzing skill;
   - have a coordinator deploy focused background probes;
   - minimize confirmed failures and promote durable tests.

The sequence is contiguous and intentional: D.2 depends on D.1 so its first
campaign can attack the expanded recursive-input contract instead of encoding
the old validation restriction as an oracle.

## Phase exit gate

Phase D is complete only when both sprint exit gates pass:

- recursive values render through the documented ingress paths with no
  regression to existing top-level or malformed-input boundaries;
- the adversarial-fuzzing skill and registered agents pass structural
  validation, return deterministic fenced contracts, enforce bounded parallel
  execution, and have at least one campaign whose confirmed findings are
  either promoted to tests or explicitly classified as non-bugs.

The phase does not include a production fix for every fuzz finding. Findings
that require a separate design or runtime change must become an explicitly
owned follow-on sprint rather than being silently carried forward.

## Non-goals

- introducing a new template engine or changing the Minijinja dependency;
- adding arbitrary schema language or bracket-path syntax;
- adding network-facing fuzz infrastructure;
- allowing generated agents to edit production code without a confirmed,
  reviewable regression-test promotion step;
- changing the standalone crate boundaries or adding ATM runtime dependencies.
