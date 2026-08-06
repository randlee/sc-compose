---
id: FIX-273
status: in-progress
branch: fix/273-array-typed-vars-accept-scalars
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/273-array-typed-vars-accept-scalars
target: develop
---

# Sprint FIX-273 — Reject Scalar Input For Array-Only Required Variables

## Problem

Issue #273: when a `required_variables` entry has no dotted path (e.g. `items`,
not `items.id`) and the template only ever consumes it via `{% for x in items
%}`, passing a plain string or a single object instead of a JSON array is
silently accepted. A string is iterated character-by-character; an object is
iterated over its keys only (values silently dropped). Exit code is 0, no
diagnostics — silent data corruption.

Repro (issue #273, minimized):

```
[{% for x in items %}"{{ x }}"{% if not loop.last %}, {% endif %}{% endfor %}]
```
with `{"items": "ab"}` renders `["a", "b", ]` (also invalid JSON — trailing
comma) instead of failing.

Also reproduced against real production templates in this repo:
`examples/jagged-array-values.md.j2` (`rows`) and
`examples/changelog-categories.md.j2` (`categories`) — both already exist in
this repo (added in an earlier fuzz-round-2 investigation) and can be reused
directly as fixtures for this sprint's regression tests.

## Root cause

`crates/sc-composer/src/validation/required_paths.rs::validate_required_value`
only performs a shape check when there is at least one dotted path segment
remaining (`segments.split_first()`). For a required variable with **no**
dotted path (the common `{% for x in items %}` case), `segments` is empty on
the first call, `split_first()` returns `None` immediately, and the function
returns `RequiredPathStatus::Satisfied` for **any** JSON type — string,
number, object, or array. There is currently no static analysis of how a
required variable is actually consumed in the template body, so this gap is
structural, not a missed case in existing logic.

## Fix design (recommended; comp may adjust within these constraints)

Add a new, narrowly-scoped check that runs alongside (not instead of) the
existing dotted-path validation in `required_paths.rs`:

1. For each **top-level** required variable (`VariableName` with no `.` in
   it) that resolves to a present value (`MissingTopLevel` already handled
   elsewhere — do not duplicate that diagnostic), scan the **raw source
   text** of the declaring origin file (`state.required_origins[variable]`,
   same file already read by `required_variable_location`) for a `{% for
   ... in <name> %}` block whose iterable is a **bare identifier** exactly
   equal to the variable name — no dots, no filters (`| sort`), no function
   calls, no attribute/method access. Use a small hand-written scanner (not
   a new `regex` crate dependency — this repo has none today and the
   pattern is simple enough to tokenize by hand: find `{%`, optional `-`,
   whitespace, `for`, whitespace, one or two comma-separated identifiers,
   whitespace, `in`, whitespace, one identifier, optional whitespace,
   optional `-`, `%}`). Reuse/extend the existing lightweight text-scanning
   style already used in `required_variable_location` rather than pulling in
   a template-AST dependency.
2. If a matching bare-identifier for-loop is found for that variable name,
   and the resolved JSON value is present but **not** `serde_json::Value::
   Array`, emit a new diagnostic:
   - New `DiagnosticCode::ErrValArrayShapeMismatch` (`ERR_VAL_ARRAY_SHAPE_
     MISMATCH`) — add it next to `ErrValShapeMismatch` in
     `crates/sc-composer/src/diagnostics.rs` with a doc comment, both in the
     enum and in the `SCREAMING_SNAKE_CASE` match arm.
   - Message: `` required variable {name} is consumed as a list via `{% for
     %}` but received {json_type} `` where `{json_type}` is `string`,
     `number`, `boolean`, `null`, or `object` as appropriate — keep it
     terse and machine-greppable, consistent with the existing
     `ErrValShapeMismatch` message style.
   - Reuse `required_path_diagnostic`'s location-lookup pattern
     (`required_variable_location`) so the diagnostic still carries a
     line/column pointing at the `required_variables:` entry.
3. Do **not** apply this check when the required variable already has a
   dotted-path sibling entry that establishes array-of-object semantics
   (e.g. both `items` and `items.id` are declared) — the existing dotted
   check already covers that case end-to-end and takes precedence; only run
   the new check when the variable's declared required-path segment count
   is zero (a bare top-level entry).
4. Scope this to the declaring origin file only (matching the existing
   `required_variable_location` convention). Cross-file loop usage (the
   for-loop lives in a different included file than the one that declares
   `required_variables`) is out of scope for this sprint — note it as a
   known limitation in the sprint doc's Out of scope section, not something
   to silently half-support.

## Required tests (two-commit red green process: commit 1 = all failing, commit 2 = fix)

1. Exact issue repro: `required_variables: [items]`, template
   `{% for x in items %}...{% endfor %}`, input `{"items": "ab"}` (string) →
   `ErrValArrayShapeMismatch`, exit code non-zero, no rendered output.
2. Same template, input `{"items": {"key": "value"}}` (object) →
   `ErrValArrayShapeMismatch`.
3. Same template, input `{"items": ["a", "b"]}` (array) → still renders
   successfully, no diagnostic (regression guard against false positives).
4. A required variable that is **never** iterated (no `{% for %}` over it in
   the template) — passing a string or object for it must NOT trigger this
   new check (only existing behavior applies). Confirms the check is scoped
   to actual for-loop consumption, not blanket-applied to every required
   variable.
5. A required variable used both as `items` (bare, in a `{% for %}`) and
   with a dotted sibling `items.id` declared — array input with objects
   missing `id` must still surface the existing `ErrValMissingNestedField`
   from the dotted check, not be short-circuited by the new check.
6. Fixture-based regression using the repo's existing
   `examples/jagged-array-values.md.j2` and its sample vars file: swap
   `rows` to a bare string in a copied/temp vars payload and confirm
   `ErrValArrayShapeMismatch` fires through the full `sc-compose render`
   CLI path (integration-level, not just unit-level on
   `required_paths.rs`).
7. A for-loop whose iterable is **not** a bare identifier (e.g. `{% for x in
   items | sort %}` or `{% for x in items.values %}`) must NOT trigger the
   new check — confirms the scanner is conservative and doesn't misfire on
   filtered/attribute-qualified iterables.

## Out of scope

- Cross-file loop detection (for-loop in an included file, required
  variable declared in the root file, or vice versa).
- General template-AST-based usage analysis (e.g. via minijinja's unstable
  `machinery` module) — this sprint uses a conservative text scan, not a
  full parser. A follow-up issue may revisit this if the text-scan approach
  proves too narrow in practice.
- Detecting array-of-scalars vs array-of-objects distinctions beyond what
  the existing dotted-path check already does.
- Any change to `crates/sc-compose` CLI argument handling — this is a
  validation-only fix inside `sc-composer`.

## Acceptance criteria

- `cargo test --workspace` passes, including all 7 new regression tests
  listed above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #273's exact repro (`{{ x }}` for-loop over a string-valued
  `items`) is rejected with `ErrValArrayShapeMismatch` and a non-zero exit
  code — no corrupted output is produced.
- No new dependency added to `Cargo.toml` (the scanner is hand-written, not
  `regex`-based) unless comp determines during implementation that a
  hand-written scanner cannot reliably avoid both false positives and false
  negatives for the patterns above — if so, stop and report back before
  adding a dependency, don't add one silently.
- Sprint doc Closeout Evidence section records exact fix commit(s),
  validation results, and any deviation from the recommended design above
  with rationale.

## References

- Issue #273: https://github.com/randlee/sc-compose/issues/273
- `crates/sc-composer/src/validation/required_paths.rs`
  (`validate_required_value`, `validate_required_path`,
  `required_variable_location`)
- `crates/sc-composer/src/diagnostics.rs` (`DiagnosticCode`)
- `crates/sc-composer/src/validation/mod.rs` (`ValidationState`,
  `required_origins`)
- `examples/jagged-array-values.md.j2`, `examples/jagged-array-values.sample-vars.json`
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

_(to be completed by comp before requesting QA)_
