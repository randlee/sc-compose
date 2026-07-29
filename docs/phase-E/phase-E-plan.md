---
id: phase-E
title: Recursive Inputs And Adversarial Validation
status: draft
branch: plan/phase-E
worktree: ../sc-compose-worktrees/plan/phase-E
---

# Phase E — Recursive Inputs And Adversarial Validation

## Objective

Extend the structured-input contract to support the finite recursive value
trees that `serde_json::Value` and Minijinja already represent, then add a
repeatable adversarial testing workflow that tries to break each rendering
boundary and preserves confirmed failures as regression tests.

This phase is split because runtime input support and reusable QA orchestration
have different closure types and ownership. E.1 owns production behavior. E.2
owns the skill, agents, contracts, and QA workflow; E.3 runs the first real
campaign and owns its classified findings and promoted tests.

## Sprint sequence

1. [Sprint E.1 — Recursive Structured Input Support](./sprint-e-1-recursive-structured-input.md)
   - remove the artificial nested-array validation restriction;
   - preserve top-level var-file and YAML string-key boundaries;
   - add JSON/YAML/default-source regression coverage.
2. [Sprint E.2 — Adversarial Fuzzing Workflow](./sprint-e-2-adversarial-fuzzing.md)
   - create the bounded multi-agent fuzzing skill;
   - have a coordinator deploy focused background probes;
   - minimize confirmed failures and define the real-campaign closure gate.
3. [Sprint E.3 — First Adversarial Campaign And Regression Closure](./sprint-e-3-first-adversarial-campaign.md)
   - execute a real bounded campaign;
   - classify every candidate and promote confirmed bugs into tests;
   - publish campaign evidence for quality-mgr review.

The sequence is contiguous and intentional: E.2 depends on E.1, and E.3
depends on both so its first real campaign can attack the expanded
recursive-input contract instead of encoding
the old validation restriction as an oracle.

## Phase exit gate

Phase E is complete only when all three sprint exit gates pass:

- recursive values render through the documented ingress paths with no
  regression to existing top-level or malformed-input boundaries;
- the adversarial-fuzzing skill and registered agents pass structural
  validation, return deterministic fenced contracts, and enforce bounded
  parallel execution;
- E.3 executes a real campaign, classifies every candidate, and promotes every
  confirmed bug that is in scope into a deterministic test, or records that no
  confirmed bug was found.

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
