# JSON interpolation migration

`sc-compose` 1.4.1 uses `auto` JSON interpolation by default. Use bare
placeholders when a value occupies a JSON value position:

```json
{
  "worktree_path": {{ worktree_path }},
  "retries": {{ retries }},
  "enabled": {{ enabled }}
}
```

The renderer owns JSON quoting and preserves object, array, boolean, number,
and null values. Do not add quotes around a placeholder in this mode.

Existing templates may temporarily opt into the compatibility mode:

```yaml
json_escape_mode: legacy
```

Legacy mode safely escapes the contents of manually quoted string
placeholders, but it is deprecated. `validate` and `validate --lint` emit:

```text
Template uses legacy JSON escape mode. Migrate to bare placeholders (auto mode) to avoid double-quoting issues. See docs/migration/json-escape-mode.md
```

The same mode can be selected for a command with
`--json-escape-mode legacy|auto`; the CLI override takes precedence over root
frontmatter, and the default is `auto`. Migrate each quoted placeholder by
removing its source quotes and selecting `auto` explicitly if the template is
shared across versions. Keep the value's intended JSON type in the fixture
used to validate the template.

The `legacy` mode must not be used as raw interpolation. Hostile strings such
as `x\", \"injected\": true` remain one JSON string and cannot create a second
object key.

## O.4 six-template migration matrix

The six in-repository assignment templates are now explicitly `auto` mode.
Every dynamic string slot is bare so the renderer owns JSON quoting. Array
loops use the same bare-value rule for each scalar element. The one reviewed
raw-JSON exception is `carry_forward_findings_json`; it is supplied as a
validated JSON fragment and is marked `safe` deliberately so it remains an
array rather than becoming a JSON string containing serialized JSON.

| Template | Scalar slots | Scalar array elements | Structured/raw contract | Ambiguous constructs |
| --- | --- | --- | --- | --- |
| `rust-best-practices-assignment` | `review_mode`, `worktree_path`, `practice_mode`, `notes` | `review_targets`, `practice_ids`, `changed_files`, `triage_records` | `carry_forward_findings_json` is reviewed raw JSON | none |
| `rust-qa-assignment` | `worktree_path`, `review_mode`, `baseline_ref`, `artifact_commands` | `review_targets`, `changed_files`, `triage_records` | `carry_forward_findings_json` is reviewed raw JSON | none |
| `rust-service-hardening-assignment` | `review_mode`, `worktree_path` | `review_targets`, `topics`, `service_indicators_extra`, `changed_files`, `triage_records` | `carry_forward_findings_json` is reviewed raw JSON | none |
| `arch-qa-assignment` | `review_type`, `worktree_path`, `branch`, `commit`, conditional `phase`, `sprint`, `sprint_doc` | `review_targets`, `reference_docs`, `changed_files`, `triage_records` | `carry_forward_findings_json` is reviewed raw JSON | none |
| `flaky-test-qa-assignment` | `worktree_path`, conditional `phase`, `sprint` | `review_targets`, `changed_files`, `triage_records` | `carry_forward_findings_json` is reviewed raw JSON | none |
| `req-qa-assignment` | conditional `phase`, `sprint`, `sprint_doc`, `worktree_path`, `branch`, `commit` | `reference_docs`, `review_targets`, `changed_files`, `triage_records` | `carry_forward_findings_json` is reviewed raw JSON | none |

The boolean fields (`round_limit`, check switches, and optional-value branch
conditions) are rendered as literal JSON booleans or `null`; they are not
quoted interpolation slots. O.4 fixtures exercise quotes, backslashes,
Unicode, newlines, empty/optional values, arrays, objects, null branches, and
control-safe strings for the six templates. The legacy fixture intentionally
keeps the old quoted source shape, renders valid JSON, and asserts exactly one
`WARN_JSON_LEGACY_ESCAPE_MODE` diagnostic.
