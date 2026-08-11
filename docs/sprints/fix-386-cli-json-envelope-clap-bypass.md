---
id: FIX-386
title: "clap argument-parsing errors (--var, --all/--brace-count conflicts) bypass the --json DiagnosticEnvelope contract"
status: complete
branch: fix/386-cli-json-envelope-clap-bypass
worktree: ../sc-compose-worktrees/fix/386-cli-json-envelope-clap-bypass
target: develop
---

## Root Cause

Per `docs/requirements.md` FR-8a, all CLI `--json` output — including
CLI-usage/argument-parsing errors — must be delivered as the versioned
`DiagnosticEnvelope` on stdout. Two distinct clap mechanisms currently bypass
this, both exiting via clap's own error path *before* sc-compose's
application-layer `--json` rendering logic ever runs:

1. **FUZZ-002** — `crates/sc-compose/src/cli/pass_input.rs::parse_var`
   (registered as `CommonArgs.vars`'s clap `value_parser`) rejects a
   malformed `--var novalue` (missing `key=value`). Repro:
   `sc-compose validate --json --var novalue` — empty stdout, plain-text
   stderr, `--json` ignored.
2. **FUZZ-003** — `crates/sc-compose/src/cli/schema.rs` declares `--all` as
   `conflicts_with_all` against `--brace-count`/`--variable-delimiters`.
   Repro: `sc-compose render --json --all --brace-count 3 --file t.j2 --root <root>`
   — same bypass.

Same root-cause family: **any `clap::Error` currently bypasses the `--json`
output contract**, regardless of which clap mechanism produced it. Fixed
together deliberately — a single fix target, not two independent branches,
to avoid two people racing the same `main.rs` code path.

## Fix design

Detect `--json` early from raw argv (alongside the existing `--pass`
raw-argv pre-scan already in `main.rs`/`filtered_args_for_clap`) and, on any
`clap::Error` — covering both custom `value_parser` failures and
argument-group conflicts — wrap the rendered clap usage text inside a
`DiagnosticEnvelope` with an appropriate `ErrConfigParse`-family diagnostic
code instead of letting clap print to stderr and exit directly.

Non-`--json` invocations of the same malformed arguments must keep clap's
normal human-readable usage output unchanged.

## Required changes / tests

1. `crates/sc-compose/tests/fuzz_regressions.rs::malformed_var_argument_does_not_bypass_the_json_output_contract`
   and `::all_and_brace_count_conflict_does_not_bypass_the_json_output_contract`
   (already added as RED regression tests) go GREEN without weakening their
   assertions.
2. `cargo test --workspace`, `cargo fmt --all --check`,
   `cargo clippy --all-targets --all-features -- -D warnings`: PASS.
3. Positive control: default (non-`--json`) CLI usage-error output is
   unchanged.

## Out of scope

- `crates/sc-composer/src/renderer.rs` / `template_init.rs` JSON round-trip
  bug (issue #385, FUZZ-001) — separate root cause, separate branch.
- Any change to which arguments conflict or which values `parse_var` accepts
  — only how the resulting error is surfaced under `--json`.

## Acceptance criteria

- Both promoted regression tests go GREEN.
- `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`: PASS.
- Closeout Evidence records the fix commit(s).
- Planning index gate: `docs/project-plan.md` includes the sprint entry
  before closeout.

## References

- Issue #386: https://github.com/randlee/sc-compose/issues/386
- Fuzz findings FUZZ-002, FUZZ-003, campaign report
  `site/reports/20260811-3-fuzz-report.json`
- `docs/requirements.md` FR-8a

## Closeout Evidence

- Fix commits: `f68079e`, `e79b279`, and QA follow-up `9299344`.
- Raw argv JSON detection now covers clap parser failures before application
  dispatch, while non-JSON invocations retain clap's normal output stream and
  exit code.
- JSON clap failures are rendered as `ERR_CONFIG_PARSE` diagnostics inside the
  standard `DiagnosticEnvelope` on stdout.
- Regression coverage for malformed `--var` input and the `--all`/
  `--brace-count` conflict passes.
- Clap display requests (`--help` and `--version`) remain plain stdout output
  with exit code 0 even when `--json` is present.
- Validation passed: `cargo test --workspace`, `cargo fmt --all --check`, and
  `cargo clippy --all-targets --all-features -- -D warnings`.

## Priority

Fuzz-discovered production bug; dispatched immediately alongside issue #385.
