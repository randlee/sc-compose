---
id: P.3
title: atm-core Released-Consumer Acceptance
phase: P
status: planned
target: external-atm-core/develop
---

# Sprint P.3 — atm-core Released-Consumer Acceptance

## Goal

Apply the same P.1-qualified released sc-lint product to an atm-core worktree
through an independently reviewed atm-core PR. This sprint supplies external
consumer evidence; it does not authorize direct writes from sc-compose.

## Hard dependencies

- P.1 dual-reference matrix PASS and the accepted ADR amendment;
- an atm-core-owned worktree and team approval;
- the exact same sc-lint release version and request schema as P.1/P.2.

## Exact targets

Targets are resolved only from the P.1-approved atm-core preview. At minimum it
must consider `sc-lint.toml`, product-managed `.sc-lint/` assets, the root
`Justfile`, and `.github/workflows/ci.yml`; it may not broaden to arbitrary
`.just` helpers or ATM runtime code without a separately approved classification.

## Deliverables

- An atm-core PR containing only the P.1-proven plan operations and retained
  preview/apply/reapply evidence.
- A documented composition for atm-core's existing `lint`, `test`, and `ci`
  recipes, or a no-write product gap. The initial absence of `sc-lint.toml`
  must be handled by the released tool, not a copied consumer installer.
- Clean-checkout and CI proof for `just setup`, `just lint`, `just test`, and
  `just upgrade` on Linux, macOS, and Windows.

## Acceptance criteria

- No atm-core helper, daemon behavior, release behavior, or unrelated recipe
  is removed merely because sc-compose did not need it.
- The atm-core preview/apply/reapply results match P.1 evidence; unexpected
  repository drift or a new recipe shape returns to P.1 as a product gap.
- The same release/config version authority works locally and in CI without
  source checkout, copied `.just` utility, or `cargo run` fallback.
- atm-core team QA approves and merges its own PR, then reruns the consumer
  acceptance matrix on `develop`.

## Required validation

- P.1 replay in an atm-core disposable copy and final consumer worktree
- Justfile/TOML/YAML parse checks and reviewed operation inventory
- `just setup`, `just lint`, `just test`, `just upgrade`
- atm-core's complete Linux/macOS/Windows CI matrix and its normal test gates
- explicit confirmation that no `ATM_HOME`/ATM runtime coupling was introduced

## This sprint does not close

- It does not merge through the sc-compose repository or make a sc-compose
  green run substitute for atm-core evidence.
- It does not permit a custom atm-core setup wrapper as a workaround.
