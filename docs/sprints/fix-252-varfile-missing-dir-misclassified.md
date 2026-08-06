---
id: FIX-252
title: "Route missing/directory --var-file open errors through ERR_CONFIG_READ instead of ERR_CONFIG_PARSE"
status: assigned
branch: fix/252-varfile-missing-dir-misclassified
worktree: ../sc-compose-worktrees/fix/252-varfile-missing-dir-misclassified
target: develop
---

## Root Cause

`crates/sc-compose/src/var_file.rs::load_var_file` (lines 13-23) is the sole
entry point for reading a `--var-file` path. It maps *every* `io::Error` from
`std::fs::read_to_string` — missing path, directory-target, permission
denied, non-UTF8 content, all of it — to the same diagnostic code:

```rust
pub(crate) fn load_var_file(
    path: &Path,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let contents = std::fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!("failed to read var-file {}", path.display())),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    parse_var_file_contents(&contents)
}
```

`DiagnosticCode::ErrConfigParse` (`crates/sc-composer/src/diagnostics.rs:91,
205`) is documented and used elsewhere in this same file
(`var_file.rs::VarFileDecodeError::into_command_error`, lines 45-71) strictly
for cases where the file's *contents* were read successfully but failed to
decode as JSON/YAML. A missing path or a directory never reaches that
decoding stage at all — the failure is entirely at the `read_to_string`
open/read boundary, before any parsing is attempted. Emitting
`ERR_CONFIG_PARSE` for these cases tells the user their file has malformed
content when in fact it could not even be opened.

The repo already has the correct pattern for this exact class of
open/read-boundary error, just not applied here. `crates/sc-compose/src/commands/extract.rs::read_input`
(lines 37-46) reads a file and maps *any* `io::Error` (missing, permission,
directory, whatever `std::fs::read_to_string` returns) to
`DiagnosticCode::ErrConfigRead` uniformly, with an `InspectPath` recovery
hint:

```rust
fn read_input(path: &Path, label: &str) -> Result<String, CommandError> {
    std::fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!("failed to read {label} file {}", path.display())),
            DiagnosticCode::ErrConfigRead,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.to_owned(),
            })],
        )
    })
}
```

`load_var_file` should follow the same pattern: any `io::Error` surfacing
from the initial `read_to_string` call (before `parse_var_file_contents` is
even invoked) belongs to the read/open boundary, not the parse boundary, and
should carry `ErrConfigRead`, not `ErrConfigParse`.

## Exact Target

In `crates/sc-compose/src/var_file.rs`, change `load_var_file`'s error
mapping for the `read_to_string` call from `ErrConfigParse` to
`ErrConfigRead`, mirroring `extract.rs::read_input`'s pattern including the
`InspectPath` recovery hint:

```rust
pub(crate) fn load_var_file(
    path: &Path,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let contents = std::fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!("failed to read var-file {}", path.display())),
            DiagnosticCode::ErrConfigRead,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.to_owned(),
            })],
        )
    })?;
    parse_var_file_contents(&contents)
}
```

This requires importing `RecoveryHint`/`RecoveryHintKind` from `sc_composer`
in `var_file.rs` (already exported and used the same way in
`extract.rs:5-9`) and using `CommandError::usage_with_code_and_hints`
instead of `CommandError::usage_with_code` for this one call site.

No other call site in `var_file.rs` changes. `parse_var_file_contents` and
everything downstream of it (`decode_var_file`,
`VarFileDecodeError::into_command_error`, `validate_var_object`) keep using
`ErrConfigParse`/`ErrConfigVarfile` exactly as today — those are genuine
parse/validation-stage failures on content that was successfully read.

## This Sprint Does NOT Change

- `parse_var_file_contents` and `decode_var_file`'s format-detection and
  parse-error handling (lines 25-104) — a malformed-but-present JSON/YAML
  var-file must still surface `ErrConfigParse`/`ErrConfigVarfile` exactly as
  today. This sprint only touches the read/open boundary before parsing
  begins.
