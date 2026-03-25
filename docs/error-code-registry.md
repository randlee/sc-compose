# SC-Compose Error Code Registry

This registry is the canonical planning reference for stable `ERR_*` codes used
by `sc-composer` and `sc-compose`.

## Registry Rules

- Codes are stable identifiers and must not be repurposed.
- A code maps to one primary canonical error family.
- Human-readable CLI output may vary; the code must not.
- New codes require updates to:
  - `docs/architecture.md`
  - `docs/project-plan.md` acceptance criteria where relevant
  - automated tests and snapshots

## Canonical Codes

| Code | Error family | Severity | Trigger condition | Expected primary emitter |
| --- | --- | --- | --- | --- |
| `ERR_RESOLVE_NOT_FOUND` | `ResolveError` | error | no matching template/profile found | `resolve_profile()`, CLI `resolve`, CLI `render` |
| `ERR_RESOLVE_AMBIGUOUS` | `ResolveError` | error | multiple candidates found without a disambiguating runtime | `resolve_profile()`, CLI `resolve`, CLI `render` |
| `ERR_INCLUDE_ESCAPE` | `IncludeError` | error | include path escapes confinement root | include engine |
| `ERR_INCLUDE_DEPTH` | `IncludeError` | error | include depth exceeds configured maximum | include engine |
| `ERR_VAL_TYPE` | `ValidationError` | error | invalid scalar type or variable type mismatch | validation pipeline |
| `ERR_VAL_DUPLICATE` | `ValidationError` | error | duplicate frontmatter variable declaration | frontmatter normalization, validation pipeline |
| `ERR_VAL_EMPTY` | `ValidationError` | error | template body is empty where composition requires content | validation pipeline |
| `ERR_VAL_MISSING_REQUIRED` | `ValidationError` | error | required variable remains unresolved after merge | validation pipeline |
| `ERR_VAL_UNDECLARED_TOKEN` | `ValidationError` | warning/error | referenced token is not declared in frontmatter | validation pipeline |
| `ERR_VAL_EXTRA_INPUT` | `ValidationError` | warning/error | caller provided a variable that is neither declared nor referenced | validation pipeline |
| `ERR_RENDER_STDIN_DOUBLE_READ` | `RenderError` | error | CLI attempts to consume stdin twice for guidance/prompt inputs | CLI input layer |
| `ERR_RENDER_WRITE` | `RenderError` | error | output write or output-target materialization failure | CLI output layer |
| `ERR_CONFIG_READONLY` | `ConfigError` | error | frontmatter rewrite or workspace update refused on read-only target | `frontmatter_init()`, `init_workspace()` |
| `ERR_CONFIG_PARSE` | `ConfigError` | error | malformed or unreadable configuration input | var-file/config parsing |
| `ERR_CONFIG_VARFILE` | `ConfigError` | error | invalid var-file shape or unsupported structure | var-file parsing |

## Planned Diagnostic Shape

Every diagnostic record emitted under FR-8 should be compatible with this
minimum logical structure:

```json
{
  "severity": "error",
  "code": "ERR_VAL_MISSING_REQUIRED",
  "message": "missing required variable: name",
  "location": "templates/example.md.j2:12:4"
}
```

## Ownership Notes for Agents

- `ResolveError` codes are owned by resolver work in Sprint 3.
- `IncludeError` and most `ValidationError` codes are owned by include and
  validation work across S3-S4.
- `RenderError` CLI-facing codes are owned by Sprint 5 command/output work.
- `ConfigError` codes are shared between Sprint 2 type/error work and Sprint 5
  workspace-helper work.

## Change Control

- Additions require a planning/doc update before implementation.
- Renames are forbidden once a code is used in snapshots or released CLI JSON.
- Deprecation must leave the old code documented until a full compatibility
  review removes it.
