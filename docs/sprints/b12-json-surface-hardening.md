---
id: B12
title: JSON Surface Hardening
status: planned
branch: feat/b12-json-surface-hardening
worktree: ../sc-compose-worktrees/feat/b12-json-surface-hardening
target: integrate/phase-B
---

# Sprint B12 — JSON Surface Hardening

## Goal

- Normalize all JSON and JSONL path fields to forward slashes across the remaining Phase B surfaces.
- Close the Windows-sensitive `templates add --json` path-contract gap found in production-readiness review.
- Add regression coverage that asserts path format directly instead of only asserting field presence.
## Hard Dependencies

- `integrate/phase-B` at the current merged Phase B tip.
- Production-readiness finding that `templates add --json` still emits `source` / `destination` via `Path::display().to_string()`.
## Exact Targets

- `crates/sc-compose/src/commands/templates.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `crates/sc-composer/src/validate.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/src/path_utils.rs`

Phase B branch note:

- Exact Targets are verified against `integrate/phase-B`, which is the target
  branch for this cleanup work.
- `crates/sc-composer/src/validate.rs` and
  `crates/sc-compose/src/path_utils.rs` already exist on that Phase B line even
  though they may not exist on older `develop` or `main` baselines.
## Deliverables

- `templates add --json` emits normalized `source` and `destination` paths via `to_forward_slash()`.
- Any remaining validation-related JSON or JSONL path surfaces on the `integrate/phase-B` line are normalized to forward slashes.
- `crates/sc-compose/tests/json_cli.rs` asserts path formatting for `templates add --json` instead of only checking `name` and `changed`.
- Windows-sensitive path regression coverage exists for every JSON surface touched by the sprint.
## Required Work

- Replace `display().to_string()` in `crates/sc-compose/src/commands/templates.rs` JSON output with `to_forward_slash()`.
- Inspect the current `integrate/phase-B` validation and JSONL output path emitters and route any remaining path strings through `to_forward_slash()`.
- Extend `crates/sc-compose/tests/json_cli.rs` so `source` and `destination` are asserted explicitly and remain normalized under Windows path conventions.
- Keep the sprint focused on machine-readable surfaces; do not expand it into unrelated text-output cleanup.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.


```rust
print_json(
    serde_json::json!({
        "source": to_forward_slash(&result.source),
        "destination": to_forward_slash(&result.destination),
        "changed": result.changed,
    }),
    Vec::new(),
)?;
```

## This Sprint Does Not Close

- No new command surface.
- No rendering-behavior changes outside JSON/JSONL path formatting.
- No repo-wide text-output normalization pass.
## Acceptance Criteria

- JSON and JSONL output paths always use forward slashes on Windows and Unix-like platforms.
- Tests assert path format directly for the touched surfaces.
- No touched JSON surface regresses existing field names or schema shape.
- `cargo test --workspace` passes on the implementation branch that lands the hardening changes.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
