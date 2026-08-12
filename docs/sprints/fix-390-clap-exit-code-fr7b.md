---
id: FIX-390
title: "clap usage errors exit with code 2 instead of FR-7b's mandated code 3"
status: complete
branch: fix/390-clap-exit-code-fr7b
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/390-clap-exit-code-fr7b
target: crates/sc-compose/src/main.rs
---

## Root Cause

`docs/requirements.md` FR-7b mandates:

- `2` for validation or render failure
- `3` for usage, configuration, or contract error

`report_cli_parse_error` (`crates/sc-compose/src/main.rs:39-61`) already
distinguishes genuine usage errors from display requests via
`is_display_request` (matches only `ErrorKind::DisplayHelp | DisplayVersion`),
and already routes genuine usage errors through `CommandError::usage_with_code`
under `--json` — but the function's *return value* (the actual process exit
code) is always `error.exit_code()`, i.e. clap's own raw exit code (`2` for
any usage error), never `exit_codes::USAGE_FAIL` (`3`). This happens in both
`--json` and plain-text modes, since `exit_code` is computed once at the top
of the function and returned unconditionally at the bottom.

This is pre-existing (confirmed present at merge-base f497461, before the
FIX-385/386/252 campaign), was surfaced independently during FIX-386 QA
(ATM-QA-001, round 1; QA-386-006, round 3) and tracked as GitHub issue #390.
quality-mgr correctly kept it out of FIX-386's scope (that sprint was about
--json envelope suppression, not exit codes) and requested it be dispatched
as its own sprint.

## Fix Design

In `report_cli_parse_error`, return `exit_codes::USAGE_FAIL` (3) whenever
`!is_display_request`, instead of clap's raw `error.exit_code()`. Display
requests (`--help`, `--version`) keep clap's own exit code (0) unchanged in
both modes.

## Required Changes / Tests

- `crates/sc-compose/src/main.rs`: change `report_cli_parse_error` to return
  `exit_codes::USAGE_FAIL` for non-display clap errors, in both `--json` and
  plain-text modes.
- Add regression tests (likely in `crates/sc-compose/src/main_tests.rs` or
  `crates/sc-compose/tests/cli.rs`, matching existing conventions) asserting:
  - a genuine clap usage error (e.g. unknown flag, missing required arg)
    exits with code `3` under `--json`.
  - the same class of error exits with code `3` in plain-text mode (no
    `--json`).
  - `--help` / `--version` continue to exit `0` in both modes (no
    regression of FIX-386's fix).

## Out of Scope

- Any change to which arguments conflict or which values are accepted by
  parsing (`parse_var`, clap arg definitions, etc.).
- Any change to `is_display_request`'s matching logic (already correct,
  fixed in FIX-386).
- Any change to `CommandError::usage_with_code` or other call sites already
  using `exit_codes::USAGE_FAIL` correctly.

## Acceptance Criteria

1. A genuine clap usage error exits with code `3`, not `2`, under `--json`.
2. The same class of error exits with code `3` in plain-text mode.
3. `--help` / `--version` exit `0` in both modes (unchanged).
4. New regression tests cover all three cases above.
5. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
   and `cargo test --workspace` are clean.
6. `docs/project-plan.md` gets a `### Follow-on Fix Sprint: FIX-390` entry
   (planning-index gate), modeled on the existing FIX-385/386/252 entries.

## References

- `docs/requirements.md` FR-7b (Exit Codes)
- GitHub issue #390
- `crates/sc-compose/src/exit_codes.rs` (`USAGE_FAIL`)
- `crates/sc-compose/src/command_error.rs` (`usage_with_code`)

## Priority

Release-blocking — explicit user directive: do not release with this CLI
contract violation open.

## Closeout Evidence

- implementation commit: `0cfe85e` (`fix: map clap usage errors to exit code 3`)
- follow-up test commit: `15e17e1` (`test: update clap usage exit expectation`)
- `report_cli_parse_error` now maps non-display clap errors to
  `exit_codes::USAGE_FAIL` (`3`) in both JSON and plain-text modes; help and
  version display requests retain exit code `0`.
- regression coverage includes malformed `--var`, `--all`/`--brace-count`
  conflict, unknown flags, and help/version behavior in both output modes.
- validation PASS: `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo test --workspace`.
