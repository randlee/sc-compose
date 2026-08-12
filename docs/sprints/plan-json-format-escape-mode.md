---
id: PLAN-JSON-FORMAT-ESCAPE-MODE
title: "JSON escape-mode compatibility and fail-closed rendered-output validation"
status: proposed
branch: plan/json-format-escape-mode
base: develop
issue: "FIX-JSON-FORMAT-DOUBLE-QUOTE"
target_release: 1.4.1
---

# PLAN-JSON-FORMAT-ESCAPE-MODE

## 1. Purpose

This plan addresses two coupled defects exposed by the 1.4.0 JSON-rendering
regression:

1. `AutoEscape::Json` correctly prevents JSON injection for a bare Jinja
   placeholder, but it is incompatible with the pre-1.4 template idiom that
   puts literal JSON quotes around the placeholder.
2. `render`, `validate`, and `validate --lint` can currently accept or emit
   output that is malformed JSON. A successful process exit is therefore not
   sufficient evidence that a JSON template produced a payload that a
   downstream consumer can parse.

The implementation must preserve the security property introduced by FIX-272,
give existing repositories an actionable compatibility path, and give
ATM-core a machine-readable answer to the narrower question it actually needs
to ask: “did this template render valid output for this exact context?”

This is a planning document. It intentionally contains no implementation,
source-code edits, generated reports, or fabricated closeout evidence.

## 2. Executive decision

Use an explicit JSON interpolation mode as the compatibility contract, with a
fail-closed rendered-output check as the safety net.

The near-term release behavior is:

| Mode | Intended source shape | String behavior | Status |
| --- | --- | --- | --- |
| `legacy` | `"{{ value }}"` | escape JSON string contents without adding another pair of quotes | compatibility mode; deprecation warning |
| `auto` | `{{ value }}` | emit a complete JSON value; strings receive quotes, structured values retain JSON structure | secure recommended mode |

The effective mode is selected by this precedence order:

1. an explicit CLI override, when the caller supplies one;
2. a `json_escape_mode` frontmatter declaration;
3. the 1.4.1 compatibility default, `legacy`, for an otherwise unannotated
   existing JSON template.

The compatibility default is deliberately temporary and emits a warning. It
prevents the first run after upgrade from silently corrupting existing
repositories while giving maintainers a concrete migration path. New
`template-init` JSON templates must declare `json_escape_mode: auto` and use
bare placeholders. A later major release may change the absent-mode default to
`auto` after the deprecation window.

The renderer must never implement `legacy` as raw, unescaped interpolation.
`legacy` means “quoted-string content escaping”: quotes, backslashes, control
characters, and the other JSON string escapes are encoded, but the renderer
does not add the outer quotes that are already present in the source template.
This preserves the old source shape without reopening the injection defect that
FIX-272 was intended to close.

The plan does not make heuristic source auto-detection the primary behavior.
An optional detector may be added later, but correctness must not depend on
guessing whether a quote belongs to a JSON string or to a surrounding Jinja
construct. Explicit mode plus a useful diagnostic is safer and easier to
support across conditionals, loops, includes, raw JSON fields, and non-string
values.

## 3. User-visible outcomes

### 3.1 Existing manually quoted templates

An existing template such as:

```json
{
  "worktree_path": "{{ worktree_path }}"
}
```

continues to produce valid JSON under the compatibility default. A warning
identifies the template, the effective mode, the deprecation, and the exact
migration:

```text
WARN_JSON_LEGACY_ESCAPE_MODE: template `.../assignment.json.j2` uses the
legacy JSON interpolation mode. String placeholders inside literal JSON quotes
are supported for compatibility but will be removed as the default in a future
release. Prefer `json_escape_mode: auto` and change
`"{{ worktree_path }}"` to `{{ worktree_path }}`. Use
`sc-compose validate --lint --check-render ...` to verify the migrated output.
```

The warning must include a stable diagnostic code, source path, and location
when the source location is known. It must be visible in human output and in
the versioned JSON diagnostic envelope.

### 3.2 New safe templates

An `auto` template uses:

```yaml
---
format: json
json_escape_mode: auto
required_variables:
  - worktree_path
defaults: {}
metadata: {}
---
{"worktree_path": {{ worktree_path }}}
```

For a string value, the renderer owns the JSON string delimiters. For a number,
boolean, null, object, or array, the renderer emits the corresponding JSON
value. The output is parsed before it is reported as a successful checked
render.

### 3.3 A malformed or mismatched template

If a template is rendered in `auto` mode while retaining the old literal-quote
idiom, the command must fail before writing the output or claiming success.
The diagnostic should be actionable, for example:

```text
ERR_RENDER_OUTPUT_INVALID_JSON: rendered output from
`templates/assignment.json.j2` is not valid JSON at line 1, column 21:
expected ',' or '}' after a JSON value. This often means a string placeholder
is manually quoted while JSON auto escaping is enabled. Either remove the
literal quotes and keep `json_escape_mode: auto`, or select the temporary
compatibility mode with `--json-escape-mode legacy` /
`json_escape_mode: legacy` and rerun validation.
```

The error must include the parser offset/line/column when available, but must
not dump secrets or the full rendered payload into stderr. `render --json`
must put the diagnostic in the normal `DiagnosticEnvelope`; it must not emit a
plain parser error outside the envelope.

