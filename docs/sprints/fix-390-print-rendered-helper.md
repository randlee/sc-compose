---
id: FIX-390-PRINT-RENDERED-HELPER
title: Extract print_rendered helper from report_cli_parse_error
status: complete
branch: fix/390-print-rendered-helper
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/390-print-rendered-helper
target: develop
---

## Root Cause

FIX-390-FOLLOWUP's QA (PR #393) passed with one new non-blocking finding
(simplification-reviewer): `report_cli_parse_error` in
`crates/sc-compose/src/main.rs` now contains the same 5-line
stderr/stdout print pattern twice — once in the early-return
display-request branch (`ErrorKind::DisplayHelp | ErrorKind::DisplayVersion`)
and once in the fall-through non-JSON usage-error branch:

```rust
if error.use_stderr() {
    eprint!("{rendered}");
} else {
    print!("{rendered}");
}
```

Low severity, no logic-drift risk, but flagged for dispatch per standing
policy of routing every QA finding.

## Fix Design

Extract a small private helper:

```rust
fn print_rendered(rendered: &str, use_stderr: bool) {
    if use_stderr {
        eprint!("{rendered}");
    } else {
        print!("{rendered}");
    }
}
```

Call it from both branches of `report_cli_parse_error`, eliminating the
duplication while preserving identical behavior.

## Required Changes / Tests

- `crates/sc-compose/src/main.rs`: add `print_rendered`, call it from both
  branches of `report_cli_parse_error`.
- No behavior change — existing tests in `crates/sc-compose/tests/cli.rs`
  and `main_tests` covering `--help`, `--version`, and usage-error exit
  codes/output must continue to pass unmodified.
- No new tests required (pure refactor, no new branches or logic).

## Out of Scope

- No changes to exit code semantics (already finalized by FIX-390 and
  FIX-390-FOLLOWUP).
- No changes to `CommandError` or `exit_codes`.

## Acceptance Criteria

- `report_cli_parse_error` contains no duplicated print logic.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and `cargo test --workspace` all pass.
- Existing CLI parse-error tests pass unmodified.

## References

- PR #393 quality report: https://github.com/randlee/sc-compose/pull/393#issuecomment-5260523644
- `docs/sprints/fix-390-followup-exit-code-source-of-truth.md`

## Priority

Low — code-quality cleanup, not release-blocking.

## Closeout Evidence

- implementation commit: `8a014e9`
- `print_rendered` is shared by both display-request and plain-text usage
  error paths; `report_cli_parse_error` contains no duplicated print block.
- existing CLI parse-error tests passed without modification.
- validation PASS: `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo test --workspace`.