- `DuplicateAwareValueVisitor` (lines 502-586) and its lack of
  `visit_i128`/`visit_u128` — that is issue #254's scope, a separate
  worktree/sprint. Do not touch this visitor.
- `crates/sc-composer/src/resolver.rs` and `crates/sc-composer/src/include.rs`'s
  `io::Error` handling for template/include paths — that is issue #251's
  scope, a separate worktree/sprint. This sprint only touches
  `crates/sc-compose/src/var_file.rs::load_var_file`.
- The frontmatter YAML parse-error wrapping issue (#248, raw `serde_yaml`
  error text leaking through `ERR_CONFIG_PARSE` in `frontmatter/parser.rs`
  and `frontmatter/normalizer.rs`) — unrelated code path, separate
  worktree/sprint, not touched here.
- No change to `ErrConfigRead`'s existing behavior/message format anywhere
  else it is already used (`include.rs:134`, `init_workspace.rs:119`,
  `extract.rs:43`, `verify.rs:73-79`).

## Required Test Matrix

All new tests live in `crates/sc-compose/src/var_file.rs`'s existing
`#[cfg(test)] mod tests`.

(a) **Red-baseline regression test (the mandatory `#[ignore]`d test — see
Process section)**: call `load_var_file` with a path that does not exist and
assert the resulting `CommandError::diagnostic_code` is
`Some(DiagnosticCode::ErrConfigRead)`, not `ErrConfigParse`. Before the fix
lands, this assertion genuinely fails (`diagnostic_code` is
`Some(DiagnosticCode::ErrConfigParse)` today) — a normal in-process
assertion failure, not a crash.

(b) Call `load_var_file` with a path pointing at a directory (e.g. a
`tempfile::tempdir()`) and assert `diagnostic_code ==
Some(DiagnosticCode::ErrConfigRead)`.

(c) Existing malformed-but-present-content cases continue to return
`ErrConfigParse`/`ErrConfigVarfile` unmodified — covered by the existing
`decode_and_validation_preserve_source_specific_boundaries` and
`duplicate_keys_remain_rejected_at_decode_boundary` tests (lines 310-350),
which must still pass without modification.

(d) A valid, existing var-file path continues to load successfully —
covered by the existing `decoded_json_and_yaml_objects_share_validated_conversion`
test (lines 297-308), unmodified.

## Mandatory Process (two-commit red -> green, standing requirement)

Confirmed clean 3/3 on FIX-245, FIX-244, and FIX-247 — standing requirement
for this sprint too:

1. **First commit**: land test (a) above as `#[ignore]`d in `var_file.rs`'s
   test module. Team-lead independently confirms it genuinely fails (a
   normal `cargo test -p sc-compose -- --ignored <test_name>` run reporting
   a `FAILED` assertion, not a crash) before any fix code is written.
2. **Second commit**: land the `ErrConfigRead` + `InspectPath` fix plus test
   (b), and remove the single `#[ignore]` line from test (a). No other
   test-logic changes in this commit. Team-lead independently re-runs the
   same command and confirms it now passes.
3. Sprint-doc closeout narrative must state accurate, verifiable provenance
   — both tests are created fresh on this branch, never described as
   promoted from elsewhere.

## Acceptance Criteria

- `cargo test --workspace` passes, including the now-unignored test (a) and
  new test (b).
- A missing `--var-file` path and a directory passed as `--var-file` both
  return `Err` with `DiagnosticCode::ErrConfigRead`, not `ErrConfigParse`.
- All pre-existing `var_file.rs` unit tests continue to pass unmodified.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #252 can be closed referencing the merged PR.

## References

- GitHub issue #252
- `crates/sc-compose/src/var_file.rs` (`load_var_file`, lines 13-23; test
  module)
- `crates/sc-compose/src/commands/extract.rs` (`read_input`, lines 37-46 —
  the reference pattern being mirrored)
- `crates/sc-composer/src/diagnostics.rs` (`ErrConfigParse` line 91/205,
  `ErrConfigRead` line 89/204)
