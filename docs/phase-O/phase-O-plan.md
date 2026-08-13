---
phase: O
title: JSON Render Contract and Fail-Closed Output Validation
status: complete
target: integrate/phase-o
final_merge: 4d37280 (PR #438), with follow-on fixes in PRs #440 and #441
planning_branch: plan/json-format-escape-mode
design_acceptance: recorded 2026-08-13 (PR #420, quality-mgr PASS at d380ff3)
adr_acceptance: recorded 2026-08-13 (ADR-0019, PR #421, quality-mgr PASS)
---

# Phase O — JSON Render Contract and Fail-Closed Output Validation

## Phase goal

Make JSON template rendering safe, backwards-compatible, and machine-verifiable
after the 1.4.0 regression in which `AutoEscape::Json` double-quoted existing
manually quoted placeholders. The phase also closes the detection gap: a
malformed JSON body must not be emitted with exit status 0, and ATM-core must be
able to distinguish static validation from a successful render of a specific
context.

This phase is based on the design in
`docs/sprints/plan-json-format-escape-mode.md`. The phase documents the
implementation work as independently reviewable sprints; it does not add
production code in this planning worktree.

The detailed design source remains the linked plan above; this Phase O folder
is the authoritative execution index and sprint decomposition for team
dispatch, dependency tracking, and QA handoff.

The detailed design document (`docs/sprints/plan-json-format-escape-mode.md`)
was accepted 2026-08-13 by team-lead and quality-mgr (PR #420, plan_gate PASS
at commit d380ff3). The design-acceptance gate below is satisfied.

## ADR reservation and dispatch gate

`ADR-0019 — JSON Render Contract and Fail-Closed Output Validation` was
accepted 2026-08-13 (PR #421, quality-mgr PASS). Both the detailed design
acceptance and ADR-0019 acceptance gates are now satisfied; O sprints may be
dispatched.

## Incident and evidence

The affected source shape is:

```json
{"worktree_path": "{{ worktree_path }}"}
```

In 1.3 it rendered valid JSON. In 1.4, after FIX-272 added JSON auto escaping,
it can render:

```text
{"worktree_path": ""/abs/path""}
```

The current renderer selects `AutoEscape::Json` in
`crates/sc-composer/src/renderer.rs::legacy_auto_escape_callback` and delegates
to minijinja's JSON formatter in `format_sc_compose_markup`. That behavior is
secure for the new bare form:

```json
{"worktree_path": {{ worktree_path }} }
```

but incompatible with the old source idiom.

The 2026-08-11 fuzz report at
`site/reports/20260811-3-fuzz-report/20260811-3-fuzz-report.json` did find the
underlying incompatibility. The shape probe reported one confirmed failure in
46 iterations and correctly identified the mismatch between `template-init`
and the renderer. It did not close the broader release contract because the
promoted renderer test covered the corrected bare form, no compatibility-mode
matrix existed, and the normal render path had no parser-backed JSON output
gate. The differential probe also explicitly treated the distinction between
`validate` and `render` as intentional because `validate` is documented as not
rendering output.

## Design decision

Use explicit JSON escape modes plus format-aware post-render validation:

| Mode | Source contract | Behavior | Rollout |
| --- | --- | --- | --- |
| `legacy` | existing literal-quoted string placeholders | JSON-escape string contents without adding outer quotes | explicit compatibility mode with deprecation warning |
| `auto` | bare placeholders | renderer emits complete JSON values, including string quotes | 1.4.1 default and recommended mode |

Effective mode precedence:

1. `--json-escape-mode` CLI override;
2. `json_escape_mode` in root template frontmatter;
3. 1.4.1 default `auto` for unannotated JSON templates.

The CLI spelling is fixed as `--json-escape-mode <legacy|auto>` and the root
frontmatter key is `json_escape_mode: legacy|auto`. No alternate flag spelling
or implicit mode alias is part of this phase.

`legacy` must never mean raw interpolation. It must safely escape quotes,
backslashes, control characters, and JSON string content. A future breaking
release may remove the legacy mode after the migration and deprecation window;
the absent-mode default is already `auto` in 1.4.1.

JSON rendering is checked before emission. `validate` remains static-only by
default, while `validate --check-render` performs an in-memory checked render
with the supplied context. `validate --lint` adds source diagnostics, and
`sc-compose lint --target template-contracts` inventories repository templates
using the same library-owned scanner and diagnostic codes. `just lint` includes
that target through the existing sc-compose lint runner rather than duplicating
Python or shell logic.

## Requirements and traceability

| ID | Contract | Sprint |
| --- | --- | --- |
| O-R1 | Existing manually quoted JSON templates have a safe compatibility mode and deprecation warning. | O.1 |
| O-R2 | New/migrated JSON templates use secure complete-value auto escaping. | O.1 |
| O-R3 | CLI/frontmatter/default mode precedence is deterministic and documented. | O.1 |
| O-R4 | A checked JSON render parses the complete body before emission and fails closed. | O.2 |
| O-R5 | Machine-readable results distinguish static contract validity from context-specific render validity. | O.2 |
| O-R6 | `validate`, `validate --lint`, `render`, `sc-compose lint`, and `just lint` expose the appropriate shared checks without conflating scope. | O.2, O.3 |
| O-R7 | The six known repository templates have a reviewed source-shape classification and migration/legacy decision. | O.4 |
| O-R8 | ATM-core has an actionable integration contract and must not use exit status alone. | O.2, O.4 |
| O-R9 | Six known repository templates are migrated where safe, with compatibility fixtures green for every legacy exception. | O.4 |
| O-R10 | Cross-repository release-corpus inventory and fuzz campaigns test both interpolation shapes and parse every successful JSON body. | O.5 |
| O-R11 | No JSON escaping regression reopens the FIX-272 injection vulnerability. | O.1, O.2, O.4, O.5 |
| O-R12 | `validate` and `validate --lint` emit the exact migration-directed deprecation warning for legacy JSON mode or quoted placeholders. | O.1, O.3 |

## Scope boundary

In scope:

- JSON escape mode parsing and renderer behavior;
- safe legacy content-only escaping;
- checked JSON output parsing;
- stable diagnostics and JSON envelope fields;
- source lint for quoted scalar placeholders;
- repository template-contract lint target and `just lint` integration;
- `template-init` JSON generation alignment;
- six known template migrations;
- fuzz oracle and release-corpus updates;
- ATM-core adapter documentation and machine contract.

Out of scope:

- ATM runtime implementation or mailbox integration;
- changing HTML, XML, CDATA, Turtle, YAML, or Markdown semantics;
- making arbitrary Jinja programs statically prove every possible future
  context;
- retrying malformed output after it has been emitted;
- restoring unsafe 1.3 raw interpolation;
- vendoring sc-lint Python scripts;
- modifying external repositories in this phase.

## Command contract

| Command | Default responsibility | New/changed responsibility |
| --- | --- | --- |
| `validate` | static includes, variables, and frontmatter | report static-only status and effective JSON mode; no false render claim |
| `validate --lint` | static validation plus template-source lint | detect quoted-placeholder mode mismatch and migration needs |
| `validate --check-render` | not currently present | render in memory with supplied context and parse output |
| `validate --lint --check-render` | combined static path | return source and rendered-output diagnostics together |
| `render` | compose and emit | use effective mode and fail closed for malformed JSON before emission |
| `render --json` | envelope around rendered body | preserve envelope while reporting body parser failures as structured diagnostics |
| `sc-compose lint --target template-contracts` | not currently present | enumerate templates and run shared static/fixture checks |
| `just lint` | delegates to sc-compose/sc-lint profiles | include template-contracts in the full repository quality profile |

The commands remain separate because they have different scope and dependency
costs. They share the mode resolver, scanner, output checker, and diagnostic
schema so their conclusions cannot drift.

## Authoritative checked-render contract

This section is the single source of truth for the parser gate and
`RenderCheckReport`. O.2, the CLI contract table, and the ATM-core section
below must implement or reference this contract; they must not introduce
alternate field names, default behavior, or parser timing.

The authoritative API shape is:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

For JSON, `check_rendered_output` parses the complete body before emission,
returns stable line/column/offset diagnostics without echoing the payload, and
is a no-op for formats outside this phase. `render --check-render` and the
1.4.1 default JSON render path invoke it before stdout/file emission.
`validate --check-render` invokes it in memory and emits no body. Plain
`validate` returns `RenderCheckReport::StaticOnly` and does not claim output
validity. The report is state-shaped: callers cannot represent contradictory
combinations of static validity, render execution, and render validity.

The channel rule is authoritative: `check_rendered_output` returns
`Err(OutputCheckError { reason: InvalidJson { .. }, .. })` for a structural
parse failure and never returns an `Ok` value with `valid: false`; an `Ok`
`CheckedOutput` is the only value permitted to reach an emitter, while
`RenderCheckReport::RenderInvalid` is its structured serialized projection for
callers and not a recoverable success channel. Advisory diagnostics may be
reported in the non-emitting states only.

The 1.4.1 default is resolved now: ordinary unflagged JSON `render` fails
closed before emitting malformed JSON. There is no opt-in transition period.
The effective mode for an unannotated template is `auto`; legacy compatibility
is provided only by the explicit `legacy` mode, not by allowing an unchecked
render.

The required migration diagnostic is emitted by `validate` and
`validate --lint` once per affected template:

```text
Template uses legacy JSON escape mode. Migrate to bare placeholders (auto mode) to avoid double-quoting issues. See docs/migration/json-escape-mode.md
```

The diagnostic includes the stable code, template path, and source location
when available. An unannotated quoted placeholder is never silently treated as
legacy: it uses `auto`, receives this migration guidance, and a render that
would produce malformed JSON still fails closed before emission.

O.1 owns creation of `docs/migration/json-escape-mode.md`; O.3 specifies the
diagnostic emission and O.4 updates the guide with the six-template migration
matrix.

## ATM-core contract

The library/CLI result must expose at least:

```json
{
  "state": "render_checked",
  "template": "path/to/assignment.json.j2",
  "output_format": "json",
  "json_escape_mode": "auto",
  "checked_context": "caller-defined exact context summary",
  "diagnostics": []
}
```

ATM-core must supply the exact context it will use, call the checked validation
or library API, inspect the structured result, and cache/send only when
`state == "render_checked"`. `static_only`, `context_required`,
`contract_invalid`, and `render_invalid` states are not permission to send or
cache. Static validity without a context must not be represented as proof that
a future render will parse. The result must not include the full prompt body
unless explicitly requested.

## Ordering and parallelism

O.1 is the only infrastructure sprint. It defines the mode resolver, shared
types, safe escaping, and core parser-check API.

After O.1 is merged to `integrate/phase-o`, O.2 and O.3 may execute in parallel:

- O.2 owns checked-render behavior, `render`/`validate` CLI integration, and the
  ATM-core machine contract.
- O.3 owns `validate --lint` source rules, the `sc-compose lint` command's
  template-contract target, report aggregation, and `just lint` wiring. It
  does not own O.2's render/validate parser gate.

O.4 depends on the merged behavior from O.2 and O.3. It owns only the six
in-repository template migrations and compatibility fixtures. O.5 then owns
the cross-repository inventory, release-corpus verification, fuzz-oracle
changes, and 1.4.1 release gate.

O.2 and O.3 must not duplicate the shared parser or mode implementation. O.4
and O.5 must not begin release-corpus claims until both parent contracts and
the six-template migration are available.

O.2 and O.3 both touch these four shared files for diagnostics, CLI fixtures,
and requirements: `crates/sc-composer/src/diagnostics/schema.rs`,
`crates/sc-compose/tests/cli/validate.rs`,
`crates/sc-compose/tests/json_cli/validate.rs`, and
`docs/requirements.md`. To avoid concurrent edit loss, O.2 owns the initial
changes in all four files; O.3 must rebase onto O.2's merged commit before
adding its non-overlapping source-lint changes, may only add entries/fixtures,
and must not rewrite O.2 changes. If any shared file's shape changes during
O.2 QA, O.3 pauses and rebases again.

## Sc-lint findings and QA routing

Every implementation sprint must run the applicable sc-lint targets against its
final sprint commit. Minor findings may be fixed in that sprint worktree.
Remaining findings require a dedicated `fix/` worktree created from that sprint
worktree's final commit and grouped by independent rule class:

- group mechanical constant/identity findings by owning crate;
- keep length-driven refactors separate by violating file/refactor;
- do not create one worktree per warning in the same crate;
- do not mix semantic output fixes with unrelated lint refactors.

The developer sends team-lead the fix worktree, parent sprint commit, finding
class, evidence, tests, and fix commit. Team-lead creates the PR and routes it
to quality-mgr. The sprint is not closed until required fix PRs are QA-approved,
merged, and revalidated. Each sprint below repeats this handoff gate locally.

## Sprint inventory

| Sprint | Scope | Dependency | Parallelism |
| --- | --- | --- | --- |
| O.1 | JSON mode contract, safe escaping, template-init alignment | none; starts from approved phase parent | infrastructure; blocks O.2/O.3 |
| O.2 | Checked render API/CLI, parser gate, ATM-core contract | O.1 merged | parallel with O.3 |
| O.3 | Template lint, sc-compose lint target, just lint/report integration | O.1 merged | parallel with O.2 |
| O.4 | Six-template migration and compatibility fixtures | O.2 and O.3 merged | parallel with inventory preparation; blocks O.5 |
| O.5 | Cross-repository inventory, release corpus, fuzz oracle, 1.4.1 gate | O.4 merged | final closure; no parallel O sprint |

## Phase acceptance and closeout

- [x] O.1–O.5 have QA-approved merge evidence.
- [x] 1.4.1 preserves safe legacy compatibility without raw interpolation.
- [x] auto mode remains injection-safe for hostile strings and structured
      values.
- [x] malformed JSON cannot be emitted as a successful checked render.
- [x] `RenderCheckReport` is state-shaped, parser failure uses the typed
      `Err` channel, and only `CheckedOutput` can reach an emitter.
- [x] `validate` clearly reports static-only status.
- [x] `validate --lint` reports quoted-placeholder source findings.
- [x] `sc-compose lint --target template-contracts` and `just lint` run the
      shared check without duplicate implementation.
- [x] ATM-core integration guidance uses the checked result and exact context.
- [x] all six known templates are migrated or explicitly legacy and parsed by
      semantic tests.
- [x] cross-repository release-candidate fuzz runs both old and new
      interpolation shapes.
- [x] `cargo test --workspace`, formatting, clippy, and the authoritative
      provisioned repository-lint profile pass; bare local `just lint` remains
      blocked only when its CI-provided sibling binaries are absent.

## Phase handoff

Phase O is complete on `integrate/phase-o` at `4d37280`. Team-lead has:

- [x] merged O.1 through O.5 into `integrate/phase-o`;
- [x] QA reports for every sprint and routed fix worktree;
- [x] release-candidate commit/version and exact command evidence;
- [x] six-template migration results;
- [x] named downstream inventory source and scan evidence;
- [x] fuzz reports proving the parser oracle ran;
- [x] documented ATM-core consumer contract;
- [x] a decision on when explicit legacy mode may be removed.
