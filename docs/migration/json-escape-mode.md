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
