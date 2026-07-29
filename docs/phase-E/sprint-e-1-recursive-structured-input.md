---
id: E.1
title: Recursive Structured Input Support
status: planned
branch: sprint/e-1-recursive-structured-input
worktree: ../sc-compose-worktrees/sprint/e-1-recursive-structured-input
target: develop
---

# Sprint E.1 — Recursive Structured Input Support

## Goal

Support finite, recursively nested JSON/YAML-compatible render values through
`--var-file` and the existing structured-input defaults. The primary user case
is `categories[].items[]` (an array of objects nested inside an array of
objects); the same implementation must support jagged scalar arrays and mixed
recursive values because Minijinja already accepts the underlying value model.

This is an implementation sprint. It lands production Rust behavior, tests,
fixtures, and aligned contract documentation.

## Hard Dependencies

- [GitHub issue #157](https://github.com/randlee/sc-compose/issues/157)
- [Phase E plan](./phase-E-plan.md)
- [Issue #157 implementation plan](../plan-157-nested-array-var-file.md)
- [requirements.md](../requirements.md), [architecture.md](../architecture.md),
  and [project-plan.md](../project-plan.md)
- current `InputValue` validation in
  `crates/sc-composer/src/types.rs`
- current JSON/YAML var-file tests in
  `crates/sc-compose/tests/cli.rs`

## Exact Targets

- `crates/sc-composer/src/types.rs`
- `crates/sc-composer/src/diagnostics.rs`
- `crates/sc-compose/src/var_file.rs` (only if ingress diagnostics or tests
  require a call-site adjustment)
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-composer/src/lib.rs` or the owning library unit-test module when
  public recursive-input behavior needs direct coverage
- `examples/changelog-categories.md.j2`
- `examples/changelog-categories.sample-vars.json`
- `examples/jagged-array-values.md.j2`
- `examples/jagged-array-values.sample-vars.json`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`
- `docs/error-code-registry.md`
- `README.md`
- `CHANGELOG.md`
- `docs/phase-E/sprint-e-1-recursive-structured-input.md`

## Deliverables

- `E1-D1` — replace the depth-sensitive `ArrayContext` validation policy with a
  recursive walk over every array element and object value while retaining
  `InputValue = serde_json::Value`.
- `E1-D2` — prove ingress parity for JSON var-files, YAML var-files, frontmatter
  defaults, and `template.json` `input_defaults`; preserve top-level var-file
  object and YAML string-key validation.
- `E1-D3` — make the four issue #157 repro fixtures successful regression
  examples: nested categories/items and jagged scalar rows.
- `E1-D4` — retain `DiagnosticCode::ErrValNestedArrayUnsupported` for public API
  compatibility but stop emitting it for supported recursive values; document
  its legacy/reserved status.
- `E1-D5` — align requirements, architecture, project-plan, diagnostic
  registry, README, and CHANGELOG text with the new recursive-value contract.
- `E1-D6` — add a dated architecture decision-record entry in
  `docs/architecture.md` that explicitly supersedes the historical H2
  nested-array restriction and names the new recursive-value contract.

The deliverable list above is authoritative for E.1 closure.

## Contract and implementation shape

Keep the existing public value type and validation entry point:

```rust
pub type InputValue = serde_json::Value;

pub fn validate_input_value(
    value: &InputValue,
) -> Result<(), InvalidInputValueError>;
```

The helper should no longer receive array-depth context:

```rust
fn validate_input_value_at(
    value: &InputValue,
) -> Result<(), InvalidInputValueError> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => Ok(()),
        Value::Array(values) => values.iter().try_for_each(validate_input_value_at),
        Value::Object(object) => object.values().try_for_each(validate_input_value_at),
    }
}
```

The exact implementation may use equivalent idiomatic code, but it must not
reintroduce an array-depth or array-member-type restriction. Keep the var-file
document itself a top-level JSON/YAML object, keep YAML map keys string-only,
and keep `--var` string-only. No renderer or resolver redesign is needed.

## Scope boundaries

Accept at any nested depth:

- scalars and `null`;
- objects with string keys;
- arrays of scalars or objects;
- arrays of arrays, including jagged arrays;
- mixed finite arrays and objects containing any accepted value.

Do not add schema validation, deep merging, bracket-path syntax, or a new
resource limit in this sprint. Existing parser and renderer behavior governs
ordinary finite trees; a separate security/resource sprint is required if
future evidence shows a concrete burden.

## Test plan

Add or update tests at both library and CLI boundaries:

| Case | Expected result |
| --- | --- |
| `categories[].items[]` JSON fixture | nested loops render category/item/PR output |
| `rows: [[1,2,3],[4,5]]` JSON fixture | both jagged rows render |
| equivalent nested YAML input | same semantics as JSON input |
| nested frontmatter default | accepted and rendered |
| nested `template.json` default | accepted and rendered |
| arrays nested through multiple objects/arrays | accepted without reserved error |
| top-level scalar/sequence var-file | remains `ERR_CONFIG_VARFILE` |
| YAML non-string map key | remains `ERR_VAL_OBJECT_SHAPE` |
| malformed JSON/YAML | retains existing parse/config diagnostics |

Replace the current nested-array rejection test with positive coverage; do not
simply delete the test. Assert stable output for successful render cases and
stable diagnostic codes for preserved negative boundaries.

## This Sprint Does Not Close

- adversarial campaign orchestration, which belongs to E.2 and E.3;
- arbitrary schema or template-language extensions;
- production fixes for bugs discovered by the later fuzz campaign;
- Python binding or external adapter behavior;
- ATM integration or runtime-specific configuration.

## Acceptance Criteria

- `categories[].items[]` renders successfully from the committed issue fixture
  through `sc-compose render --var-file`.
- Jagged scalar arrays render successfully from the committed issue fixture.
- JSON and YAML var-files, frontmatter defaults, and `template.json` defaults
  use the same recursive acceptance policy.
- Existing top-level-object, YAML string-key, malformed-input, and variable
  validation boundaries remain enforced with their stable diagnostics.
- No supported recursive value emits `ERR_VAL_NESTED_ARRAY_UNSUPPORTED`.
- The five historical H2 restriction locations carry explicit supersession
  notes naming the historical issue-plan label; the current phase index calls
  this work E.1 to avoid collision with completed Phase D.
- A dated architecture decision-record entry explicitly supersedes the H2
  restriction and is linked from the affected architecture sections.
- README and CHANGELOG are checked and updated when they describe the old
  restriction; otherwise the sprint records that no user-facing text exists.
- Required validation passes with no known release-blocking finding.

## Required Validation

Run from the E.1 worktree:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
```

Also run the concrete repository boundary suite
`cargo test -p sc-compose --test repo_boundaries`, both issue fixture commands,
and the new YAML/default-source regression tests.