### 3.4 ATM-core integration result

ATM-core must not infer renderability from a zero exit code alone. It must call
the checked validation/render contract with the exact variable context it is
about to use, inspect the structured result, and proceed only when:

```text
template_contract_valid == true
render_valid_for_context == true
output_format == expected format
diagnostics contain no error severity
```

The result must distinguish a static contract result from a context-specific
render result. No implementation can prove that arbitrary future values will
make every dynamic branch valid unless the template uses only the safe,
structured interpolation contract. The API must therefore say what was
actually checked rather than promise an unqualified “always renderable.”

## 4. Evidence and root cause

### 4.1 Current renderer behavior

The current implementation is in
`crates/sc-composer/src/renderer.rs`:

- `legacy_auto_escape_callback` strips template suffixes and selects
  `AutoEscape::Json` for a `.json` stem at lines 54–61.
- `format_sc_compose_markup` delegates every non-HTML custom mode to
  minijinja's `escape_formatter` at lines 167–179.
- The existing renderer tests at approximately lines 532–578 cover a bare
  JSON placeholder, an injection value, and the corrected bare
  `worktree_path` fixture. They do not cover the old source shape with literal
  quotes around the placeholder.

For this reason:

```text
source:  {"worktree_path": "{{ worktree_path }}"}
value:   /abs/path
1.3:     {"worktree_path": "/abs/path"}
1.4:     {"worktree_path": ""/abs/path""}
```

The 1.4 behavior is correct for the new bare-placeholder contract and wrong
for the old quoted-placeholder contract. It is not a reason to remove JSON
escaping altogether: reverting to the 1.3 behavior would restore the
injection demonstrated by FIX-272.

### 4.2 Current validation behavior

`crates/sc-compose/src/commands/compose.rs::run_validate` at lines 110–165:

- builds the request;
- calls `sc_composer::validate_with_observer`;
- optionally calls `lint_request` when `--lint` is present;
- prints `valid` or the diagnostics;
- returns success based on the structural validation report.

It does not render the body and does not parse format-specific output. This is
consistent with the command help and requirements language that `validate`
does not write/render output, but it means `valid` currently means “the
template and variable/include contract passed static validation,” not “the
rendered JSON payload parses.”

`crates/sc-compose/src/commands/template_lint.rs` at lines 7–50 expands the
include graph and scans Jinja expressions. The only current rule is the
`frontmatter_safe | yaml_safe` redundant-chain warning at lines 33–44 and
53–75. There is no JSON output rule.

`crates/sc-compose/src/commands/compose_render.rs` at lines 15–67 composes and
passes the rendered text directly to `emit_render_output`. No format-aware
post-render parse gate sits between composition and emission.

`crates/sc-compose/src/extract/json.rs` is not a suitable existing gate. It
parses already-rendered input for the separate `extract` command and is not on
the normal render or validate path.

### 4.3 Current repository-level lint behavior

`sc-compose lint` is an allowlisted subprocess runner, not an alias for
`validate --lint`:

- `crates/sc-compose/src/commands/sc_lint.rs` lines 17–31 define the target
  registry location and command descriptor shape;
- lines 112–180 execute the selected external `sc-lint` command and materialize
  its raw JSON and HTML reports;
- the root `justfile` lines 8–9 maps `just lint` to
  `sc-compose lint --root . --target <target> --json`;
- `justfile` lines 11–21 contains the repository's consumer profile and
  currently invokes code, boundary, portability, line-count, and dependency
  checks.

The distinction is intentional:

| Path | Scope | Current oracle | External tools | Current JSON-render check |
| --- | --- | --- | --- | --- |
| `validate` | one template plus includes | static parse, declarations, variables, includes | no | no |
| `validate --lint` | same template graph plus source lint | current template lint rules and source locations | no | no |
| `sc-compose lint` | repository quality profile | allowlisted sc-lint target result | yes | no, unless a target adds it |

The commands should not be collapsed into one overloaded operation. They should
share the same library-owned template diagnostics and render-check engine so
the rules cannot drift.

### 4.4 Exact fuzz evidence

The 2026-08-11 sc-compose fuzz session (`site/reports/20260811-3-fuzz-report`)
did find the defect. Its shape-probe worker ran 46 iterations, passed 45, and
reported one confirmed bug. The minimal template was:

```json
{"worktree_path": "{{ worktree_path }}"}
```

The finding recorded that `template-init` preserved the source quotes while
the renderer's JSON auto-escape re-quoted the value, producing invalid JSON
with exit 0 and no diagnostics. The finding was classified as a
`template-init` contract gap, and the recommended fix was to consume the
surrounding quotes during JSON template initialization.

The same report also shows why the release regression was missed:

1. The campaign's main corpus was split across workers and treated missing
   variables/undeclared tokens as expected baseline noise.
2. The durable renderer regression test promoted from the finding uses the
   corrected bare source shape and proves that auto escaping owns the quotes.
3. The campaign verified the `template-init` round-trip case but did not create
   a compatibility test for hand-authored old templates after the renderer
   contract changed.
4. A rendered process exit of 0 was not a sufficient output oracle, because
   the render path did not parse JSON before emitting it.
5. The fuzz campaign therefore found a real representative failure without
   closing the complete product contract: source idioms, renderer mode,
   post-render parser validation, and release-corpus migration were not tested
   together.

