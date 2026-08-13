# Plan: Support recursively nested var-file arrays (#157)

## Status and intent

This is an implementation plan only. It addresses GitHub issue #157 and the
follow-up direction that callers should be able to use the recursive data
shapes they would reasonably put in a JSON or YAML var-file.

The recommended scope is broader than only `categories[].items[]`: accept any
finite, recursively nested JSON-compatible value tree (scalars, objects with
string keys, and arrays containing any supported value). This includes both
the issue's primary object-nested-in-object case and the secondary jagged
scalar-array case.

The var-file document itself remains a top-level JSON/YAML object, and YAML
maps must continue to use string keys. Those are input-format constraints,
not nested-array restrictions.

## Root cause

`sc-composer` defines `InputValue` as `serde_json::Value`, which already has
the representation needed for recursive arrays and objects. The renderer
passes the resulting `BTreeMap<String, serde_json::Value>` to Minijinja, so
there is no renderer or context-type limitation preventing nested traversal.

The rejection is introduced by `validate_input_value` in
`crates/sc-composer/src/types.rs`:

- `validate_input_value` starts the walk with `ArrayContext::TopLevel`.
- `validate_input_value_at` rejects an array element that is itself an array.
- It rejects an object element when the containing array is not at the
  top-level variable boundary.
- Object fields are recursively visited with `ArrayContext::Nested`, so an
  object field containing any array is rejected once that array contains an
  array or an object.

The two ingress paths then expose the same restriction:

- JSON var-files call `validate_input_value` from
  `crates/sc-compose/src/var_file.rs::parse_json_object_value`.
- YAML var-files recursively convert through
  `sc_composer::input_value_from_yaml`, which validates the converted value.

The same public validation gate is also used by frontmatter defaults in
`crates/sc-composer/src/frontmatter/normalizer.rs::normalize_frontmatter` and
`template.json` defaults in
`crates/sc-compose/src/template_store.rs::validate_manifest_defaults`.
Broadening the shared contract therefore keeps all structured-input sources
consistent instead of making `--var-file` behave differently from defaults.

## Proposed data-model and validation change

No new data type or renderer abstraction is needed. Keep:

```rust
pub type InputValue = serde_json::Value;
```

Replace the depth-sensitive validation walk in
`crates/sc-composer/src/types.rs` with a shape-preserving recursive walk:

1. Scalars and `null` are valid leaves.
2. Arrays recursively validate every element, regardless of array depth.
3. Objects recursively validate every value, regardless of object depth.
4. No value-shape error is emitted solely because an array is nested.

The `ArrayContext` enum and its parameter can be removed if the resulting
implementation no longer needs them. Keep the existing validation function
as the public gate so all callers continue to receive one consistent policy.

`input_value_from_yaml` should retain its existing conversion behavior:

- recursively convert sequences and mappings to `serde_json::Value`;
- continue rejecting non-string YAML mapping keys with
  `ERR_VAL_OBJECT_SHAPE`;
- run the same recursive input validation after conversion.

`parse_var_file_contents` should retain its existing top-level-object checks,
variable-name validation, JSON/YAML selection, and error wrapping. Only the
nested-value policy changes.

### Shape policy after the change

Accepted values include, at any nesting depth:

- scalars and `null`;
- objects/maps with string keys;
- arrays of scalars;
- arrays of objects;
- arrays of arrays, including jagged arrays;
- objects containing any of the above;
- mixed arrays containing combinations of supported values.

The accepted value is a finite JSON-compatible tree. This does not introduce
schema validation, bracket-path syntax, deep merging, or new `--var`
parsing. It only removes the artificial array-depth restriction from the
existing value model.

### Minijinja compatibility rationale

Minijinja already renders serializable recursive sequences and mappings, and
the current renderer passes `serde_json::Value` directly as the context. The
implementation should therefore follow the library's capability boundary:
if a value is representable by the existing JSON/YAML conversion and
Minijinja can traverse it, input validation should accept it. There is no
benefit to preserving a narrower array-depth rule. A future restriction would
need a concrete burden such as an incompatible representation, a security
limit, or a measurable resource problem; issue #157 provides no such burden.

### Diagnostic compatibility

Retain `DiagnosticCode::ErrValNestedArrayUnsupported` in
`crates/sc-composer/src/diagnostics.rs` for public API and serialized-code
compatibility. Removing a public enum variant would be an unnecessary API
break. Update its documentation to describe it as a legacy/reserved code (or
otherwise make clear that ordinary nested arrays are no longer a current
validation failure). The implementation should stop emitting it for valid
recursive values.

If a future input-shape rule needs a stable error, it should use a distinct
diagnostic rather than reusing this now-retired restriction code.

## Primary scope: nested arrays of objects

The first acceptance target is the issue's categorized changelog shape:

```json
{
  "categories": [
    {
      "name": "Added",
      "items": [
        { "summary": "New feature", "pr": 588 }
      ]
    }
  ]
}
```

