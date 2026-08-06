---
id: FIX-246
title: "--strict validation is delimiter-blind: ignores active --variable-delimiters/--brace-count"
status: assigned
branch: fix/246-strict-ignores-custom-delimiters
worktree: ../sc-compose-worktrees/fix/246-strict-ignores-custom-delimiters
target: develop
---

# Sprint FIX-246 — `--strict` validation is delimiter-blind

## Goal

Fix GitHub issue #246: under custom `--variable-delimiters`/`--brace-count`,
`--strict` validation always scans for hardcoded default `{{ }}` tokens
regardless of what delimiters the render actually uses. This produces both
a false positive (inert literal `{{x}}` text flagged as undeclared under
active `<<..>>` delimiters) and a false negative that is materially worse:
an actually-referenced undeclared variable through the active custom
delimiters is silently rendered as an empty string and `--strict` passes
with exit 0 — `--strict` provides no protection at all for custom-delimiter
renders today.

## Hard Dependencies

- `develop` branch at HEAD (no blocked sprints)

## Exact Targets

- `crates/sc-compose/src/commands/compose.rs` (`execute_custom_delimiter_render`)
- `crates/sc-composer/src/validation.rs` (or wherever `validate()`/
  `validate_with_observer()` performs undeclared-token scanning)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint,
the sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- Root cause fixed: undeclared-token validation must scan using the same
  active delimiters (`--variable-delimiters`/`--brace-count`, or defaults if
  neither is set) that the render itself uses. No render path may validate
  against a different delimiter configuration than it renders with.
- `execute_custom_delimiter_render` in `crates/sc-compose/src/commands/compose.rs`
  must apply the caller's delimiters before calling into validation, not
  after — validation needs delimiter awareness added if it doesn't already
  support parameterized delimiters.
- False positive fixed: literal `{{x}}` body text must NOT be flagged as an
  undeclared token when `<<..>>` (or any other non-`{{ }}`) delimiters are
  active and `{{x}}` is inert text under those delimiters.
- False negative fixed: an undeclared variable referenced through the
  active custom delimiters (e.g. `<<undeclared>>`) MUST fail `--strict`,
  matching the default-delimiter baseline behavior for `{{undeclared}}`.
- Regression tests promoted from `crates/sc-compose/tests/fuzz_regressions.rs`:
  `strict_validation_with_custom_delimiters_does_not_flag_literal_default_delimiter_text`
  and `strict_validation_with_custom_delimiters_catches_undeclared_custom_delimiter_reference`
  — remove their `#[ignore]` attribute once the fix lands; both must pass.
- No change to default-delimiter (`{{ }}`) validation behavior — verify by
  re-running the existing validation/CLI test suites clean.

## Required Work

- Read `crates/sc-compose/tests/fuzz_regressions.rs` for the two `#[ignore]`d
  regression tests (`boundary-probe-04`/`boundary-probe-05`) and confirm the
  exact reproduction shape before changing code.
- Trace `execute_custom_delimiter_render` -> `validate_with_observer` ->
  `validate()` to find where delimiter information is dropped.
- Thread the active delimiter configuration through to undeclared-token
  scanning so it uses the same delimiters as the actual render pass.
- Remove `#[ignore]` from the two promoted regression tests once green.
- Add or extend a CLI integration test if the existing regression tests
  don't already exercise the exact `--strict --json --variable-delimiters`
  CLI invocation end-to-end.

## Explicit Code Samples

False positive (must go from failing to passing — should NOT fail strict):

```
---
name: t
version: 1.0.0
format: markdown
required_variables:
  - name
---
<<name>>{{x}}
```

`sc-compose render --file t.j2 --var name=World --variable-delimiters "<<" ">>" --strict --json --root <root>`
today: exit 2, `ERR_VAL_UNDECLARED_TOKEN: undeclared referenced token: x`.
After fix: exit 0, `{{x}}` renders as literal text.

False negative (must go from silently passing to correctly failing):

```
---
name: t
version: 1.0.0
format: markdown
required_variables:
  - name
---
<<name>><<undeclared>>
```

`sc-compose render --file t.j2 --var name=World --variable-delimiters "<<" ">>" --strict --json --root <root>`
today: exit 0, no diagnostics, `<<undeclared>>` silently renders empty.
After fix: exit 2, `ERR_VAL_UNDECLARED_TOKEN: undeclared referenced token: undeclared`.

## This Sprint Does Not Close

- No change to default-`{{ }}`-delimiter validation behavior
- No change to `--brace-count` multi-pass rendering semantics themselves,
  only to what delimiters validation scans against
- No broader validation/diagnostic schema changes beyond delimiter awareness

## Acceptance Criteria

- `cargo test --workspace` passes, including the two promoted regression
  tests (no longer `#[ignore]`d)
- Both explicit code samples above behave exactly as described after the fix
- Existing default-delimiter validation/CLI test suites remain green
- GitHub issue #246 can be closed referencing the merged PR

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
