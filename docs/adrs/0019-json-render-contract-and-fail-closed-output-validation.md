# ADR-0019: JSON Render Contract and Fail-Closed Output Validation

## Status

Accepted (2026-08-13) — merged via PR #421, quality-mgr PASS

## Context

Release 1.4.0 changed JSON escaping to use MiniJinja's JSON auto-escaping.
That behavior is correct for a bare value placeholder:

```json
{"worktree_path": {{ worktree_path }}}
```

but it is incompatible with the older manually quoted source shape:

```json
{"worktree_path": "{{ worktree_path }}"}
```

The latter can become invalid JSON with two pairs of quotes around a rendered
string. This is a real compatibility problem for templates distributed by
other repositories, so a release cannot assume that all downstream source
files can be migrated atomically.

The incident also exposed a validation boundary: a template can pass static
validation while a particular render context produces malformed JSON. A
successful process exit or a static validation result must not authorize
emitting or caching output that has not been parsed as JSON. ATM-core needs a
machine-readable result that distinguishes static contract validity from a
render checked with its actual context.

## Decision

### 1. Define two explicit JSON escape modes

sc-compose supports exactly these modes for JSON templates:

- `auto` is the default and recommended mode. A placeholder supplies a
  complete JSON value; strings receive their JSON quotes and structured values
  retain their JSON structure.
- `legacy` is an explicit compatibility mode for existing manually quoted
  string placeholders. It escapes JSON string contents without adding the
  surrounding quotes already present in the source.

`legacy` is never raw interpolation. It must escape quotes, backslashes,
control characters, and other JSON string content safely. It is temporary
compatibility behavior and emits the migration warning below. A future
breaking release may remove it after downstream migration evidence and a
documented compatibility window.

The effective mode has deterministic precedence:

1. an explicit `--json-escape-mode <legacy|auto>` CLI override;
2. `json_escape_mode: legacy|auto` in root template frontmatter;
3. default `auto` for an unannotated JSON template.

Included templates do not silently override the root mode. Conflicting mode
declarations are a validation finding that identifies both paths.

Implementation status (1.4.1): declaration conflicts in an expanded include
chain emit `ERR_JSON_MODE_INCLUDE_CONFLICT` with the root and included paths
and both effective/declarative modes. Matching declarations and includes that
omit a mode continue to inherit the root mode without a finding.

The implementation must not make heuristic source detection the primary mode
selection mechanism. A quoted placeholder found in JSON context is diagnosed
and guided toward migration; correctness must not depend on guessing whether a
quote belongs to a Jinja construct or to the surrounding JSON string.

### 2. Require migration-directed diagnostics

`validate` and `validate --lint` emit one deprecation warning per affected
template when an explicit `legacy` mode is selected or a quoted placeholder is
detected in a JSON context. The warning text is stable and must be:

```text
Template uses legacy JSON escape mode. Migrate to bare placeholders (auto mode) to avoid double-quoting issues. See docs/migration/json-escape-mode.md
```

The diagnostic includes its stable code, template path, and source location
when available. The warning is migration guidance, not permission to emit
unchecked output. In `auto` mode, a render that would produce malformed JSON
still fails closed.

The migration guide is owned by Phase O.1 at
`docs/migration/json-escape-mode.md`; O.3 owns diagnostic emission and O.4
updates the guide with the six-template migration matrix.

### 3. Check JSON output before CLI emission

Every JSON render path in the `sc-compose` CLI invokes the shared output
checker before writing stdout or a file. The checker parses the complete
rendered body and returns structured location data without echoing the
potentially sensitive payload. A structural JSON parse failure is an error,
not a successful result with a false flag.

Within that CLI path, the checked-output capability is the only value accepted
by an emitter. A caller cannot construct a successful checked output directly
or bypass the parser gate through a boolean. `validate --check-render`
performs the same check in memory and emits no body. Plain `validate` remains
static-only and does not claim that a future context will render valid JSON.