The implementation must allow loops over both levels and field access inside
the inner object, for example `category.items`, `item.summary`, and
`item.pr`. This should work through `--var-file` without flattening or
template-side grouping.

## Secondary scope: jagged scalar arrays

Support the issue's second repro in the same change:

```json
{ "rows": [[1, 2, 3], [4, 5]] }
```

It is not structurally harder once validation is recursive. Treating it as a
separate test target is still useful because it proves the fix is not narrowly
special-cased for arrays of objects.

## Concrete files and functions to touch during implementation

Production code:

- `crates/sc-composer/src/types.rs`
  - simplify `validate_input_value` / `validate_input_value_at`;
  - update function/type documentation;
  - replace the rejection unit test with recursive acceptance coverage.
- `crates/sc-composer/src/diagnostics.rs`
  - update the stale nested-array diagnostic documentation while retaining the
    public code for compatibility.

Likely test/docs updates:

- `crates/sc-compose/tests/cli.rs`
  - replace `render_rejects_nested_arrays_in_var_file_with_reserved_code`;
  - add end-to-end JSON and YAML render coverage for nested object arrays and
    jagged scalar arrays;
  - add coverage for nested values supplied through frontmatter and
    `template.json` defaults, since they share the validator.
- `crates/sc-composer/src/types.rs` unit tests
  - cover nested arrays, arrays nested in objects, arrays of objects nested in
    arrays of objects, and mixed recursive values;
  - preserve rejection coverage for non-string YAML map keys.
- `examples/changelog-categories.md.j2` and
  `examples/changelog-categories.sample-vars.json`
  - committed in this planning worktree as the primary issue regression
    fixture; update the descriptive text from "currently fails" when the
    implementation lands and the example becomes successful.
- `examples/jagged-array-values.md.j2` and
  `examples/jagged-array-values.sample-vars.json`
  - committed in this planning worktree as the secondary issue regression
    fixture; update the descriptive text from "currently rejected" when the
    implementation lands.

Documentation and release metadata:

- `docs/requirements.md`
  - revise FR-1b/FR-13 and the initial-release out-of-scope statements;
  - describe recursive arrays as supported while retaining the top-level
    object and string-key rules.
- `docs/architecture.md`
  - update the `InputValue` contract, structured-input ingress rules, and
    validation-impact sections;
  - remove the H1/H2 nested-array prohibition and stale future-design claims.
- `docs/project-plan.md`
  - update the completed structured-input acceptance notes that currently say
    nested arrays remain rejected.
- `docs/error-code-registry.md`
  - mark `ERR_VAL_NESTED_ARRAY_UNSUPPORTED` as legacy/reserved or otherwise
    document that it is no longer emitted for recursive arrays.
- `README.md` / `CHANGELOG.md`
  - search for user-facing shape restrictions before implementation;
  - add a release-note entry if the project's release convention requires one.
  - No README change is expected if it contains no current restriction text.

## Test plan

### Unit coverage

1. Accept `[[1, 2], [3]]`.
2. Accept an object containing an array of objects whose members contain an
   inner array of objects.
3. Accept a mixed recursive tree containing scalars, objects, arrays, and
   empty arrays at multiple depths.
4. Preserve acceptance of existing scalar arrays and top-level arrays of
   objects.
5. Preserve rejection of YAML mappings with non-string keys.

### CLI regression coverage

1. Render the primary issue fixtures:
   `examples/changelog-categories.md.j2` with
   `examples/changelog-categories.sample-vars.json`; assert success and the
   category/item/pr output. These files are committed alongside this plan and
   currently reproduce the validation failure before the production change.
2. Render the secondary issue fixtures:
   `examples/jagged-array-values.md.j2` with
   `examples/jagged-array-values.sample-vars.json`; assert success and both
   rows. These files are committed alongside this plan and currently
   reproduce the validation failure before the production change.
3. Add a YAML var-file equivalent for at least the primary shape so the JSON
   and YAML conversion paths are both exercised.
4. Verify nested arrays supplied through frontmatter defaults and
   `template.json` `input_defaults` follow the same policy.
5. Keep a negative test for a non-object top-level JSON/YAML var-file and for
   invalid YAML map keys, confirming the change does not weaken file-shape
   validation.
6. Run the existing workspace checks: `cargo fmt --all`,
   `cargo test --workspace`, and
   `cargo clippy --all-targets --all-features -- -D warnings`.

The current nested-array rejection test should not merely be deleted; it
should be replaced with positive coverage so the new contract is protected.

## Adversarial QA plan for quality-mgr

This change is a good candidate for an independent attempt to break the new
recursive-value contract. The implementation is intentionally permissive, so
the QA goal is to find both values that Minijinja can render but validation
still rejects and values that validation accepts but rendering mishandles.

