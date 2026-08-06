---
id: FIX-250
title: ERR_CONFIG_VARFILE top-level-not-an-object message wording differs between JSON and YAML var-files
status: complete
branch: fix/250-varfile-object-wording-inconsistent
worktree: ../sc-compose-worktrees/fix/250-varfile-object-wording-inconsistent
target: develop
---

# Sprint FIX-250 — Unify `ERR_CONFIG_VARFILE` top-level-not-an-object wording

## Goal

Fix GitHub issue #250: `ERR_CONFIG_VARFILE` uses different wording for the
same underlying condition — "the var-file's top-level value is not a
key/value mapping" — depending on which format the input happened to parse
as. A JSON var-file whose top level is a scalar or array gets one message;
a YAML var-file whose top level is a scalar or sequence gets a different
message, even though both are the identical failure mode (not a mapping)
surfaced through the same diagnostic code.

## Hard Dependencies

- `develop` branch at HEAD (no blocked sprints)

## Root Cause

`crates/sc-compose/src/var_file.rs`:

- `VarFileDecodeError::NotAnObject { format: VarFileFormat }` (lines 36,
  39-43) is constructed at two call sites:
  - `decode_json_object` (lines 214-219): when `value` is not
    `serde_json::Value::Object`, returns
    `NotAnObject { format: VarFileFormat::Json }`.
  - `decode_yaml_object` (lines 231-236): when `value` is not
    `serde_yaml::Value::Mapping`, returns
    `NotAnObject { format: VarFileFormat::Yaml }`.
- `VarFileDecodeError::into_command_error` (lines 45-71) turns each variant
  into the final user-facing message, and this is where the wording
  diverges:
  - JSON case (lines 58-63): `anyhow!("var-file must be a JSON object")`
  - YAML case (lines 64-69): `anyhow!("var-file must be a JSON or YAML object")`

Both branches raise `DiagnosticCode::ErrConfigVarfile`. The JSON message
names only JSON ("must be a JSON object"), which is accurate for a file that
parsed as JSON but wasn't an object. The YAML message says "must be a JSON
or YAML object" — mentioning JSON even though the input was recognized and
parsed as YAML (JSON never entered into this branch at all, since
`decode_var_file`, lines 93-103, only reaches `decode_yaml_object` after
`parse_json_value_rejecting_duplicate_keys` at line 94 has already failed).
The two messages describe the identical condition (top-level value isn't a
mapping) with format-specific, inconsistent, and in the YAML case
misleading text.

## Exact Target

- `crates/sc-compose/src/var_file.rs`, `VarFileDecodeError::into_command_error`
  (lines 58-69 specifically) — collapse both `NotAnObject` arms to emit one
  shared, format-neutral message.

Before:

```rust
Self::NotAnObject {
    format: VarFileFormat::Json,
} => CommandError::usage_with_code(
    anyhow!("var-file must be a JSON object"),
    DiagnosticCode::ErrConfigVarfile,
),
Self::NotAnObject {
    format: VarFileFormat::Yaml,
} => CommandError::usage_with_code(
    anyhow!("var-file must be a JSON or YAML object"),
    DiagnosticCode::ErrConfigVarfile,
),
```

After (illustrative — exact wording may be adjusted for house style, but
must be identical text regardless of `format`):

```rust
Self::NotAnObject { .. } => CommandError::usage_with_code(
    anyhow!("var-file top-level value must be an object (JSON) or mapping (YAML)"),
    DiagnosticCode::ErrConfigVarfile,
),
```

The `VarFileFormat` field on `NotAnObject` becomes unused once both arms
collapse to identical text; either drop the field (and simplify
`decode_json_object`/`decode_yaml_object` to construct a unit variant), or
keep it only if it is still needed elsewhere (it is not — grep confirms
`VarFileFormat` is referenced only at the two construction sites and the two
match arms being unified). Prefer removing the now-dead field/enum rather
than leaving an unused discriminant, since `cargo clippy -D warnings` will
flag an unused field/variant if nothing else reads it.

## This Sprint Does NOT Change

- No change to `UnsupportedYamlMergeKey` or `InvalidFormat` message wording
  (lines 48-57) — those are different failure modes, not part of this bug.