This is a test-oracle and contract-coverage failure, not evidence that the
fuzzing approach was useless. The plan must preserve the existing four-worker
campaign while adding a compatibility matrix and a parser-backed output oracle.

## 5. Scope and non-goals

### 5.1 In scope

- explicit JSON escape mode in the template/render contract;
- safe legacy quoted-string content escaping;
- auto mode for bare JSON placeholders and structured values;
- CLI override and frontmatter declaration with defined precedence;
- deprecation diagnostics and migration guidance;
- format-aware post-render JSON validation;
- static quoted-placeholder linting;
- integration through `validate`, `validate --lint`, `render`,
  `sc-compose lint`, and `just lint`;
- machine-readable result for ATM-core and other callers;
- `template-init` generation aligned with the selected mode;
- migration fixtures for the six known repository templates;
- release-corpus checks for every repository enumerated and pinned in
  `docs/phase-O/release-corpus-roots.txt` (the campaign reports the actual
  count rather than assuming 20–30);
- fuzz-oracle and regression-test improvements.

### 5.2 Out of scope

- ATM runtime code or ATM-core implementation in this repository;
- automatic modification of external repositories;
- making arbitrary Jinja programs statically proveable for every possible
  variable value;
- changing HTML, XML, CDATA, Turtle, YAML, or Markdown escaping semantics;
- making `validate` silently render and write files;
- treating a retry after malformed output as a correctness strategy;
- removing `AutoEscape::Json` or restoring unescaped 1.3 interpolation;
- adding an unconstrained “raw JSON” escape bypass. Raw JSON insertion must be
  explicit, typed/documented, and covered separately if retained.

## 6. Contract design

### 6.1 Frontmatter key

Add a parsed, validated key with an exact name:

```yaml
json_escape_mode: legacy | auto
```

The key has meaning only when the effective template output format is JSON.
The plan must resolve the existing filename/frontmatter relationship explicitly:

1. the current filename convention (`*.json.j2`, after stripping the Jinja
   suffix) remains the authoritative output-format selector for 1.4.1;
2. `format: json` is accepted as a declaration and must agree with the
   filename-derived kind when both are present;
3. a mismatch produces a stable configuration diagnostic rather than silently
   choosing one behavior;
4. consuming `format:` for every output type is not part of this fix unless
   implementation discovery shows that the current parser already owns the
   field; the plan must not create a second competing format system.

This avoids changing unrelated XML/HTML behavior while making JSON mode
explicit and observable in the render contract.

### 6.2 CLI option

Add an override named consistently across relevant commands:

```text
--json-escape-mode <legacy|auto>
```

The option is valid only for JSON output. Supplying it for a non-JSON template
must either be a clear usage error or a structured warning; the implementation
must choose one behavior and test it. The recommended behavior is a usage
error, because silently ignoring a caller's safety choice is surprising.

The override applies to:

- `render`;
- `validate --check-render` (and its `--lint` combination);
- `template-init` only when the command is explicitly asked to generate a
  JSON template, otherwise it has no effect.

`sc-compose lint` does not override a template's mode. It reports the effective
mode and can accept a repository-level policy such as `--json-escape-mode`
only if the selected target is explicitly a template contract target. The
default repository lint should not mutate or reinterpret production templates.

### 6.3 Mode semantics

#### `auto`

- A string expression is a complete JSON string literal; authors omit source
  quotes.
- A number, boolean, null, object, or array is emitted as a complete JSON
  value.
- A literal quote pair around an interpolated scalar is a static lint error or
  a render-contract error, not a silent compatibility conversion.
- Injection values must be escaped as data and never create additional object
  keys or array members.
- Raw JSON insertion, if supported by existing templates, must be an explicit
  reviewed mechanism and must not be accidentally treated as a string.

#### `legacy`

- A string expression inside source JSON quotes is escaped as JSON string
  content without adding outer quotes.
- A non-string value in a legacy quoted-string position is rejected with a
  diagnostic explaining that legacy mode is string-only at that position.
- A bare placeholder in legacy mode is rejected or warned with a direct fix to
  select `auto`; it must not be rendered as unescaped text.
- Existing six templates must render valid JSON without source edits when
  legacy mode is effective.
- Every use emits a deprecation warning once per template, not once per
  variable occurrence.

The implementation should prefer a dedicated renderer/filter abstraction over
duplicating escape logic in the CLI. The legacy content-only encoder must be
unit-tested against the same JSON string escaping rules as auto mode.

### 6.4 Precedence and compatibility window

The exact precedence is:

```text
CLI override
  > json_escape_mode in root template frontmatter
  > compatibility default legacy in 1.4.1
```

Included templates must not silently change the root template's mode. If an
included JSON fragment declares a conflicting mode, validation must report the
conflict with both paths and require the caller to resolve it. If an include
has a different output format, it must retain its own format-specific escaping
only where the include mechanism already defines that behavior; the plan must
not introduce cross-document JSON concatenation semantics.

The deprecation warning must state the planned default change only if the
project has committed to a release target. The initial 1.4.1 wording should
say “compatibility mode; migrate to `auto`” rather than promise a date.

## 7. Render-check design

### 7.1 Library-owned result

Add a library-level result type in `sc-composer`, with names finalized during
implementation but equivalent fields to:

