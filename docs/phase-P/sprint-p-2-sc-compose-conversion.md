---
id: P.2
title: sc-compose Released-Consumer Conversion
phase: P
status: planned
target: develop
---

# Sprint P.2 — sc-compose Released-Consumer Conversion

## Goal

Apply the P.1-proven release plan to a dedicated sc-compose worktree, preserving
consumer-owned behavior while removing the known 0.4 integration workaround.

## Hard dependencies

- P.1 dual-reference matrix PASS and accepted ADR-0016/0017 amendment;
- same released sc-lint artifact, request schema, and request selection used in
  the approved sc-compose qualification cell.

## Exact targets

- `sc-lint.toml`
- `Justfile` and only product-approved `.sc-lint/` managed assets
- `.github/actions/setup-sc-lint/action.yml`
- `.github/workflows/ci.yml`
- `.sc/sc-lint/targets/` and `reports/latest/sc-lint/` only as decided by the
  accepted ADR amendment and P.1 preview
- `docs/phase-P/qualification/` final sc-compose evidence

## Deliverables

- One configuration version authority and released-artifact setup path.
- Removal of the source-archive download/copy utility fallback and temporary
  `lint-ci-consumer` workaround only when P.1 proves the replacement covers its
  required behavior.
- A Justfile integration that preserves consumer-owned recipes and comments
  outside the reviewed transformation. No whole-file replacement is allowed.
- CI that executes the same configured setup/lint/test contract and retains
  required consumer evidence artifacts without recreating a private runner.

## Acceptance criteria

- The real worktree preview is semantically identical to the accepted P.1
  sc-compose preview except for recorded baseline drift; unexpected drift stops
  the sprint and reruns qualification.
- `just setup`, `just lint`, `just test`, and `just upgrade` work from a clean
  checkout using the released artifact path only.
- A post-apply reapply produces no file changes; the source scan finds no
  active source archive/copy utility/temporary 0.4 workaround in setup/lint/CI.
- No Cargo dependency or sc-lint orchestration enters `sc-composer` or
  `bindings/python`; all CLAUDE.md boundary rules and the amended ADRs pass.

## Required validation

- P.1 replay against the final worktree
- Justfile/TOML/YAML parse checks and reviewed diff inventory
- `just setup`, `just lint`, `just test`, `just upgrade`
- full Linux/macOS/Windows CI matrix
- `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check`

## This sprint does not close

- It does not modify atm-core or assert that atm-core acceptance is implied by
  sc-compose success.
- It does not delete an unclassified `.just` helper or consumer report asset.
