---
id: K.3
title: Var-File Decoding and Validation
phase: K
status: planned
branch: sprint/k-3-var-file-decoding
worktree: ../sc-compose-worktrees/sprint/k-3-var-file-decoding
target: integrate/phase-k
---

# Sprint K.3 — Var-File Decoding and Validation

## Purpose and evidence

Issue #311 ranks `crates/sc-compose/src/var_file.rs` at 3.91/10 with CCN 17 and 722 NLOC. It combines JSON/YAML decoding, lexical JSON integer protection, YAML merge-key scanning, duplicate-key visitors, object validation, and command-error mapping. The high branch count warrants a seam-first split rather than algorithm changes.

## Exact targets and deliverables

- `crates/sc-compose/src/var_file.rs`, especially `decode_var_file`,
  `find_out_of_range_json_integer`, `find_yaml_merge_key`, `scan_yaml_line`,
  `unquoted_uncommented`, `decode_*_object`, `validate_var_object`, and
  duplicate-aware JSON visitor code.
- Create private decoding/scanning/validation modules while preserving `load_var_file`, `parse_var_file_contents`, `VarFileDecodeError`, and all existing command diagnostics.
- Characterize JSON duplicate keys, i64/u64 boundaries, YAML merge keys versus quoted `<<`, comments/block scalars, nested arrays/objects, top-level non-object values, and malformed input before moving code.

## Planned seam

Decoding remains format-specific and validation remains the command-facing
boundary. The private split must preserve these existing signatures:

```rust
pub(crate) fn parse_var_file_contents(
    contents: &str,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError>;
fn decode_var_file(contents: &str) -> Result<DecodedVarObject, VarFileDecodeError>;
fn validate_var_object(
    object: DecodedVarObject,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError>;
```

Private decoder/scanner/visitor modules may be introduced, but no generic
JSON/YAML parser abstraction, accepted shape, or source-specific error path is
introduced. No existing var-file source path is deleted or renamed.

## Acceptance criteria

- Parsed values, duplicate-key behavior, integer range behavior, YAML merge-key diagnostics, line/column locations, and error codes/messages are unchanged.
- JSON and YAML scanners remain format-specific; no generic parser abstraction is introduced without measured duplication and contract tests.
- Production-NLOC evidence shows reduced mixed ownership and all tests remain in the appropriate module.
- No scanner or visitor is removed without an equivalent characterization
  result; test relocation does not count as decoder decomposition by itself.

## Required validation

Run `cargo test -p sc-compose var_file::tests`, `cargo test -p sc-compose
--test cli -- var_file`, and `cargo test -p sc-compose --test json_cli --
var_file` against the baseline before the move and rerun the same commands
after the move. Then run `cargo fmt --all --check`, `git diff --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test
--workspace`. Record unchanged values, diagnostics, and before/after
production-NLOC evidence.

## Dependencies and non-closure

Independent from K.1-K.2 and K.4-K.8. No new var-file syntax or accepted input shapes are in scope.