```rust
pub struct RenderCheckReport {
    pub template: PathBuf,
    pub output_format: OutputFormat,
    pub json_escape_mode: Option<JsonEscapeMode>,
    pub template_contract_valid: bool,
    pub rendered: bool,
    pub render_valid_for_context: Option<bool>,
    pub checked_context: ContextSummary,
    pub diagnostics: Vec<Diagnostic>,
}
```

The public result must not contain the full rendered body by default. A caller
that already requested a render may retain the body separately. The report
must contain enough information for an external integration to decide whether
to continue, and it must be serializable into the existing diagnostic envelope.

The report should distinguish these cases:

| Result | Meaning | ATM-core action |
| --- | --- | --- |
| contract invalid | template cannot be trusted under declared/effective mode | do not render/send; show diagnostics |
| contract valid, no context supplied | static analysis passed; output not proven | do not claim render guarantee |
| contract valid, context render failed | actual context could not produce valid output | do not cache/send; surface exact error |
| contract valid, context output parsed | exact context produced valid output | safe to consume/cache that result |

### 7.2 JSON parser gate

Create one reusable format-aware post-render checker in `sc-composer` or a
small adjacent module owned by the library:

```text
check_rendered_output(format, template_path, rendered_text)
```

For JSON it must:

1. parse the complete rendered text with the repository's existing JSON parser
   dependency;
2. report line, column, byte offset, and a stable diagnostic code on failure;
3. avoid echoing secret values or the complete payload;
4. return success only after the parser succeeds;
5. be a no-op for formats whose validation contract is not part of this fix,
   while leaving an extension point for XML/YAML later;
6. distinguish a parser failure from a renderer failure and from a static
   variable-validation failure.

The checker must run before any file write or stdout emission when the caller
requests a checked render. For an ordinary `render` command, the 1.4.1 default
is fail-closed for JSON because emitting invalid JSON with exit 0 is the
production incident being fixed. There is no `--check-output` opt-in
transition and no silent acceptance path: malformed JSON is rejected before
emission.

### 7.3 Render API and CLI flag

Introduce the canonical `--check-render` switch rather than overloading
`validate`:

```text
--check-render       perform composition and format-aware output validation
```

`--check-output` is not an alias. The implementation, help, requirements,
examples, and integration tests must use only `--check-render`.

The flag should be available to `render` and `validate`, with the following
semantics:

- `render --check-render`: compose with the supplied context, validate the
  output, then emit only if the check passes;
- `validate --check-render`: perform the normal static validation, compose in
  memory with the supplied context, validate output, emit no output file/body,
  and return a structured report;
- `validate --check-render` without enough context: return a clear “static
  contract checked; context required” result, not a false success;
- `validate --lint --check-render`: combine static source lint and the checked
  render result in one diagnostic envelope;
- `render --json`: validate the rendered body before wrapping it in the CLI
  envelope. The envelope itself is not the template output being checked.

### 7.4 `validate` remains a distinct operation

Do not make plain `validate` render by default. Its current help and
requirements deliberately define it as a no-output structural validation
command. Changing that silently would make it expensive, environment
dependent, and potentially side-effectful.

Instead, update the help text and JSON payload to say:

```text
valid: true
template_contract_valid: true
render_checked: false
render_valid_for_context: null
```

This removes the misleading interpretation of `valid` without making plain
validation perform work the caller did not request.

## 8. Static template lint design

### 8.1 New diagnostic rules

Extend `template_lint.rs` with format-aware rules, using stable codes such as:

- `WARN_JSON_LEGACY_ESCAPE_MODE`: an unannotated or explicitly legacy JSON
  template uses compatibility semantics;
- `WARN_JSON_QUOTED_PLACEHOLDER`: a JSON string contains a manually quoted
  placeholder while the effective mode is `auto`;
- `ERR_JSON_MODE_CONTRACT`: a placeholder shape is incompatible with the
  selected mode;
- `ERR_RENDER_OUTPUT_INVALID_JSON`: checked output failed JSON parsing;
- `WARN_JSON_MODE_MIGRATION`: a legacy declaration should be migrated to
  `auto` after source changes.

Final code names must be registered in the canonical diagnostic schema and
documented in the error/diagnostic registry. Do not use ad hoc string-only
warnings in one command.

### 8.2 Source scanner requirements

The current scanner searches simple `{{ ... }}` spans. Extend it carefully:

1. identify effective JSON templates by the same suffix helper used by the
   renderer;
2. inspect source context around a variable expression, including whitespace
   and immediate quote delimiters;
3. ignore Jinja expressions inside comments and quoted Jinja literals where the
   token is not a rendered variable;
4. do not flag explicit raw JSON fields merely because their names end in
   `_json`;
5. report the source path and expression location;
6. distinguish a scalar string interpolation from a loop/object expression;
7. prefer a conservative “cannot prove” diagnostic over a false migration
   recommendation for complex conditionals or macros.

The linter must not attempt to parse arbitrary rendered output. That belongs to
the checked-render gate. Static lint catches source-shape mistakes before a
context is available; the parser gate catches actual output mistakes after a
context is available.

### 8.3 Warning/error policy

