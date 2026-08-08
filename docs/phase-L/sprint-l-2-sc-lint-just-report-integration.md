---
id: L.2
title: Shared sc-lint Runner, Reports, and Just Contract
phase: L
status: in-progress
branch: sprint/l-2-sc-lint-just-report-integration
worktree: ../sc-compose-worktrees/sprint/l-2-sc-lint-just-report-integration
target: integrate/phase-l
---

# Sprint L.2 — Shared sc-lint Runner, Reports, and Just Contract

## Goal

Create one reusable sc-compose CLI/reporting path and one standard just command
set for all sc-lint consumers, with no repository-specific Python runner.

## Hard Dependencies

L.1 must be merged to integrate/phase-l. L.3-L.16 are blocked only on this
sprint's generic runner contract, not on one another.

## Parallel Execution

This sprint has no parallel Phase L sprint. It runs serially after L.1 and
unblocks the mutually parallel target wave L.3-L.16. L.17 waits for that wave.

## Exact Targets

- crates/sc-compose/src/cli/mod.rs
- crates/sc-compose/src/commands/mod.rs
- crates/sc-compose/src/commands/sc_lint.rs
- crates/sc-compose/src/reporting/source_entry.rs
- crates/sc-compose/src/reporting/render_many.rs
- crates/sc-compose/src/commands/reports/render.rs
- crates/sc-compose/tests/sc_lint_runner.rs
- Justfile
- deny.toml
- .github/workflows/ci.yml
- .sc/sc-lint/targets/
- reports/inputs/lint/
- docs/phase-L/sc-lint-reporting-contract.md
- docs/adrs/0017-sc-lint-runner-allowlist-and-reporting.md

## Deliverables

- A CLI-owned sc-compose lint orchestration command that executes an allowlisted
  sc-lint command, passes --json --root, preserves exit status, captures
  stdout/stderr separately, and records raw JSON as an artifact.
- A declarative target registry at `.sc/sc-lint/targets/<id>.toml` with stable
  command IDs for every L.3-L.16 target. Each target sprint adds only its own
  descriptor/fixture and does not edit the shared runner.
- Generic JSON-source report ingestion and a single sc-compose HTML/XHTML
  materialization path that shows command, status, diagnostics, findings, and
  raw payload links. The renderer is the sole report materialization path;
  there is no unused alternate template.
- A canonical Justfile recipe set: lint, lint <target>, view findings,
  check <target>, clippy <target>, and ci, with the same names and default
  behavior in every consuming repository.
- CI wiring that runs just lint and publishes the generated report/raw JSON
  artifacts.
- ADR-0017 recording the command allowlist, subprocess boundary, JSON capture,
  report ownership, and security rationale for the shared runner.
- The ADR explicitly defers composite-profile fix ownership to the Phase L
  plan's Atomic target ownership section; that routing mechanism is outside
  the ADR's runner/security decision.

## Required Work

- Use the sc-lint JSON envelope and exit code as the source of truth; never
  scrape human text or reimplement analyzer behavior.
- Keep the command allowlist explicit so arbitrary process execution cannot be
  smuggled through a target descriptor.
- Make report generation generic over JSON payloads, so later target sprints do
  not add a new Python converter or report template.
- Keep Justfile recipes thin and deterministic. Feature sprints must not append
  one-off recipes or Python scripts.
- Ensure just lint defaults to sc-lint lint full, while explicit target recipes
  remain available for focused local and QA runs.

## sc-lint Reuse Reference

- Representative orchestration sources: `../sc-lint/.just/run_lint.py`,
  `../sc-lint/.just/python_adapter.py`,
  `../sc-lint/crates/sc-lint/src/contract.rs`, and
  `../sc-lint/crates/sc-lint/src/render.rs`.
- Reuse the sc-lint JSON/adapter/profile contract through the installed CLI;
  do not copy `run_lint.py`, `python_adapter.py`, or report converters into
  sc-compose. L.17 records the packaging gap and links maturin issue #83.

## sc-lint Cleanup Routing

Run all L.2 runner/Just/report targets on the final sprint commit. Fix minor
runner portability, JSON/report schema, identity-literal, or line-count
findings immediately. For remaining findings, create
`fix/l-2-<class>-<owner>` from this sprint worktree's final commit. Keep
report-schema changes, process/portability changes, constant strings, and
length refactors in separate worktrees; group constant strings by owning crate
and never by individual finding. Send each worktree and fix commit to
team-lead for PR creation; team-lead sends it to quality-mgr for QA. L.2
cannot close until all required fix PRs are QA-approved, merged, and rerun.

## Explicit Code Samples

The stable orchestration seam must be equivalent to:

~~~
pub(crate) fn run_sc_lint(
    root: &Path,
    command: ScLintCommand,
) -> Result<ScLintResult, CommandError>;
~~~

The consumer-facing recipes must be equivalent to:

~~~
just lint                 # sc-lint lint full
just lint fast|full|ci
just lint sc-boundary|sc-portability|sc-runtime|line-counts|identity-literals
just view findings
just check native|xwin
just clippy native|xwin
just ci                   # sc-lint ci
~~~

## This Sprint Does Not Close

- It does not implement analyzer rules or target-specific fixtures.
- It does not add view graph.
- It does not create a Python adapter per target; any such addition is out of
  scope and fails the sprint.

## Acceptance Criteria

- just lint invokes sc-lint 0.4.0 through the shared sc-compose path and
  produces one summary report plus raw JSON artifacts.
- The JSON envelope, command ID, diagnostics, finding count, stderr, and
  non-zero exit status are preserved for both pass and fail cases.
- A target descriptor can be added without modifying Justfile, the shared
  runner, or a shared report template.
- The recipe names and default profile match the Phase L command contract.
- A clean repository checkout can run the runner with no Python dependency
  beyond existing sc-compose project tooling.
- The CI setup action materializes the pinned sc-lint `.just/*.py` utilities at
  runner time when a Python-backed target needs them; no utility is copied into
  or maintained in this repository.

- All required cleanup fixes are QA-approved, merged, and revalidated before sprint closure.

## Required Validation

- just lint
- just lint fast
- just lint full
- just lint ci
- cargo test -p sc-compose --test sc_lint_runner
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
