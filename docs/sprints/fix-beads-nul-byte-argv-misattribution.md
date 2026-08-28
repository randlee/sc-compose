---
status: complete
branch: fix/beads-nul-byte-argv-misattribution
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/beads-nul-byte-argv-misattribution
---

# FIX-BEADS-FUZZ-ENC-001: NUL-byte bead-variable misattributed as bd-unavailable

## Source

PR #564 (`fuzz/beads-integration-campaign`), Phase 2 custom
execution/filesystem-safety fuzz campaign finding, pinned as
`nul_byte_poisoned_bead_variable_value_is_misreported_as_bd_unavailable` in
`crates/sc-composer-beads/src/execute.rs`.

## Problem

A NUL byte in a `bead_variable` value caused `Command::spawn` to fail with
`InvalidInput`, which `run_stage_with_output` misreported as
`BeadComposeError::BdUnavailable` / `BEADS_BD_UNAVAILABLE` even though `bd`
was available.

## Resolution

Bead variable values are validated before execution and rejected with
`BEADS_VARIABLE_VALUE_INVALID`, including the offending variable and escaped
value. Any remaining `InvalidInput` process error is reported as
`BEADS_PROCESS_ARGUMENT_INVALID` rather than `BEADS_BD_UNAVAILABLE`.

## Validation

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`

## References

- PR #564: https://github.com/randlee/sc-compose/pull/564
- `crates/sc-composer-beads/src/execute.rs`