| Context | Legacy quoted placeholder | Auto quoted placeholder | Raw/ambiguous construct |
| --- | --- | --- | --- |
| plain `validate` | compatibility warning if mode known | no render claim | static diagnostic when provable |
| `validate --lint` | warning with migration fix | error-level lint finding | warning/error per contract |
| `validate --check-render` | warning plus checked result | checked render fails with parser/error diagnostic | checked result is authoritative |
| `render` checked/default JSON gate | warning if valid legacy output | fail closed before emission | fail closed |
| `sc-compose lint --target template-contracts` | finding in report | finding in report | finding in report |

The implementation must define whether warning diagnostics affect exit status.
The recommended policy is:

- warnings alone preserve exit 0 for `validate --lint`;
- error-level mode violations and failed checked renders return the existing
  validation/render failure code;
- repository lint target status follows the normal sc-lint finding policy;
- a strict CI profile can promote migration warnings to failures without
  changing interactive defaults.

## 9. `validate`, `validate --lint`, `render`, and `sc-compose lint`

### 9.1 Plain `validate`

Required changes:

- parse and report the effective JSON escape mode;
- report whether only the static contract was checked;
- detect impossible mode declarations and format mismatches;
- do not claim that output is parseable without `--check-render` or the
  fail-closed default render path;
- preserve the no-write/no-body behavior.

Plain `validate` is the cheap preflight used when the caller lacks a complete
context. It answers “can the template be analyzed?” not “will this exact
payload be valid?”

### 9.2 `validate --lint`

Required changes:

- run all existing source lint rules;
- add the JSON quoted-placeholder and mode rules;
- support `--check-render` to add a context-specific parser gate;
- expose all diagnostics through the same human and JSON formats;
- include the effective mode and migration hint in the payload;
- never silently invoke repository-wide `sc-lint` or external tools.

This is the command a template author runs while editing one template or one
include graph.

### 9.3 `render`

Required changes:

- resolve mode before rendering;
- select the legacy content-only encoder or auto complete-value encoder;
- run the JSON parser gate before emission when checked/default fail-closed
  behavior is active;
- include the parser diagnostic in `render --json`'s existing envelope;
- ensure `bytes_written` is zero/not reported as successful when emission is
  prevented;
- retain warning diagnostics for a successful legacy compatibility render.

The render command is the production safety boundary. A caller must not receive
exit 0 and malformed JSON.

### 9.4 `sc-compose lint`

Required changes:

- add a repository-level allowlisted target named consistently, recommended
  `template-contracts`;
- have that target enumerate the repository's template roots and invoke the
  shared Rust template scanner/checker rather than duplicating parsing logic in
  Python or shell;
- produce the normal sc-lint JSON and HTML report shape;
- include mode, source location, diagnostic code, migration recommendation,
  and whether a context-backed checked render was available;
- allow a repository fixture map for representative contexts, but never claim
  all dynamic branches are proven when only fixtures ran;
- include this target in the repository's `full` profile and document whether
  it is part of `just lint` or a separate opt-in during the 1.4.1 rollout.

`sc-compose lint` remains the repo-level orchestrator. It may call the same
library/CLI contract, but it must not reimplement JSON escaping or create a
second diagnostic taxonomy.

### 9.5 `just lint`

The root `justfile` currently delegates `lint target="full"` to
`sc-compose lint`. The plan must:

- add the template contract target to the allowlist and full-profile order;
- keep the command shape consistent with the other repos;
- ensure a missing optional fixture context is a reported capability/config
  result, not a green false positive;
- preserve the repository's existing code/boundary/portability lint targets;
- run the template contract target before final report aggregation so a failed
  render check cannot be hidden by later output.

## 10. ATM-core integration prescription

### 10.1 Adapter responsibility

ATM-core remains outside this repository's dependency boundary. Its adapter
must:

1. identify the template path and expected output format;
2. assemble the exact JSON/YAML context that will be used for the production
   render;
3. invoke `validate --json --check-render` or the equivalent library API;
4. inspect the structured `RenderCheckReport`, not human stdout and not only
   the process exit code;
5. reject the assignment/cache operation on any error diagnostic;
6. render/send only after the checked result succeeds;
7. persist the exact rendered output or its hash only after the successful
   check, according to ATM-core's own storage contract.

### 10.2 Required machine-readable fields

The CLI payload should include at least:

```json
{
  "template": "path/to/assignment.json.j2",
  "output_format": "json",
  "json_escape_mode": "auto",
  "template_contract_valid": true,
  "render_checked": true,
  "render_valid_for_context": true,
  "context_fingerprint": "optional caller-owned identifier",
  "diagnostics": []
}
```

The command envelope remains the outer transport. The template body is not
embedded in the preflight report unless the caller explicitly asks for it.
This keeps reports safe for logs and prevents accidental disclosure of prompt
contents.

### 10.3 Guarantee language

Documentation and API names must use these precise terms:

- “static contract valid” means source analysis passed;
- “render valid for context” means one exact context rendered and parsed;
- “safe for arbitrary string values” may be claimed only for auto mode where
  every interpolation is represented as a complete JSON value and no raw JSON
  bypass is used;
- “all conditional branches covered” requires a fixture matrix or an explicit
  static branch-analysis result, not one successful render.

This wording prevents ATM-core from treating a lightweight validate call as a
proof that a later, different context will parse.

## 11. Migration of the six known templates

The following six current files are the release-corpus migration set:

