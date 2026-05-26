---
id: B2
title: Producer Recipes, Report-Init Scaffold, And Just Surface
status: draft
branch: plan/phase-B
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/phase-B
---

# Sprint B2 — Producer Recipes, Report-Init Scaffold, And Just Surface

## Goal

Implement the standard producer command contract so lint, test, smoke, and
repo-specific custom producers all generate evidence in the same shape, while
`just reports` remains the shared aggregator and verifier and `sc-compose`
provides the scaffold path that makes adoption easy.

## Hard Dependencies

- [docs/phase-A/sprint-A2.md](../phase-A/sprint-A2.md)
- [docs/phase-B/sprint-B1.md](./sprint-B1.md)

## Exact Targets

- `Justfile`
- `.claude/skills/reports-init/SKILL.md`
- `.claude/skills/reports-init/assets/reports.toml.j2`
- `.claude/skills/reports-init/assets/Justfile.append.j2`
- `crates/sc-compose/src/reporting/init.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/phase-B-plan.md`
- `docs/phase-B/sprint-B2.md`

## Deliverables

- one standard producer runtime and scaffold contract for:
  - `just lint`
  - `just test`
  - `just smoke`
  - repo-specific custom producers such as diagram or schema reports
- one explicit `just reports` contract for:
  - verify expected evidence exists
  - build or refresh a combined index if needed
  - print or summarize the latest report entrypoints/paths
- one `sc-compose reports init` CLI flow that scaffolds:
  - `reports/catalog/reports.toml`
  - `reports/latest/`
  - `reports/archive/`
  - `reports/templates/`
- one smoke test scaffold contract that defines:
  - a reference smoke template fixture
  - a sample `sample-vars.json` fixture
  - one `sc-compose reports smoke` CLI subcommand that accepts `--fixture`
    and `--vars` flags, runs the smoke fixture through the render pipeline,
    and emits results to `reports/latest/smoke/`
  - one harness wrapper entrypoint that the generated `just smoke` target
    invokes by calling that `sc-compose reports smoke` subcommand
- one bundled `.claude/skills/reports-init/` scaffold skill that emits:
  - `Justfile` stubs
  - report-catalog starter entries
  - directory layout
  - the smoke template fixture and sample vars fixture
  - TODO markers for consumer-owned scripts and templates
- one explicit statement that adding repo-specific producer commands must not
  require changing the shared aggregation contract
- one explicit note that wrapper-owned helpers such as `just reports-open` may
  exist locally but are not part of the shared command contract
- one explicit note that this sprint owns scaffold-generated `Justfile` stubs
  only; the real `sc-compose`-backed implementation of `just reports` and
  `just reports-verify` lands in B5
- one explicit producer ownership split:
  - the consumer repo owns `just lint`, `just test`, and `just smoke`
  - `sc-compose` owns the contract and scaffold skill that generate the stub
    shape
- one explicit note that `sc-compose` owns the smoke scaffold fixture shape
  and the harness wrapper, while the consumer repo owns the actual smoke test
  command body and any repo-specific templates
- one explicit note that the B8 backward-compat harness builds on this smoke
  scaffold contract so the same fixture shape is reused to verify that existing
  examples still pass after the shared runtime is adopted

## Explicit Code Samples

```make
lint:
	@echo "TODO: repo-owned lint producer"

test:
	@echo "TODO: repo-owned test producer"

smoke:
	sc-compose reports smoke --fixture reports/smoke/reference-template.html.j2 --vars reports/smoke/sample-vars.json

reports:
	sc-compose reports index --catalog reports/catalog/reports.toml
```

```toml
[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
```

## This Sprint Does Not Close

- source collection or render-many behavior
- template-family behavior
- panel chrome/copy behavior
- archive output policy
- publish-manifest behavior

## Acceptance Criteria

- the runtime makes producer recipes the owners of report generation
- `sc-compose reports init` creates a valid report scaffold in an empty repo
- the scaffolded catalog passes the B1 catalog validator
- the runtime reserves `just reports` for aggregation, verification, and
  deterministic latest-entrypoint/path reporting rather than primary
  generation
- the runtime allows repo-specific custom producers without changing the report
  discovery contract
- the scaffold explicitly documents that the consumer owns `just smoke`
  implementation
- the scaffold explicitly documents that `sc-compose` owns the shared contract
  and the scaffold skill, not the repo-specific producer body
- the scaffold defines the generated smoke fixture shape and the harness
  wrapper entrypoint that `just smoke` invokes

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
