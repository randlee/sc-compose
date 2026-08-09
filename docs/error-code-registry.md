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

### sc-lint CLI Integration Classes

The `CLI.*` classes are stable diagnostics from the sc-lint subprocess
boundary. They are distinct from the `ERR_*` codes emitted by the
sc-composer library and are normalized by the shared lint runner.

| Code | Error family | Severity | Trigger condition | Expected primary emitter |
| --- | --- | --- | --- | --- |
| `CLI.CONFIG_ERROR` | `ScLintOutcome::ConfigError` | error | repository configuration or a required sc-lint utility is missing or malformed | sc-lint integration runner |
| `CLI.CAPABILITY_ERROR` | `ScLintOutcome::CapabilityError` | error | repository configuration is valid but the host lacks a required lint capability | sc-lint integration runner |
| `CLI.BACKEND_EXEC_FAILURE` | `ScLintOutcome::Failed` | error | an allowlisted sc-lint workflow step exits unsuccessfully | sc-lint integration runner |
| `CLI.BACKEND_PROTOCOL_ERROR` | `ScLintOutcome::Failed` or `ConfigError` | error | sc-lint returns malformed adapter JSON; missing view utilities are normalized to `CLI.CONFIG_ERROR` | sc-lint integration runner |

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
| `WARN_LINT_REDUNDANT_FILTER_CHAIN` | `TemplateLint` | warning | template applies the redundant `frontmatter_safe | yaml_safe` filter chain | CLI `validate --lint` |
| `WARN_CONFIG_SINGLE_PASS_ALL_FALLBACK` | `ConfigError` | warning | `--all` was requested for a template without stacked headers; the single pass is used as a documented fallback | CLI input/configuration layer |
| `ERR_VAL_EMPTY` | `ValidationError` | error | template body is empty where composition requires content | validation pipeline |
| `ERR_VAL_MISSING_FRONTMATTER` | `ValidationError` | warning | a root or included template file references variables but has no frontmatter block | validation pipeline |
| `ERR_VAL_MISSING_REQUIRED` | `ValidationError` | error | required variable remains unresolved after merge | validation pipeline |
| `ERR_VAL_MISSING_NESTED_FIELD` | `ValidationError` | error | a required nested field path is missing inside a present object value | validation pipeline |
| `ERR_VAL_SHAPE_MISMATCH` | `ValidationError` | error | nested required-path traversal expected an object but found a scalar or array | validation pipeline |
| `ERR_VAL_UNDECLARED_TOKEN` | `ValidationError` | warning/error | referenced token is not declared in frontmatter | validation pipeline |
| `ERR_VAL_EXTRA_INPUT` | `ValidationError` | warning/error | caller provided a variable that is neither declared nor referenced | validation pipeline |
| `ERR_VAL_UNBOUND_VARIABLE` | `ValidationError` | warning/error | referenced variable has no value binding after context/default merge | validation pipeline |
| `INFO_VAL_DEFAULT_USED` | `ValidationError` | info | variable was not provided explicitly and a default value was used | validation pipeline, CLI `validate`, CLI `render --dry-run` |
| `ERR_RENDER_STDIN_DOUBLE_READ` | `RenderError` | error | CLI attempts to consume stdin twice for guidance/prompt inputs | CLI input layer |
| `ERR_RENDER_WRITE` | `RenderError` | error | output write or output-target materialization failure | CLI output layer |
| `ERR_CONFIG_READONLY` | `ConfigError` | error | frontmatter rewrite or workspace update refused on read-only target | `frontmatter_init()`, `init_workspace()` |
| `ERR_CONFIG_MODE` | `ConfigError` | error | command or helper invoked in an incompatible mode | CLI argument validation, `resolve_profile()` |
| `ERR_CONFIG_READ` | `ConfigError` | error | a required text/config file exists but cannot be read as valid text | include engine, `verify()`, workspace helpers |
| `ERR_CONFIG_PARSE` | `ConfigError` | error | malformed or unreadable configuration input | var-file/config parsing |
| `ERR_CONFIG_VARFILE` | `ConfigError` | error | invalid var-file shape or unsupported structure, including YAML merge keys (`<<`) rejected with a source line/column and explicit-mapping recovery guidance | var-file parsing |
| `ERR_CONFIG_PACK_NOT_FOUND` | `ConfigError` | error | named example or template pack does not exist under the selected pack root | CLI `examples`, CLI `templates` |
| `ERR_CONFIG_PACK_NOT_RENDERABLE` | `ConfigError` | error | named pack cannot be rendered because it is ambiguous or lacks exactly one renderable root template | CLI `examples`, CLI `templates` |
| `ERR_CONFIG_TEMPLATE_EXISTS` | `ConfigError` | error | `templates add` target pack already exists | CLI `templates add` |
| `ERR_EXTRACT_INVALID_REQUEST` | `ExtractError` | error | in-memory extraction request violates source, filter, or report invariants | `sc_composer::extract()` and report construction |
| `ERR_EXTRACT_MALFORMED` | `ExtractError` | error | rendered XML cannot be parsed as well-formed input | XML extraction engine |
| `ERR_EXTRACT_UNSUPPORTED` | `ExtractError` | error | known template uses syntax outside the supported reversible XML subset | XML extraction engine |
| `ERR_EXTRACT_AMBIGUOUS` | `ExtractError` | error | multiple structural interpretations remain for an extraction result | XML extraction engine and report construction; Phase I/I.2 raw-text mode |
| `ERR_EXTRACT_XML_ELEMENT_MISMATCH` | `ExtractError` | error | rendered XML element name differs from the corresponding known-template element name | XML structural matching |
| `ERR_EXTRACT_XML_ATTRIBUTE_MISMATCH` | `ExtractError` | error | rendered XML attribute count or attribute-name set differs from the corresponding known-template element | XML structural matching |
| `ERR_EXTRACT_XML_CHILD_STRUCTURE_MISMATCH` | `ExtractError` | error | rendered XML child markup does not match the known template's approved child structure | Phase I.3 XML block/mixed-content extraction |
| `ERR_EXTRACT_XML_CONTROL_FLOW_UNSUPPORTED` | `ExtractError` | error | known XML template requires unsupported control-flow reconstruction in a block/mixed-content placeholder | Phase I.3 XML block/mixed-content extraction |
| `ERR_EXTRACT_XML_DYNAMIC_ELEMENT_NAME` | `ExtractError` | error | known XML template uses a dynamic element name outside the supported fixed-name structure | Phase I.3 XML block/mixed-content extraction |
| `ERR_EXTRACT_XML_STATIC_MISMATCH` | `ExtractError` | error | rendered XML static content does not match the known template's static content during value matching | XML extraction engine |
| `ERR_EXTRACT_XML_NAMESPACE_UNSUPPORTED` | `ExtractError` | error | rendered or known XML uses qualified names or namespace declarations outside the unambiguous extraction subset | XML extraction rejection |
| `WARN_EXTRACT_NOT_OBSERVED` | `ExtractionReport` | warning | a declared scalar occurrence is absent from the rendered XML | XML extraction engine |
| `WARN_EXTRACT_LOW_CONFIDENCE` | `ExtractionReport` | warning | structural or static evidence is insufficient for a high-confidence report | XML extraction engine; Phase I/I.2 raw-text mode |
| `WARN_EXTRACT_DIRTY_PREFIX_STRIPPED` | `ExtractionReport` | warning | rendered XML had an accepted leading text preamble removed before parsing | Phase I.4 XML dirty-prefix normalizer |