1. `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`
2. `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2`
3. `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2`
4. `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2`
5. `.claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2`
6. `.claude/skills/codex-orchestration/req-qa-assignment.json.j2`

Migration is not “remove every quote around every Jinja expression.” Each file
must be classified by expression shape first:

| Shape | Migration |
| --- | --- |
| string scalar in a JSON string property | remove source quotes; select `auto` |
| string scalar inside a string array element | remove source quotes; select `auto` |
| number/bool/null/object/array expression | keep bare; select `auto` |
| raw JSON fragment intentionally inserted | document/replace with an explicit structured-value path; do not apply string escaping blindly |
| conditional producing a complete JSON fragment | test every branch; keep the expression bare only if each branch is a valid JSON value |
| macro/include whose output shape is not statically known | retain legacy temporarily or refactor with a fixture-backed contract before selecting `auto` |

For each file, the migration work must:

1. add `json_escape_mode: auto` to the root frontmatter;
2. remove literal quotes only from expressions whose values are JSON string
   values owned by the renderer;
3. leave structural JSON punctuation and intentionally raw structured values
   unchanged;
4. render with representative variables including quotes, backslashes,
   Unicode, newline, empty string, array, object, null, and control-character
   cases where the field type permits them;
5. parse the complete output with a real JSON parser;
6. compare the parsed semantic payload to the expected object, not just a
   string snapshot;
7. run the legacy-mode render as a compatibility fixture before deleting the
   old source shape where practical;
8. record the migration in the release notes and repository migration guide.

For repositories that cannot migrate immediately, the owner may add
`json_escape_mode: legacy` or pass `--json-escape-mode legacy`. That path must
remain safe content escaping and must produce the deprecation warning.

## 12. Test strategy

### 12.1 Renderer unit tests

Add tests in `crates/sc-composer/src/renderer.rs` or a focused renderer test
module for:

1. auto mode string value round-trip with quote, backslash, newline, tab,
   control character, and Unicode content;
2. auto mode injection value cannot create a second top-level key;
3. auto mode bare string placeholder produces valid JSON;
4. auto mode object, array, number, boolean, and null placeholders preserve
   their JSON types;
5. legacy mode manually quoted string placeholder produces valid JSON and
   round-trips the original string;
6. legacy mode escapes the same hostile input as auto mode without adding
   another pair of quotes;
7. legacy mode rejects a non-string value in a quoted-string position;
8. legacy mode warns exactly once per template;
9. an explicit mode beats the compatibility default;
10. CLI/request override beats frontmatter;
11. conflicting include modes produce a stable diagnostic;
12. non-JSON templates are unaffected by JSON mode;
13. existing HTML/XML/CDATA/Turtle tests remain unchanged and pass;
14. renderer mode selection uses the same suffix stripping as template-init;
15. filename/frontmatter format mismatch is diagnosed deterministically.

### 12.2 Post-render parser tests

Add focused tests for the shared output checker:

1. valid JSON object, array, scalar, and whitespace-only-at-boundaries cases;
2. malformed double-quoted output returns the stable invalid-JSON code;
3. parser line/column/offset are retained;
4. secrets are not included in the diagnostic message;
5. non-JSON format is not accidentally parsed as JSON;
6. parser failure prevents file write and stdout body emission;
7. `render --json` wraps the error in the normal versioned envelope;
8. `bytes_written` is not reported as a successful write on failure.

### 12.3 Template-lint tests

Add tests for:

1. quoted scalar placeholder in a JSON template is located with line/column;
2. the same source in legacy mode emits a deprecation warning, not a false
   auto-mode error;
3. the same source in auto mode emits the migration/error diagnostic;
4. a bare placeholder in auto mode is clean;
5. a raw JSON field is not falsely flagged when explicitly structured;
6. loop elements and nested arrays are handled;
7. Jinja comments/string literals do not create false positives;
8. includes report the owning source path and include chain;
9. `validate --lint` preserves the existing redundant-chain rule;
10. warning vs error exit behavior is stable.

### 12.4 CLI integration tests

Add tests under `crates/sc-compose/tests/` for:

1. `validate` static report says render was not checked;
2. `validate --lint` reports the quoted-placeholder diagnostic;
3. `validate --check-render --var-file` reports valid JSON for the exact
   context;
4. `validate --lint --check-render --json` emits one envelope containing both
   static and render diagnostics;
5. `render` rejects malformed JSON before writing;
6. `render --json` returns the parse diagnostic in the payload envelope;
7. `--json-escape-mode legacy` renders the six old-shape fixtures safely;
8. `--json-escape-mode auto` renders bare fixtures safely;
9. frontmatter mode and CLI override precedence;
10. an invalid mode is a usage/configuration error;
11. `template-init` emits `json_escape_mode: auto` and bare placeholders for
    JSON files;
12. template-init followed by render is a semantic JSON round-trip;
13. a manually authored old template is detected before release output;
14. output to a file and output to stdout use the same parser gate;
15. `--all`/multi-pass validation applies the check to every selected pass and
    reports the pass identity for a failing render.

### 12.5 Repository lint tests

The `template-contracts` target must have a fixture repository containing:

- one valid auto-mode template;
- one valid legacy-mode template with deprecation warning;
- one auto-mode quoted-placeholder failure;
- one injection attempt;
- one nested array/loop fixture;
- one raw structured JSON field;
- one conditional branch fixture;
- one missing-context case;
- one include graph with a mode conflict;
- one non-JSON control template proving scope isolation.

