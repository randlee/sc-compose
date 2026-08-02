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
| `ERR_INCLUDE_NOT_FOUND` | `IncludeError` | error | include target cannot be resolved within the allowed roots | include engine |
| `ERR_INCLUDE_ESCAPE` | `IncludeError` | error | include path escapes confinement root | include engine |
| `ERR_INCLUDE_CYCLE` | `IncludeError` | error | include graph revisits a file already on the active include stack | include engine |
| `ERR_INCLUDE_DEPTH` | `IncludeError` | error | include depth exceeds configured maximum | include engine |
| `ERR_VAL_OBJECT_SHAPE` | `ValidationError` | error | structured input object uses an unsupported shape such as a non-string key | structured input parsing, validation pipeline |
| `ERR_VAL_NESTED_ARRAY_UNSUPPORTED` | `ValidationError` | reserved | legacy H2 nested-array restriction; retained for compatibility and not emitted for recursive JSON/YAML-compatible values | compatibility only |
| `ERR_VAL_DUPLICATE` | `ValidationError` | error | duplicate frontmatter variable declaration | frontmatter normalization, validation pipeline |
| `WARN_VAL_CONFLICTING_DEFAULT_SECTIONS` | `ValidationError` | warning | frontmatter declared both `defaults` and `input_defaults`; `input_defaults` overrides overlaps | frontmatter normalization, validation pipeline |
| `ERR_VAL_EMPTY` | `ValidationError` | error | template body is empty where composition requires content | validation pipeline |
| `ERR_VAL_MISSING_FRONTMATTER` | `ValidationError` | warning | a root or included template file references variables but has no frontmatter block | validation pipeline |
| `ERR_VAL_MISSING_REQUIRED` | `ValidationError` | error | required variable remains unresolved after merge | validation pipeline |
| `ERR_VAL_MISSING_NESTED_FIELD` | `ValidationError` | error | a required nested field path is missing inside a present object value | validation pipeline |
| `ERR_VAL_SHAPE_MISMATCH` | `ValidationError` | error | nested required-path traversal expected an object but found a scalar or array | validation pipeline |
| `ERR_VAL_UNDECLARED_TOKEN` | `ValidationError` | warning/error | referenced token is not declared in frontmatter | validation pipeline |
| `ERR_VAL_EXTRA_INPUT` | `ValidationError` | warning/error | caller provided a variable that is neither declared nor referenced | validation pipeline |
| `INFO_VAL_DEFAULT_USED` | `ValidationError` | info | variable was not provided explicitly and a default value was used | validation pipeline, CLI `validate`, CLI `render --dry-run` |
| `ERR_RENDER_STDIN_DOUBLE_READ` | `RenderError` | error | CLI attempts to consume stdin twice for guidance/prompt inputs | CLI input layer |
| `ERR_RENDER_WRITE` | `RenderError` | error | output write or output-target materialization failure | CLI output layer |
| `ERR_CONFIG_READONLY` | `ConfigError` | error | frontmatter rewrite or workspace update refused on read-only target | `frontmatter_init()`, `init_workspace()` |
| `ERR_CONFIG_MODE` | `ConfigError` | error | command or helper invoked in an incompatible mode | CLI argument validation, `resolve_profile()` |
| `ERR_CONFIG_READ` | `ConfigError` | error | a required text/config file exists but cannot be read as valid text | include engine, `verify()`, workspace helpers |
| `ERR_CONFIG_PARSE` | `ConfigError` | error | malformed or unreadable configuration input | var-file/config parsing |
| `ERR_CONFIG_VARFILE` | `ConfigError` | error | invalid var-file shape or unsupported structure | var-file parsing |
| `ERR_CONFIG_PACK_NOT_FOUND` | `ConfigError` | error | named example or template pack does not exist under the selected pack root | CLI `examples`, CLI `templates` |
| `ERR_CONFIG_PACK_NOT_RENDERABLE` | `ConfigError` | error | named pack cannot be rendered because it is ambiguous or lacks exactly one renderable root template | CLI `examples`, CLI `templates` |
| `ERR_CONFIG_TEMPLATE_EXISTS` | `ConfigError` | error | `templates add` target pack already exists | CLI `templates add` |
| `ERR_EXTRACT_INVALID_REQUEST` | `ExtractError` | error | in-memory extraction request violates source, filter, or report invariants | `sc_composer::extract()` and report construction |
| `ERR_EXTRACT_MALFORMED` | `ExtractError` | error | rendered XML cannot be parsed as well-formed input | XML extraction engine |
| `ERR_EXTRACT_UNSUPPORTED` | `ExtractError` | error | known template uses syntax outside the supported reversible XML subset | XML extraction engine |
| `ERR_EXTRACT_AMBIGUOUS` | `ExtractError` | error | multiple structural interpretations remain for an extraction result | XML extraction engine and report construction |
| `WARN_EXTRACT_NOT_OBSERVED` | `ExtractionReport` | warning | a declared scalar occurrence is absent from the rendered XML | XML extraction engine |
| `WARN_EXTRACT_LOW_CONFIDENCE` | `ExtractionReport` | warning | structural or static evidence is insufficient for a high-confidence report | XML extraction engine |

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
  validation work across Sprint 3 and Sprint 4.
- `ERR_VAL_OBJECT_SHAPE`, `ERR_VAL_SHAPE_MISMATCH`, and
  `ERR_VAL_MISSING_NESTED_FIELD` are owned by Phase HTML-Report / Sprint H1.
- `ERR_VAL_NESTED_ARRAY_UNSUPPORTED` is retained as a reserved compatibility
  code after E.1; recursive JSON/YAML-compatible values must not emit it.
- `RenderError` CLI-facing codes are owned by the Sprint 4 release-gate
  command/output verification work.
- `ConfigError` codes are shared between Sprint 2 type/error work and Sprint 4
  workspace-helper and release-gate verification work.

## Change Control

- Additions require a planning/doc update before implementation.
- Renames are forbidden once a code is used in snapshots or released CLI JSON.
- Deprecation must leave the old code documented until a full compatibility
  review removes it.