### Accepted Phase-H Cross-Format Extraction Codes

These codes are accepted by H.1 and are required implementation targets for
the owning sprint. They are deliberately distinct from the existing XML/general
codes above so format-specific parser and policy failures remain stable.

| Code | Error family | Severity | Trigger condition | Expected primary emitter |
| --- | --- | --- | --- | --- |
| `ERR_EXTRACT_FORMAT_UNSUPPORTED` | `ExtractError` | error | requested format is not enabled by the public format selector | H.3 adapter surfaces |
| `ERR_EXTRACT_TEMPLATE_UNSUPPORTED` | `ExtractError` | error | unsupported loop, branch, dynamic key, typed placeholder, or other cross-format template syntax | H.2/H.4/H.5 adapters; Phase I/I.2 raw-text mode |
| `ERR_EXTRACT_INPUT_LIMIT` | `ExtractError` | error | input size, depth, or occurrence limit is exceeded | H.2 JSON, H.4 YAML, H.5 TOML, and H.7 JSON/YAML/XML hardening adapters |
| `ERR_EXTRACT_JSON_MALFORMED` | `ExtractError` | error | rendered input is not one well-formed JSON value | H.2 JSON adapter |
| `ERR_EXTRACT_JSON_DUPLICATE_KEY` | `ExtractError` | error | a JSON object repeats a key | H.2 JSON adapter |
| `ERR_EXTRACT_JSON_PATH_MISSING` | `ExtractError` | error | a known-template JSON path is absent | H.2 JSON adapter |
| `ERR_EXTRACT_JSON_SHAPE_MISMATCH` | `ExtractError` | error | JSON object/array or static value differs from the known template | H.2 JSON adapter |
| `ERR_EXTRACT_JSON_VALUE_UNSUPPORTED` | `ExtractError` | error | placeholder occurs in a key, non-string value, or structural position | H.2 JSON adapter |
| `ERR_EXTRACT_JSON_AMBIGUOUS` | `ExtractError` | error | one variable occurs at multiple distinct JSON paths | H.2 JSON adapter/report |
| `ERR_EXTRACT_YAML_MALFORMED` | `ExtractError` | error | rendered input is not one well-formed YAML document | H.4 YAML adapter |
| `ERR_EXTRACT_YAML_DUPLICATE_KEY` | `ExtractError` | error | a YAML mapping repeats a key | H.4 YAML adapter |
| `ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED` | `ExtractError` | error | YAML alias or anchor is present | H.4 YAML adapter |
| `ERR_EXTRACT_YAML_DOCUMENT_STREAM` | `ExtractError` | error | more than one YAML document is present | H.4 YAML adapter |
| `ERR_EXTRACT_YAML_PATH_MISSING` | `ExtractError` | error | a known-template YAML path is absent | H.4 YAML adapter |
| `ERR_EXTRACT_YAML_SHAPE_MISMATCH` | `ExtractError` | error | YAML mapping/sequence or static scalar differs from the known template | H.4 YAML adapter |
| `ERR_EXTRACT_YAML_VALUE_UNSUPPORTED` | `ExtractError` | error | placeholder occurs in a key, typed scalar, null, tag, alias, or structure | H.4 YAML adapter |
| `ERR_EXTRACT_YAML_AMBIGUOUS` | `ExtractError` | error | one variable occurs at multiple distinct YAML paths | H.4 YAML adapter/report |
| `ERR_EXTRACT_TOML_MALFORMED` | `ExtractError` | error | rendered input is not one well-formed TOML document | H.5 TOML adapter |
| `ERR_EXTRACT_TOML_DUPLICATE_KEY` | `ExtractError` | error | a TOML table or document repeats a key | H.5 TOML adapter |
| `ERR_EXTRACT_TOML_PATH_MISSING` | `ExtractError` | error | a known-template TOML path is absent | H.5 TOML adapter |
| `ERR_EXTRACT_TOML_SHAPE_MISMATCH` | `ExtractError` | error | TOML table/array or static value differs from the known template | H.5 TOML adapter |
| `ERR_EXTRACT_TOML_VALUE_UNSUPPORTED` | `ExtractError` | error | placeholder occurs in a key, non-string value, null-equivalent, or structure | H.5 TOML adapter |
| `ERR_EXTRACT_TOML_AMBIGUOUS` | `ExtractError` | error | one variable occurs at multiple distinct TOML paths | H.5 TOML adapter/report |

For every Phase-H code, the serialized diagnostic uses the existing
`diagnostics[]` envelope with stable `code`, `severity`, `message`, and
optional `location` fields; the owning sprint must add the documented recovery
hint to the diagnostic detail before emitting the code. The full trigger,
recovery, path/source, and scope policy is in
`docs/phase-H/sprint-h-1-reverse-extraction-extension-contract.md`.

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