Tests must invoke the same target through:

```text
sc-compose lint --target template-contracts --root <fixture-root> --json
just lint target=template-contracts
```

The report must distinguish findings from capability/configuration errors and
must not pass when the target did not actually inspect the fixture set.

### 12.6 Fuzz campaign changes

Update `.claude/skills/adversarial-fuzzing/SKILL.md` and the coordinator
contract so JSON renderer campaigns include both compatibility shapes:

| Probe | Source | Oracle |
| --- | --- | --- |
| secure auto | bare placeholder | valid JSON; no injection |
| legacy compatibility | literal-quoted placeholder | valid JSON; warning; no injection |
| mode mismatch | quoted source in auto and bare source in legacy | stable diagnostic/fail closed |
| template-init round-trip | generated JSON template | semantic round-trip |
| output contract | every successful render | parse complete body before PASS |
| release corpus | all known JSON templates | exact context or explicit missing-context result |

The worker result must record the effective release/binary commit, template
path, mode, exact minimal template, exact input context, observed exit code,
parser result, and whether the parser gate ran. “Rendered text was produced”
must not count as a pass for a JSON target.

## 13. Rollout sequencing

### 13.1 Sprint 1 — mode and checked-render core

**Depends on:** none. **Unblocks:** Sprint 2.

Work:

- define and parse `JsonEscapeMode` and effective-mode precedence;
- implement safe legacy content escaping and retain safe auto escaping;
- define stable diagnostics and result fields;
- implement the shared post-render JSON checker;
- wire the library-level checked-render report;
- update `template-init` JSON generation;
- add renderer and parser unit tests, including both old and new source shapes.

Do not add repository-wide lint orchestration in this sprint. The core contract
must be independently testable before CLI/report integration.

### 13.2 Sprint 2 — CLI paths, lint target, migration, and release gate

**Depends on:** Sprint 1's mode/checker contract. **Can run in parallel after
the core API is stable:** CLI integration, template-lint rules, and migration
fixture preparation may be developed in separate worktrees; final integration
and release-corpus verification are sequential.

Work:

- add the canonical `--check-render` and `--json-escape-mode` CLI surfaces;
- update `validate`, `validate --lint`, `render`, and JSON envelopes;
- add `template-contracts` to the allowlisted sc-lint target registry;
- wire `just lint` and the full report aggregation;
- migrate the six known templates to auto mode;
- add compatibility fixtures for unmodified legacy templates;
- update requirements, help, migration docs, changelog, and ATM-core adapter
  contract documentation outside this repository as a follow-up;
- run the full cross-repository release-corpus check;
- run the revised fuzz campaign against the release candidate.

### 13.3 Shipping decision

Ship the mode, diagnostics, parser gate, and CLI integration together in
1.4.1. Do not ship only the renderer mode change again: that is the failure
mode that produced this incident.

The six in-repository templates should migrate in the same release or in the
release branch immediately before the binary is published. The legacy mode and
warning are required in 1.4.1 because external repositories cannot all be
edited atomically.

The default-mode removal is a later breaking change only after:

- downstream inventory confirms the high-traffic repositories have migrated;
- the release-corpus lint target is green in those repositories;
- ATM-core uses the checked-render report;
- the deprecation warning has been present for a documented compatibility
  window;
- a release note calls out the default change.

## 14. Dependency graph and ownership

```text
S1 mode + checker core
 ├── S2a validate/render CLI integration
 ├── S2b template lint rules
 ├── S2c sc-compose lint target + just lint integration
 └── S2d six-template migration fixtures
       └── S2e release-corpus/fuzz verification
             └── 1.4.1 release gate
```

Ownership:

- `crates/sc-composer/src/renderer.rs`: mode semantics and escaping;
- `crates/sc-composer/src/diagnostics/`: stable diagnostic/result schema;
- `crates/sc-composer/src/` checked-render module: output parser gate;
- `crates/sc-compose/src/commands/compose.rs`: validate path;
- `crates/sc-compose/src/commands/compose_render.rs`: render emission gate;
- `crates/sc-compose/src/commands/template_lint.rs`: source lint;
- `crates/sc-compose/src/commands/template_init.rs`: generated JSON shape;
- `crates/sc-compose/src/commands/sc_lint.rs` and `.sc/sc-lint/targets/`:
  repository target integration;
- `justfile`: stable repository lint entry point;
- `.claude/skills/adversarial-fuzzing/`: campaign oracle and regression
  promotion rules;
- the six `.json.j2` templates: consumer migration fixtures.

The sc-composer crate remains runtime-agnostic. No ATM imports, mailbox logic,
or ATM-core dependency may be introduced to implement this contract.

## 15. Requirements and documentation updates

Before implementation begins, update the planning branch's requirements or
ADR only where needed:

1. document that JSON output is a format contract, not merely rendered text;
2. define `json_escape_mode` and the compatibility deprecation;
3. define the distinction between static validation and context-specific
   render validation;
4. define the machine-readable fields ATM-core must consume;
5. define that a checked JSON render cannot emit malformed output or return
   success;
6. update FR-8/FR-8a language to cover parser-backed output diagnostics;
7. add a migration note for existing quoted placeholders.