The authoritative Rust-level shape is:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonEscapeMode {
    Legacy,
    Auto,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenderCheckMeta {
    pub template: PathBuf,
    pub output_format: OutputFormat,
    pub json_escape_mode: Option<JsonEscapeMode>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RenderCheckReport {
    StaticOnly {
        meta: RenderCheckMeta,
        diagnostics: Vec<Diagnostic>,
    },
    ContractInvalid {
        meta: RenderCheckMeta,
        diagnostics: Vec<Diagnostic>,
    },
    ContextRequired {
        meta: RenderCheckMeta,
        diagnostics: Vec<Diagnostic>,
    },
    RenderInvalid {
        meta: RenderCheckMeta,
        diagnostics: Vec<Diagnostic>,
    },
    RenderChecked {
        meta: RenderCheckMeta,
        checked_context: ContextSummary,
        diagnostics: Vec<Diagnostic>,
    },
}

pub struct CheckedOutput {
    body: String,
    meta: RenderCheckMeta,
}

impl CheckedOutput {
    pub fn emit<W: std::io::Write>(&self, writer: W) -> std::io::Result<()>;
}

pub fn check_rendered_output(
    format: OutputFormat,
    template: &Path,
    rendered: &str,
) -> Result<CheckedOutput, OutputCheckError>;

#[derive(Debug)]
#[non_exhaustive]
pub struct OutputCheckError {
    pub reason: OutputCheckReason,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum OutputCheckReason {
    InvalidJson {
        line: usize,
        column: usize,
        byte_offset: usize,
    },
    ContractViolation,
    RenderFailure,
}
```

For a structural JSON parse failure, `check_rendered_output` returns
`Err(OutputCheckError { reason: InvalidJson { .. }, .. })`; it never returns an
`Ok` value containing `valid: false`. `RenderInvalid` is the serialized report
projection for callers and is not a recoverable emission channel. The report
is state-shaped so callers cannot represent contradictory combinations such as
“not checked” with “render valid.”

### 3.1 Library boundary: Checked-Emission Caller Contract

The CLI-only enforcement above does not make the public
`sc_composer::compose()` library boundary typestate-safe.
`sc_composer::compose()` returns a public `ComposeResult` whose
`rendered_text` is a plain `String`; a library consumer can read that string
without calling the checker. Therefore the following is a named caller
contract for every library consumer that emits or caches a composition:

1. Compose with the exact context and options intended for emission.
2. Run `check_rendered_output` on the complete final text, using the resolved
   template path and the appropriate `OutputFormat`.
3. Discard the raw `ComposeResult::rendered_text` for emission and emit only
   the returned `CheckedOutput` through `CheckedOutput::emit` after the check
   succeeds. Any `OutputCheckError` denies emission or caching.

This is a documented caller responsibility, not an automatically enforced
guarantee at the `sc-composer` boundary. A bundled `compose_checked()` API is
not added by ADR-0019: making it correct would require deciding how callers'
format policy, final output assembly, and checked-report context should be
represented. That additive API is deferred to a future, explicitly named
**Checked Library Composition API** sprint.

### 4. Define the command and consumer contract

The commands retain distinct scopes while sharing the mode resolver, source
scanner, output checker, and diagnostic schema:

| Command | Contract |
| --- | --- |
| `validate` | Static-only result; report effective mode and migration diagnostics, but make no render-validity claim. |
| `validate --lint` | Static validation plus source diagnostics, including the migration warning. |
| `validate --check-render` | Render in memory with supplied context and parse the complete output. |
| `validate --lint --check-render` | Return source and checked-render diagnostics together. |
| `render` | Resolve mode, check JSON output, then emit only a successful checked output. |
| `render --json` | Preserve its envelope while representing body-check failures as structured diagnostics. |
| `sc-compose lint --target template-contracts` | Inventory templates with the same library-owned scanner and diagnostic codes. |

ATM-core supplies the exact context it will use, consumes the structured
`RenderCheckReport`, and sends or caches output only when the state is
`render_checked`. `static_only`, `context_required`, `contract_invalid`, and
`render_invalid` are not permission to send or cache. ATM-core must not infer
permission from process exit status alone.

## Consequences

- New JSON templates and `template-init` use the safe bare-placeholder
  contract and default to `auto`.
- Existing quoted templates remain usable through an explicit, safely escaped
  `legacy` mode while every validation path directs owners to migrate.
- Malformed JSON cannot be emitted through the CLI's checked render path, and
  callers receive stable machine-readable failure states and locations.
- ATM-core integration has an explicit capability boundary: it must provide a
  context and inspect `RenderCheckReport` rather than treating static validity
  as proof of render validity.
- Library consumers of `sc-composer::compose()` must follow the named
  Checked-Emission Caller Contract; the public raw `ComposeResult` string is
  not itself proof that output was checked.
- `validate`, `validate --lint`, `sc-compose lint`, and `just lint` can share
  implementation without becoming interchangeable commands or duplicating
  scanner logic.
- The six known in-repository templates require a reviewed migration or a
  fixture-backed explicit legacy exception before the release-corpus gate.
- Downstream repositories need a migration window. Removal of `legacy` is a
  later breaking-release decision gated by inventory, green corpus lint, ATM-
  core adoption of the checked result, and release documentation.

## Non-goals

- Do not restore unsafe 1.3 raw interpolation.
- Do not change HTML, XML, CDATA, Turtle, YAML, or Markdown semantics.
- Do not claim that arbitrary Jinja programs can be statically proven valid for
  every future context.
- Do not retry malformed output after it has been emitted.
- Do not add ATM runtime or mailbox dependencies to sc-compose.
- Do not make heuristic auto-detection the authoritative mode resolver.

## Implementation and acceptance boundary

This ADR records the accepted contract; the implementation plan remains in
[`docs/phase-O/phase-O-plan.md`](../phase-O/phase-O-plan.md) and its O.1–O.5
sprint documents. No production implementation should begin until this ADR
and the Phase O design-acceptance gate are accepted. O.1 owns the mode and
shared output-check infrastructure; O.2 owns checked-render and CLI emission
integration; O.3 owns lint diagnostics and the repository lint target; O.4
owns the six-template migration; O.5 owns the release corpus and fuzz gate.

Acceptance requires, at minimum:

- precedence tests proving CLI > frontmatter > default `auto`;
- legacy quoted-string fixtures that escape safely and emit the exact warning;
- auto bare scalar/object/array fixtures and the FIX-272 injection fixture;
- malformed JSON tests proving no stdout/file bytes are emitted;
- state-shaped report serialization tests and ATM-core consumption tests;
- `validate`/`validate --lint` warning tests, including the migration link;
- cross-platform workspace tests and repository lint checks;
- reviewed migration/legacy decisions for all six known templates.

## References

- [Phase O plan](../phase-O/phase-O-plan.md)
- [Phase O detailed design](../sprints/plan-json-format-escape-mode.md)
- [Sprint O.1 — JSON Mode Contract](../phase-O/sprint-o-1-json-mode-contract.md)
- [Sprint O.2 — Checked Render Contract](../phase-O/sprint-o-2-checked-render-contract.md)
- [Sprint O.3 — Template Lint and Repository Target](../phase-O/sprint-o-3-template-lint-and-repo-target.md)
- [Sprint O.4 — Template Migration](../phase-O/sprint-o-4-template-migration.md)
- [Sprint O.5 — Release Corpus and Fuzz Gate](../phase-O/sprint-o-5-release-corpus-fuzz-gate.md)
- PR #420: JSON format escape mode and fail-closed render plan
