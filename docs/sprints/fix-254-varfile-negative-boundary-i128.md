---
id: FIX-254
title: "Add i128/u128 visitor coverage so var-file integers outside i64/u64 fail closed instead of silently corrupting"
status: complete
branch: fix/254-varfile-negative-boundary-i128
worktree: ../sc-compose-worktrees/fix/254-varfile-negative-boundary-i128
target: develop
---

## Root Cause

`crates/sc-compose/src/var_file.rs::DuplicateAwareValueVisitor` (lines
502-586) implements `serde::de::Visitor` for JSON var-file decoding with
`visit_bool`, `visit_i64`, `visit_u64`, `visit_f64`, `visit_str`,
`visit_string`, `visit_none`, `visit_unit`, `visit_seq`, `visit_map` — but
no `visit_i128` or `visit_u128` (confirmed by reading the full `impl`
block; no such methods exist).

`serde_json`'s `deserialize_any` calls `visit_i128`/`visit_u128` for
integer literals outside the signed/unsigned 64-bit range in either
direction. Because `DuplicateAwareValueVisitor` doesn't override those
methods, `serde::de::Visitor`'s default trait implementations run instead,
and their default behavior for `visit_i128`/`visit_u128` is to convert to
`f64` and delegate to `visit_f64` (`var_file.rs:535-542`), which succeeds
and silently produces a lossy floating-point approximation instead of an
error.

This was empirically reproduced on this branch (throwaway test against
`parse_var_file_contents`, removed before commit):

- JSON `{"n": -9223372036854775809}` (`i64::MIN - 1`) →
  `Ok(Number(-9.223372036854776e18))` — silently corrupted, matching the
  issue body's claim for the negative boundary exactly as issue #241
  documented for the positive boundary (`u64::MAX + 1` also reproduces as
  `Ok(Number(1.8446744073709552e19))`, confirming this is the same code
  path, not sign-specific).
- YAML `n: -9223372036854775809` (`i64::MIN - 1`) → `Err(ErrConfigParse)`
  with message `invalid type: integer '-9223372036854775809' as i128,
  expected any YAML value` — `serde_yaml::Value`'s own deserializer (not
  this crate's visitor) rejects the value outright because
  `serde_yaml::Value::Number` has no representation below `i64::MIN`, even
  though it represents unsigned values up to `u64::MAX` correctly. This
  confirms the issue body's "fails closed in YAML" claim.
- YAML `n: 18446744073709551615` (`u64::MAX`, in-range positive) →
  `Ok(Number(18446744073709551615))` — correct, for contrast.

So the root cause is exactly as the issue describes and is independent of
`#241`'s positive-boundary fix: **it is a visitor-completeness gap in
`DuplicateAwareValueVisitor`, not a sign-specific bug**, but it only
manifests as a *silent* corruption on the JSON side because
`serde_yaml::Value` already fails closed for both boundaries on its own
(this crate's YAML path, `input_value_from_yaml`, never sees an
out-of-range integer — `serde_yaml::from_str` errors before that point).
The user-visible defect is therefore JSON-only; the "YAML asymmetry"
described in the issue is between JSON's silent corruption and YAML's
existing (correct) failure, not a YAML-side bug requiring a code change.

Existing error plumbing: `parse_var_file_contents` →
`decode_var_file` → `parse_json_value_rejecting_duplicate_keys`
(`var_file.rs:493-500`) surfaces any `serde_json::Error` from the visitor
as `VarFileDecodeError::InvalidFormat`, which
`VarFileDecodeError::into_command_error` (`var_file.rs:45-71`) maps to
`DiagnosticCode::ErrConfigVarfile` — the same code the JSON
top-level-non-object case uses (see `NotAnObject { format: Json }`,
`var_file.rs:58-63`). No new `DiagnosticCode` variant is needed.

## Exact Target