quality-mgr should own a separate adversarial report and should not rely only
on the developer's hand-written issue fixtures. If the team has capacity,
quality-mgr may split the following probes across several interchangeable QA
agents, then consolidate all findings into one PASS/FAIL report:

1. **Recursive-shape agent** — generate or hand-author JSON trees with arrays
   nested in objects, arrays nested in arrays, objects nested at several array
   depths, empty arrays/objects, and mixed arrays containing scalars, objects,
   `null`, and arrays. Render templates that loop at every relevant level and
   access fields in inner objects.
2. **Ingress-parity agent** — run equivalent JSON and YAML var-files, then
   repeat representative shapes through frontmatter defaults and
   `template.json` `input_defaults`. Confirm every ingress path uses the same
   acceptance policy and produces equivalent output.
3. **Boundary-negative agent** — try to weaken constraints that must remain:
   top-level scalar/array var-files instead of objects, non-string YAML map
   keys, malformed JSON/YAML, invalid variable names, and unrelated missing or
   shape-mismatch diagnostics. These must continue to fail with their existing
   stable codes.
4. **Rendering-stress agent** — exercise empty and jagged dimensions,
   optional/missing fields, numeric and boolean leaves, Unicode strings,
   deeply nested but realistic trees, nested loops, and mixed whitespace.
   Compare rendered output against a small expected-output oracle.
5. **Contract/regression agent** — search the repository for stale claims that
   nested arrays are forbidden, run the full workspace checks, and verify the
   four committed issue fixtures are unchanged except for any intentional
   post-fix description wording.

The adversarial matrix should include at least these cases:

| Shape | Expected result |
| --- | --- |
| `[[1, 2], [3]]` | renders successfully |
| `[{"items": [{"name": "x"}]}]` | nested loop and field access succeed |
| `{"a": {"b": [{"c": [[true, null]]}]}}` | renders successfully |
| `[[], [{}], ["text", 3, false]]` | renders successfully if the template handles mixed values |
| empty nested arrays at any depth | renders successfully |
| top-level JSON/YAML scalar or sequence var-file | remains rejected as a non-object var-file |
| YAML map with a non-string key | remains rejected with `ERR_VAL_OBJECT_SHAPE` |
| malformed JSON/YAML | remains rejected with the existing parse/config code |

Where practical, the recursive-shape agent should use a bounded property-based
generator for finite JSON values rather than only a fixed corpus. The bound
should prevent pathological test runtime while still exploring at least four
nesting levels and mixed array contents. A useful invariant is:

> For every generated finite JSON-compatible tree that Minijinja can render,
> the shared validator accepts it, JSON and YAML equivalent inputs agree, and
> rendering either succeeds or reports a template-specific error unrelated to
> nested-array validation.

quality-mgr should report each failure with the smallest reproducer, ingress
format, command, expected/actual result, diagnostic code, and whether the
failure is a production bug, an intentional boundary, or a test oracle issue.
The implementation is not ready for a PASS if any realistic Minijinja-
renderable recursive shape is rejected by `ERR_VAL_NESTED_ARRAY_UNSUPPORTED`,
or if broadening validation causes an existing negative boundary test to pass.

## Risk and compatibility notes

- This is additive for valid existing inputs. Existing scalar values, scalar
  arrays, objects, and top-level arrays of objects retain their behavior.
- Formerly rejected nested arrays become accepted, so callers that depended
  on `ERR_VAL_NESTED_ARRAY_UNSUPPORTED` as a rejection signal will observe a
  behavior change. That is the intended feature behavior.
- Keeping the diagnostic enum variant avoids a public API break, but the
  variant should no longer describe the active supported-input contract.
- JSON/YAML parsing and the top-level var-file object requirement remain
  unchanged. YAML non-string keys remain invalid.
- Minijinja already receives serializable recursive values; no dependency,
  renderer, resolver, or crate-boundary change is expected.
- Recursive validation is linear in the number of value nodes and adds no
  second copy of the input tree. Very deeply adversarial inputs still inherit
  the recursion behavior of the existing YAML/JSON conversion and template
  engine; this feature should not invent a lower nesting limit without a
  separate resource-limit requirement.
- The change affects the documented input contract, so requirements and
  architecture text must be updated in the same implementation PR. README
  and CHANGELOG changes depend on whether they currently describe the old
  restriction and the project's release-note policy.

## Implementation sequence

1. Update the shared validator and its public/docs comments, retaining the
   legacy diagnostic code.
2. Add unit tests for recursive JSON values and YAML conversion.
3. Add CLI regression tests for both issue repros, JSON/YAML ingress, and
   default-input sources.
4. Reuse the four committed issue fixtures as regression coverage and update
   their current failure descriptions to successful examples once the
   production change lands.
5. Update requirements, architecture, project-plan, and diagnostic registry
   references.
6. Run formatting, workspace tests, clippy, and boundary checks.
7. Review the diff specifically for accidental removal of top-level-object or
   YAML string-key validation.