Reserve `ADR-0019 — JSON Render Contract and Fail-Closed Output Validation`.
Architecture owns acceptance and O.1 must not dispatch implementation before
the ADR and detailed design acceptance are recorded. The ADR is a planning
gate, not a request to create a second implementation of the renderer rules.

## 16. Acceptance criteria

### Contract

- [ ] `legacy` and `auto` have precise, tested semantics.
- [ ] legacy mode safely escapes contents and does not reintroduce injection.
- [ ] auto mode owns complete JSON values and rejects the old quoted shape
      through lint/check diagnostics.
- [ ] CLI/frontmatter/default precedence is documented and tested.
- [ ] mode conflicts and non-JSON misuse have stable diagnostics.

### All execution paths

- [ ] plain `validate` clearly reports static-only status.
- [ ] `validate --lint` reports the quoted-placeholder anti-pattern with source
      location and preserves existing lint rules.
- [ ] `validate --check-render` validates the exact context without writing.
- [ ] `validate --lint --check-render` combines both layers.
- [ ] `render` fails closed before emission for malformed JSON.
- [ ] `render --json` preserves the versioned diagnostic envelope.
- [ ] `sc-compose lint --target template-contracts` reports the same findings.
- [ ] `just lint` includes the target without duplicating the core scanner.

### ATM-core contract

- [ ] machine-readable result distinguishes static and context-specific checks.
- [ ] ATM-core integration guidance says to inspect fields and diagnostics,
      not only process exit status.
- [ ] the result does not claim arbitrary-branch proof from one fixture.
- [ ] the checked render happens before the output is cached or sent.

### Regression and migration

- [ ] all six known templates either migrate to auto mode or have an explicit
      legacy declaration during the compatibility window.
- [ ] six-template representative contexts parse as JSON.
- [ ] injection values cannot add keys or alter JSON structure.
- [ ] template-init JSON output round-trips through render.
- [ ] fuzz campaigns test both source idioms and parse every successful JSON
      body.
- [ ] release-candidate fuzz reports record the binary/commit and parser oracle.
- [ ] no HTML/XML/CDATA/Turtle behavior regresses.

### Quality gates

- [ ] `cargo test --workspace` passes.
- [ ] `cargo fmt --all --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `just lint` passes with the template contract target enabled.
- [ ] documentation and changelog identify the compatibility mode and
      migration.
- [ ] no implementation code is committed to this planning worktree.

## 17. Review questions for QA and architecture

The plan is ready for review when reviewers can answer “yes” to these
questions:

1. Does legacy mode preserve the old source syntax without permitting JSON
   injection?
2. Does auto mode remain the secure recommended contract for new templates?
3. Can a caller tell the difference between static validation and a successful
   render with the exact context?
4. Does malformed JSON fail before any file/stdout emission?
5. Do `validate --lint` and `sc-compose lint` share implementation and codes
   while retaining their different scopes?
6. Can ATM-core consume a stable result without importing ATM runtime code?
7. Are all six known templates covered by semantic JSON tests?
8. Does the pinned `docs/phase-O/release-corpus-roots.txt` inventory identify
   old quoted templates in every available consumer repository before the
   compatibility default is removed?
9. Does the fuzz oracle test both source forms and parse successful output?
10. Does the rollout ship the renderer, detector, diagnostics, and migration
    path together rather than repeating the 1.4.0 partial-fix failure?

## 18. Plan validation status

This document has been grounded against:

- `crates/sc-composer/src/renderer.rs`;
- `crates/sc-compose/src/commands/compose.rs`;
- `crates/sc-compose/src/commands/compose_render.rs`;
- `crates/sc-compose/src/commands/template_lint.rs`;
- `crates/sc-compose/src/commands/template_init.rs`;
- `crates/sc-compose/src/commands/sc_lint.rs`;
- `crates/sc-compose/src/extract/json.rs`;
- `docs/requirements.md` FR-1/FR-2/FR-3/FR-7/FR-8/FR-8a;
- `justfile` and `.sc/sc-lint/targets/`;
- `docs/sprints/fix-272-format-aware-escaping.md`;
- the 2026-08-11 fuzz report and adversarial-fuzzing oracle.

The planning decisions are resolved as follows:

- the only CLI spelling is `--check-render`;
- ordinary unflagged JSON `render` is fail-closed in 1.4.1 before emission;
- `legacy` is the compatibility default for every unannotated JSON template in
  1.4.1, with a deprecation diagnostic; no pre-1.4 marker is required;
- O.4 must migrate all six known templates or record a reviewed, fixture-backed
  legacy exception before O.5 release-corpus work begins;
- ATM-core consumes the structured `RenderCheckReport` defined by the Phase O
  plan and supplies the exact context; consumer-side adapter implementation is
  not sc-compose production scope;
- removal of the legacy default is a later breaking-release decision, gated by
  migration and corpus evidence, and is not an open Phase O dependency.

## 19. Closeout evidence

To be filled by the implementation and QA worktrees. This section must record:

- implementation commit(s);
- the release-candidate version/commit tested;
- exact command lines for `validate`, `validate --lint`,
  `validate --check-render`, `render`, `sc-compose lint`, and `just lint`;
- the six-template semantic JSON results;
- the fuzz session IDs and parser-oracle results;
- QA decision and any deferred findings.
