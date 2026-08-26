---
id: R.2
title: Bead CLI and JSON Protocol
status: in_progress
branch: sprint/r-2-bead-cli-and-json-protocol
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/r-2-bead-cli-and-json-protocol
target: integrate/phase-r
depends_on: R.1
---

# Sprint R.2 — Bead CLI and JSON Protocol

## Goal

Expose the R.1 host-neutral integration through a thin, scriptable
`sc-compose bead` command without duplicating rendering, process, or Beads
policy in the CLI.

## Exact targets

- `crates/sc-compose/Cargo.toml`
- `crates/sc-compose/src/cli/{schema,capability}.rs`
- `crates/sc-compose/src/commands/{dispatch,bead}.rs`
- `crates/sc-compose/docs/manual/bead.md`
- `crates/sc-compose/src/help_topics.rs`
- `crates/sc-compose/tests/cli/bead.rs`
- `crates/sc-composer-beads/tests/fixtures/beads/request.json` — R.2-owned
  canonical CLI-request fixture; additive only, with no R.1 formula fixture
  changed.
- `docs/{requirements,architecture}.md` — normative CLI surface and command
  mapping for the shipped `bead` command.

## Deliverables

1. Add one `bead` command with subcommands `render`, `validate`,
   `preview-pour`, and `pour`. Each subcommand accepts exactly one complete
   `--request <JSON>` file and emits the R.1 receipt with `--json`. The request
   file is the only machine protocol; do not add a competing partial flag
   protocol in this sprint.
2. Make human output a concise stage summary with rendered path, Beads version,
   and per-stage state. Machine output is exactly one existing diagnostic JSON
   envelope whose payload is the `sc-compose/beads/v1` receipt.
3. Map invalid request, render failure, unavailable `bd`, failed `cook`, failed
   preview, and refused/failed pour to the `BeadComposeError` stable codes and
   `BeadOutcome`/`BeadStageReceipt` defined in ADR-0021, with non-zero exit
   codes. Do not print raw subprocess output outside the JSON envelope or
   human diagnostic stream.
4. Add a manual that documents the two variable namespaces, triple braces for
   sc-compose expansion, the `bd where` active-registry requirement and TOML/
   JSON collision rule for preview/pour, and the irreversible nature of
   authorized `pour`.
5. Add CLI tests that use the R.1 fake runner to prove request loading,
   envelope shape, exit mapping, and no execution past a failed stage; retain
   one pinned-`bd` end-to-end CLI fixture in CI. The test loads the canonical
   `crates/sc-composer-beads/tests/fixtures/beads/` files directly; R.1 owns
   updates when the shared contract changes.

## Acceptance criteria

- [ ] `sc-compose bead validate --request fixture.json --json` produces a
      receipt matching the R.1 library result.
- [ ] `preview-pour` cannot run before a successful validation and reports
      exactly which Beads stage failed.
- [ ] `pour` without the typed authorization value refuses before starting
      `bd`; CLI tests prove it.
- [ ] The CLI contains no Beads argv construction, formula parsing, or
      duplicated stage logic; it only deserializes, calls R.1, and presents the
      resulting receipt.
- [ ] CLI success and failure envelopes preserve the ADR-0021
      `BeadStageReceipt`, `BeadOutcome`, and `BeadComposeError` definitions
      without introducing CLI-local error variants or codes.
- [ ] The manual is reachable through `sc-compose help bead`.

The acceptance criteria will be checked again after the repaired Windows
fake-`bd` fixture has passed its hosted CI rerun.

## Required validation

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p sc-compose --test bead
cargo test --workspace
sc-compose bead validate --request crates/sc-composer-beads/tests/fixtures/beads/request.json --json
sc-compose bead preview-pour --request crates/sc-composer-beads/tests/fixtures/beads/request.json --json
```

Also require `git diff --check`.

## Validation evidence

Validated on 2026-08-25 at `e221919` before this evidence-only update:

- `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passed.
- `cargo test -p sc-compose --test bead` passed all 11 CLI-request and
  error-envelope tests. The Windows-specific fake-`bd` recheck is pending the
  current hosted CI rerun.
- `SC_LINT_SOURCE_ROOT=/Users/randlee/Documents/github/sc-lint cargo test
  --workspace` passed. The explicit source-root setting supplies the existing
  external sc-lint Python utilities required by the workspace test harness.
- In an isolated `bd init` workspace with the pinned local `bd`, the built
  `sc-compose` binary successfully ran `bead validate --request request.json
  --json` and `bead preview-pour --request request.json --json`. The receipts
  confirmed the expected render, cook, active-registry, and dry-run-pour
  stages.
- `git diff --check` passed.

## Out of scope

Ad-hoc CLI flags that bypass the request schema, automatic formula installation,
Python bindings, and any Beads-side `bd compose` command remain out of scope.
