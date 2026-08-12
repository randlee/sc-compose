---
id: K.3
title: Var-File Decoding and Validation
phase: K
status: complete
branch: sprint/k-3-var-file-decoding
worktree: ../sc-compose-worktrees/sprint/k-3-var-file-decoding
target: integrate/phase-k
---

# Sprint K.3 — Var-File Decoding and Validation

## Purpose and evidence

Issue #311 ranks `crates/sc-compose/src/var_file.rs` at 3.91/10 with CCN 17 and 722 NLOC. It combines JSON/YAML decoding, lexical JSON integer protection, YAML merge-key scanning, duplicate-key visitors, object validation, and command-error mapping. The high branch count warrants a seam-first split rather than algorithm changes.

## Goal

Produce a production-ready format-specific decomposition of var-file
decoding, scanning, and validation without changing accepted input or
diagnostics.

## Required work

- Record the JSON and YAML characterization result against the Phase K
  baseline before moving implementation code.
- Implement only the seams listed under Exact targets and deliverables, retain
  format-specific behavior and errors, and rerun the characterization suite
  after the move.
- Record ownership and production-NLOC evidence and complete every command in
  Required validation before claiming closure.

## Hard dependencies

The hard dependencies are this sprint's plan-gate approval and
`integrate/phase-k` as the merge-forward target. There is no hard dependency on
another Phase K sprint.

## Production-ready expectation

Every deliverable listed below must land at production-ready quality for this
sprint's behavior-preserving scope. Partial module movement, test-only work,
or an unmeasured ownership split cannot satisfy the acceptance criteria.

## Exact targets and deliverables

- `crates/sc-compose/src/var_file.rs`, especially `decode_var_file`,
  `find_out_of_range_json_integer`, `find_yaml_merge_key`, `scan_yaml_line`,
  `unquoted_uncommented`, `decode_*_object`, `validate_var_object`, and
  duplicate-aware JSON visitor code.
- Create private decoding/scanning/validation modules while preserving `load_var_file`, `parse_var_file_contents`, `VarFileDecodeError`, and all existing command diagnostics.
- Add or strengthen characterization tests for JSON duplicate keys, i64/u64
  boundaries, YAML merge keys versus quoted `<<`, comments/block scalars,
  nested arrays/objects, top-level non-object values, and malformed input
  before moving code.

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

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-compose var_file::tests`
- `cargo test -p sc-compose --test cli -- var_file`
- `cargo test -p sc-compose --test json_cli -- var_file`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

Record unchanged values, diagnostics, and before/after production-NLOC
evidence.

## Completion evidence

- Baseline var-file characterization was captured before the move from the
  Phase K plan baseline (`76d6c7f`); the subsequent target merge-forward
  `4872876` added the already-merged K.1 XML decomposition and did not touch
  `var_file.rs`. Baseline results were green: 25/25 `var_file::tests`, 15/15
  CLI var-file tests, 2/2 JSON CLI var-file tests, formatting/diff checks,
  clippy, and the workspace suite. The characterization covers duplicate JSON
  keys, i64/u64 boundaries, YAML merge keys versus quoted `<<`, comments and
  block scalars, nested arrays/objects, top-level non-object values, malformed
  input, and source locations.
- Post-move results are unchanged: 25/25 `var_file::tests`, 15/15 CLI
  var-file tests, and 2/2 JSON CLI var-file tests passed. The full workspace
  suite passed 266/266 unit tests plus 51 extraction-integration and 16
  integration tests; formatting, diff, and clippy gates passed.
- Ownership evidence uses a simple nonblank, non-comment Rust-line count and
  reports production separately from the retained characterization tests. The
  baseline file was 827 lines / 722 counted lines overall, with 313 counted
  production lines before its test module. After the move, `var_file.rs` is
  460 / 401 overall and 71 production lines; private `var_file_decode.rs` is
  38 / 31, `var_file_json.rs` is 176 / 154, `var_file_yaml.rs` is 127 / 113,
  and `var_file_validate.rs` is 55 / 51. The largest production owner fell
  from 313 to 154 counted lines. The retained tests remain in `var_file.rs` as
  the contract characterization corpus rather than being deleted or moved to
  disguise ownership.
- Format boundaries remain explicit: JSON duplicate-aware visitation and
  integer scanning live in `var_file_json.rs`; YAML merge-key scanning and
  object decoding live in `var_file_yaml.rs`; shared decoded-object
  validation lives in `var_file_validate.rs`; and format dispatch plus the
  preserved `VarFileDecodeError` boundary live in `var_file_decode.rs` and
  `var_file.rs`. No generic JSON/YAML parser abstraction was introduced.
- Public/diagnostic surface review passed: `load_var_file`,
  `parse_var_file_contents`, `VarFileDecodeError`, parsed values, duplicate
  behavior, integer boundaries, YAML merge diagnostics, line/column locations,
  error codes, and error messages remain unchanged.

## Dependencies and non-closure

Independent from K.1-K.2 and K.4-K.8. No new var-file syntax or accepted input shapes are in scope.
