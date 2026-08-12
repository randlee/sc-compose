---
phase: O
title: JSON Render Contract and Fail-Closed Output Validation
status: planned
target: integrate/phase-o
planning_branch: plan/json-format-escape-mode
---

# Phase O — JSON Render Contract and Fail-Closed Output Validation

## Phase goal

Make JSON template rendering safe, backwards-compatible, and machine-verifiable
after the 1.4.0 regression in which `AutoEscape::Json` double-quoted existing
manually quoted placeholders. The phase also closes the detection gap: a
malformed JSON body must not be emitted with exit status 0, and ATM-core must be
able to distinguish static validation from a successful render of a specific
context.

This phase is based on the agreed design in
`docs/sprints/plan-json-format-escape-mode.md`. The phase documents the
implementation work as independently reviewable sprints; it does not add
production code in this planning worktree.

The detailed design source remains the linked plan above; this Phase O folder
is the authoritative execution index and sprint decomposition for team
dispatch, dependency tracking, and QA handoff.

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
| `legacy` | existing literal-quoted string placeholders | JSON-escape string contents without adding outer quotes | 1.4.1 compatibility mode with deprecation warning |
| `auto` | bare placeholders | renderer emits complete JSON values, including string quotes | recommended for new/migrated templates |

Effective mode precedence:

1. `--json-escape-mode` CLI override;
2. `json_escape_mode` in root template frontmatter;
3. 1.4.1 compatibility default `legacy` for unannotated existing JSON
   templates.

`legacy` must never mean raw interpolation. It must safely escape quotes,
backslashes, control characters, and JSON string content. A future breaking
release may change the absent-mode default to `auto` after the migration and
deprecation window.

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
| O-R7 | The six known repository templates are migrated or explicitly declared legacy and tested semantically. | O.4 |
| O-R8 | ATM-core has an actionable integration contract and must not use exit status alone. | O.2, O.4 |
| O-R9 | Fuzz campaigns test both interpolation shapes and parse every successful JSON body. | O.4 |
| O-R10 | No JSON escaping regression reopens the FIX-272 injection vulnerability. | O.1, O.2, O.4 |

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

## ATM-core contract

The library/CLI result must expose at least:

```json
{
  "template": "path/to/assignment.json.j2",
  "output_format": "json",
  "json_escape_mode": "auto",
  "template_contract_valid": true,
  "render_checked": true,
  "render_valid_for_context": true,
  "diagnostics": []
}
```

ATM-core must supply the exact context it will use, call the checked validation
or library API, inspect the structured result, and cache/send only after the
result says `render_valid_for_context: true`. Static validity without a context
must not be represented as proof that a future render will parse. The result
must not include the full prompt body unless explicitly requested.

## Ordering and parallelism

O.1 is the only infrastructure sprint. It defines the mode resolver, shared
types, safe escaping, and core parser-check API.

After O.1 is merged to `integrate/phase-o`, O.2 and O.3 may execute in parallel:

- O.2 owns checked-render behavior, CLI render/validate paths, and the ATM-core
  machine contract.
- O.3 owns source lint, repository target registration, report aggregation,
  and `just lint` wiring.

O.4 depends on the merged behavior from O.2 and O.3. It owns migrations,
release-corpus verification, fuzz-oracle changes, and the 1.4.1 release gate.

O.2 and O.3 must not duplicate the shared parser or mode implementation. O.4
must not begin release-corpus claims until both parent contracts are available.

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
| O.4 | Six-template migration, release corpus, fuzz oracle, 1.4.1 gate | O.2 and O.3 merged | final closure |

## Phase acceptance

- [ ] O.1–O.4 have QA-approved merge evidence.
- [ ] 1.4.1 preserves safe legacy compatibility without raw interpolation.
- [ ] auto mode remains injection-safe for hostile strings and structured
      values.
- [ ] malformed JSON cannot be emitted as a successful checked render.
- [ ] `validate` clearly reports static-only status.
- [ ] `validate --lint` reports quoted-placeholder source findings.
- [ ] `sc-compose lint --target template-contracts` and `just lint` run the
      shared check without duplicate implementation.
- [ ] ATM-core integration guidance uses the checked result and exact context.
- [ ] all six known templates are migrated or explicitly legacy and parsed by
      semantic tests.
- [ ] release-candidate fuzz runs both old and new interpolation shapes.
- [ ] `cargo test --workspace`, formatting, clippy, and repository lint pass.

## Phase handoff

Before Phase O is marked complete, team-lead must have:

- merged O.1 through O.4 into `integrate/phase-o`;
- QA reports for every sprint and routed fix worktree;
- release-candidate commit/version and exact command evidence;
- six-template migration results;
- fuzz reports proving the parser oracle ran;
- documented ATM-core consumer contract;
- a decision on when the compatibility default may change to `auto`.
