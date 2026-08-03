---
id: sprint-I.1
title: Contract, Raw-Text Semantics, and Traceability
phase: I
status: planned
branch: plan/phase-i-1-contract-and-traceability
worktree: ../sc-compose-worktrees/plan/phase-i-1-contract-and-traceability
target: develop
---

# Sprint I.1 — Contract, Raw-Text Semantics, and Traceability

## Purpose

Freeze the product and architecture contract before any runtime change. This
is a planning/design sprint: it creates no executable implementation and must
not claim that raw text, XML mixed content, dirty-prefix recovery, loop
built-ins, or YAML merge-key support already exists.

## Inputs and dependencies

- Phase G FR-16 and ADR-0011;
- Phase H FR-16 boundary, ADR-0012, shared `match_raw_text` design, and H.8
  closure evidence;
- current XML matcher in `crates/sc-composer/src/extract/xml.rs`;
- current token discovery in `crates/sc-composer/src/validation.rs`;
- current var-file decoder in `crates/sc-compose/src/var_file.rs` and YAML
  conversion in `crates/sc-composer/src/types.rs`;
- GitHub issues #193, #167, and #166, including their reproductions.

No implementation sprint may start until the I.1 contract review accepts the
decisions below. I.4, I.5, and I.6 remain independent of I.2 after this gate.

## Decisions to record

### Raw-text mode

Define one format-neutral known-template operation over in-memory text. The
public selector is `format="raw"` for Rust, CLI, and Python, with a stable
report shape shared with the existing extraction formats. It:

- matches literal template text and `{{ variable }}` segments using the H
  shared matcher;
- supports Markdown and arbitrary text documents without a structural parser;
- retains occurrence order, variable names, rendered values, and confidence;
- rejects unsupported statements, ambiguous adjacent captures, and malformed
  template delimiters with stable diagnostics;
- never identifies an unknown template, executes Jinja, or infers source types.

The contract must state how line/column evidence is represented when there is
no structural path and how `include`/`exclude` filters behave.

### XML block and mixed content

Define the exact known-template subset for I.3: a full element-content
placeholder may capture rendered text and a deterministic serialization of
allowed child markup. Multiple variables, dynamic element names, control-flow
reconstruction, and arbitrary malformed XML repair remain unsupported. The
contract must state whether matching uses canonical child serialization or a
text-only projection, and must provide an example for description,
references, and workflow blocks from #193.

### Dirty-prefix recovery

Define the accepted rendered-only preamble, including plain text and permitted
XML declaration/comment processing instructions. Define rejection for a
malformed suffix, multiple roots, a second document, or a prefix containing
ambiguous markup. Decide whether recovery emits a new warning (recommended:
`WARN_EXTRACT_DIRTY_PREFIX_STRIPPED`) and require the recovery detail to
identify that a prefix was removed.

### Loop context

Define the implicit names available only while an active Jinja `for` scope is
being scanned: `loop`, `loop.index`, `loop.index0`, `loop.revindex`,
`loop.revindex0`, `loop.first`, `loop.last`, `loop.length`, `loop.depth`,
`loop.depth0`, and the supported `loop.cycle` call form. The contract must
state how nested scopes and a caller variable named `loop` outside a loop are
handled. Do not make all dotted names implicit.

### YAML merge keys

Choose and document one policy for `<<` in JSON/YAML var-files. The default
recommendation is fail-closed rejection with `ERR_CONFIG_VARFILE` and an
actionable message because the current decoder cannot promise YAML merge
semantics and silently drops inherited fields. A fully specified bounded
expansion is acceptable only if duplicate-key precedence, nested aliases,
cycles, limits, and cross-surface behavior are specified and tested.

## Required documentation changes

- Amend `docs/requirements.md` with new FR numbers for raw text, XML
  block/prefix behavior, loop built-ins, and the var-file merge-key policy.
- Amend `docs/architecture.md` with ownership, report/path semantics, the
  raw-text seam, prefix normalization boundary, validation scope model, and
  var-file policy.
- Create ADR-0013 for the Phase-I extraction and input-safety decisions, or
  explicitly extend ADR-0012 if architecture QA confirms the decision set is
  still one coherent reversal of the Phase-H boundary.
- Add any new stable diagnostic to `docs/error-code-registry.md`, including
  trigger, recovery, owner, and serialized shape.
- Update `docs/project-plan.md`, the Phase I plan, and the Phase H future-work
  references so no document claims these features are still unplanned once
  the contract is accepted.

## Acceptance criteria

- The four issue gaps map one-to-one to I.3-I.6, and the extra raw-text mode
  is explicitly labeled product-directed scope.
- Rust, CLI, and Python ownership is unambiguous; no adapter owns a second
  matcher.
- Every accepted case has a positive example and every rejection boundary has
  a diagnostic and recovery expectation.
- The plan states that independent sprint QA may run in parallel and contains
  no undocumented sequential QA gate.
- Requirements, architecture, ADR, registry, project plan, and sprint docs
  use the same names, selectors, codes, and scope limits.
- `git diff --check` passes and the documentation review can be performed
  without reading implementation patches.

## Removal path

If the contract is rejected, remove this sprint's proposed FR/ADR/registry
amendments and keep the Phase-H future-work boundary authoritative. No runtime
artifact is created by this sprint.

## Explicit non-goals

- writing extraction or validation code;
- implementing YAML merge expansion without the contract decision;
- changing the Phase-H shipped behavior before a later implementation sprint;
- unknown-template identification, arbitrary Jinja execution, or source-type
  reconstruction.