- No change to `decode_json_object` / `decode_yaml_object`'s detection logic
  for what counts as "not an object" — only the message text produced from
  the already-detected condition.
- No change to `validate_var_object` (lines 248+) or any other
  `ErrConfigVarfile` call site (key-must-be-string, invalid key name,
  duplicate-key rejection, etc.) — those already use consistent, format-
  neutral wording today and are out of scope.
- No change to `DiagnosticCode::ErrConfigVarfile`'s string representation
  (`diagnostics.rs:206`).

## Required Test Matrix

All four cases below must produce byte-identical `ErrConfigVarfile` message
text (differing only in nothing format-specific):

- (a) JSON var-file, top-level scalar (e.g. `"hello"` or `42`)
- (b) JSON var-file, top-level array (e.g. `[1, 2, 3]`)
- (c) YAML var-file, top-level scalar (e.g. `hello` or a bare `42`)
- (d) YAML var-file, top-level sequence (e.g. `- a\n- b\n`)

Plus no-regression checks:

- (e) A valid JSON object var-file still loads successfully (unaffected)
- (f) A valid YAML mapping var-file still loads successfully (unaffected)
- (g) `UnsupportedYamlMergeKey` and `InvalidFormat` messages are unchanged
  (spot-check one case of each)

## Mandatory Process

The two-commit red→green regression-test process is mandatory (standing
requirement for this queue, confirmed clean 3/3 on FIX-245, FIX-244, and
FIX-247):

1. First commit: add a failing `#[ignore]`d regression test to
   `crates/sc-compose/tests/fuzz_regressions.rs` (or a var-file-focused unit
   test in `var_file.rs`'s own test module if one exists — check first)
   asserting that the JSON top-level-scalar case and the YAML
   top-level-scalar case produce the same message text. This is a normal
   in-process assertion failure (comparing two `CommandError` message
   strings for equality) — **not** a crash-mode test like FIX-247's; no
   special SIGABRT/stack-overflow verification is needed here. Confirm the
   test genuinely fails against current `develop` code (the messages differ:
   "var-file must be a JSON object" vs "var-file must be a JSON or YAML
   object"), commit and push it alone.
2. Second commit: collapse the two `NotAnObject` match arms to one shared
   message, remove `#[ignore]`, confirm the test passes, commit and push.
   No other test-logic changes in this commit.

## Acceptance Criteria

- `cargo test --workspace` passes, including the now-unignored regression
  test and the full test matrix above.
- All four not-an-object cases (JSON scalar, JSON array, YAML scalar, YAML
  sequence) produce identical `ErrConfigVarfile` message text.
- `UnsupportedYamlMergeKey` and `InvalidFormat` messages are unchanged.
- Valid JSON-object and YAML-mapping var-files continue to load
  successfully.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #250 can be closed referencing the merged PR.
- Sprint doc closeout narrative accurately describes the two-commit
  red→green trail on this branch.

## Closeout Evidence

Status: **complete**.

- Red baseline: `8d4edd7` (`test: reproduce varfile object wording
  mismatch`). The fresh ignored regression test was independently confirmed
  failing before implementation: JSON produced `var-file must be a JSON
  object`, while YAML produced `var-file must be a JSON or YAML object`.
- Green implementation: `41e3208` (`fix: unify varfile object diagnostics`).
  The `NotAnObject` variant is now format-neutral, the unused `VarFileFormat`
  enum/field is removed, and the full required matrix covers JSON/YAML scalar
  and sequence inputs, valid object inputs, and unchanged unrelated errors.
- Focused var-file tests: PASS.
- `cargo test --workspace`, `cargo clippy --all-targets --all-features
  -- -D warnings`, `cargo fmt --all --check`, and `git diff --check`: PASS.

All regression tests were created fresh on this branch and were not promoted
from another worktree.

## References

- GitHub issue #250
- `crates/sc-compose/src/var_file.rs` (`VarFileDecodeError`,
  `into_command_error` lines 45-71, `decode_json_object` lines 214-229,
  `decode_yaml_object` lines 231-246)
- `crates/sc-composer/src/diagnostics.rs:93,206` (`DiagnosticCode::ErrConfigVarfile`, unchanged)