Add `visit_i128` and `visit_u128` to `DuplicateAwareValueVisitor` in
`crates/sc-compose/src/var_file.rs`, each returning an explicit
`E::custom(...)` error for any value that doesn't fit `i64`/`u64`, instead
of falling through to the lossy `visit_f64` default:

```rust
fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
where
    E: DeError,
{
    i64::try_from(v)
        .map(|v| serde_json::Value::Number(v.into()))
        .map_err(|_| {
            E::custom(format!(
                "integer {v} is outside the representable range \
                 ({min}..={max})",
                min = i64::MIN,
                max = u64::MAX,
            ))
        })
}

fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
where
    E: DeError,
{
    u64::try_from(v)
        .map(|v| serde_json::Value::Number(v.into()))
        .map_err(|_| {
            E::custom(format!(
                "integer {v} is outside the representable range \
                 ({min}..={max})",
                min = i64::MIN,
                max = u64::MAX,
            ))
        })
}
```

Both methods first try to narrow into the crate's existing representable
range (`i64` for negative values that fit, `u64` for values that fit
there) so genuinely in-range values that merely arrive via the `i128`/
`u128` visitor callback (e.g. a value at exactly `i64::MIN` presented as
`i128`) still succeed instead of spuriously erroring — only values outside
`i64::MIN..=u64::MAX` produce `E::custom`.

The resulting `serde_json::Error` propagates through the existing
`VarFileDecodeError::InvalidFormat` → `DiagnosticCode::ErrConfigVarfile`
path unchanged — no new error variant, no new diagnostic code, no changes
to `decode_var_file`, `into_command_error`, or any call site outside
`DuplicateAwareValueVisitor`'s `impl` block.

No YAML-side code change: `serde_yaml`'s own deserializer already fails
closed for both boundaries via the existing `ErrConfigParse` path
(`decode_var_file`, `var_file.rs:98-99`); this sprint does not touch
`decode_yaml_object` or `input_value_from_yaml`.

## This Sprint Does NOT Change

- The positive/`u64` boundary fix scoped to issue #241 — that fix lives on
  its own independent branch/worktree (`fix/241-json-var-int-overflow`,
  not read or touched by this sprint). This sprint's `visit_u128` change
  is expected to overlap textually with whatever #241 lands (both add a
  `visit_u128` override to the same struct) — that overlap is a future
  merge-conflict-resolution concern for whichever branch merges second,
  not something to coordinate or avoid here. This sprint must be correct
  and mergeable to `develop` on its own regardless of #241's landing order.
- `decode_yaml_object`, `input_value_from_yaml`, or any YAML-side parsing —
  confirmed via direct testing that YAML already fails closed at both
  boundaries with no code change needed.
- `validate_input_value`, `VariableName`, or any downstream validation
  logic in `sc-composer`.
- Any new `DiagnosticCode` variant — `ErrConfigVarfile` is reused as-is.
- Any change to `visit_i64`, `visit_u64`, `visit_f64`, or any other
  existing visitor method already correctly handling in-range values.

## Required Test Matrix

a. JSON var-file with a value at exactly `i64::MIN - 1`
   (`-9223372036854775809`) → `Err` with `diagnostic_code ==
   Some(DiagnosticCode::ErrConfigVarfile)` (not silently corrupted to a
   float, not `ErrConfigParse`).
b. JSON var-file with a value at exactly `u64::MAX + 1`
   (`18446744073709551616`) → same outcome as (a) — confirms the fix is
   not sign-specific (this reproduces the same defect issue #241 targets
   from the positive side; verifying it here guards against a regression
   if this sprint lands before #241 and #241's own fix is later merged on
   top).
c. JSON var-file with `i64::MIN` exactly (`-9223372036854775808`, in
   range) → succeeds, decodes to the correct integer, unchanged from
   current behavior.
d. JSON var-file with `u64::MAX` exactly (`18446744073709551615`, in
   range) → succeeds, decodes to the correct integer, unchanged from
   current behavior.
e. YAML var-file with `i64::MIN - 1` → still `Err` with
   `Some(DiagnosticCode::ErrConfigParse)`, unchanged from current
   (pre-fix) behavior — confirms this sprint does not alter the YAML path.
f. Existing `var_file.rs` test-module tests (duplicate-key rejection,
   merge-key rejection, object-shape errors, etc.) continue to pass
   unmodified.

## Mandatory Process

Two-commit red→green regression test, standing requirement across the
fuzz-queue (confirmed clean 3/3 on FIX-245, FIX-244, FIX-247): land an
`#[ignore]`d regression test for case (a) (and ideally (b)) in
`var_file.rs`'s existing `#[cfg(test)] mod tests` FIRST, in its own commit.
Team-lead independently confirms it genuinely fails before any fix code is
written — this red baseline is a normal in-process assertion failure (the
test asserts `diagnostic_code == Some(ErrConfigVarfile)` or asserts the
value is *not* the lossy float, and that assertion currently fails because
today's actual result is `Ok` with a corrupted float) — no crash-detection
special-casing needed, unlike FIX-247.

Then land the `visit_i128`/`visit_u128` fix plus the remaining test-matrix
cases and remove the single `#[ignore]` line, in a second commit with no
other test-logic changes.

## Acceptance Criteria

- `cargo test --workspace` passes, including all Required Test Matrix
  cases (a)-(f).
- JSON var-files with integers outside `i64::MIN..=u64::MAX` return
  `Err(DiagnosticCode::ErrConfigVarfile)` instead of silently succeeding
  with a lossy float, at both the negative and positive boundaries.
- In-range boundary values (`i64::MIN`, `u64::MAX` exactly) continue to
  decode correctly with no behavior change.
- YAML var-file behavior at both boundaries is unchanged (still fails
  closed via `ErrConfigParse`).
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #254 can be closed referencing the merged PR.

## Closeout Evidence

Status: **complete**.

- Red baseline: `4d47d10` (`test: reproduce out-of-range JSON integer
  handling`). The negative and positive boundary cases initially succeeded
  with lossy floating-point values.
- Green implementation: `690b85d` (`fix: preserve default JSON number
  rendering`). The visitor now has explicit `visit_i128`/`visit_u128`
  narrowing, and a post-parse lexical check rejects out-of-range integer
  literals with `ErrConfigVarfile` without changing serde_json/minijinja's
  normal number representation. `b37d017` contains the final lint-only
  cleanup.
- Reverted experiment: `7c88aed` enabled serde_json `arbitrary_precision`,
  but full-workspace validation exposed internal number-marker objects in
  minijinja output; `690b85d` reverted that approach.
- QA follow-up: the retained i128/u128 visitor callbacks are documented as
  defense-in-depth for a currently unreachable serde_json dispatch path. The
  lexical scanner remains the primary enforcement gate, with direct tests for
  quoted digit runs and visitor narrowing/error behavior.
- Full validation at the final branch state: `cargo test --workspace`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo fmt --all --check`, and `git diff --check`: PASS.

The implementation differs from the sprint document's assumption that the
default serde_json `deserialize_any` path directly invokes the i128/u128
visitor callbacks for oversized literals; the actual dependency narrows those
literals to f64 first. The retained boundary check is therefore necessary to
meet the same fail-closed contract without enabling dependency-wide arbitrary
precision behavior that changes minijinja rendering. YAML parsing remains
unchanged. All regression tests were created fresh on this branch.

## References

- GitHub issue #254
- GitHub issue #241 (related, positive-boundary sibling — independent fix,
  separate worktree `fix/241-json-var-int-overflow`, not touched by this
  sprint)
- `crates/sc-compose/src/var_file.rs` (`DuplicateAwareValueVisitor`,
  lines 502-586; `VarFileDecodeError`, lines 32-72;
  `parse_json_value_rejecting_duplicate_keys`, lines 493-500)
- `crates/sc-composer/src/diagnostics.rs` (`DiagnosticCode::ErrConfigVarfile`,
  line 93 / 206)
