---
id: H.1
title: Reverse Extraction Extension Contract
status: planned
branch: sprint/h-1-reverse-extraction-extension-contract
worktree: ../sc-compose-worktrees/sprint/h-1-reverse-extraction-extension-contract
target: develop
---

# Sprint H.1 — Reverse Extraction Extension Contract

## Goal

- This is a planning/design sprint, not a runtime delivery sprint.
- Convert the five confirmed issue #193 gaps into one accepted, implementable
  Phase-H contract before any code sprint starts.
- Amend FR-16, the extraction architecture, and ADR-0012 without changing
  Phase-G runtime behavior.

## Hard Dependencies

- Phase G.1 through G.7 are complete on the `develop` baseline.
- The read-only issue #193 gap review is available as the source analysis.
- ADR-0011 remains authoritative until this sprint is accepted.

## Exact Targets

- `docs/requirements.md`
- `docs/architecture.md`
- `docs/adrs/0012-phase-h-reverse-extraction-extension-gates.md`
- `docs/phase-H/phase-H-plan.md`
- `docs/project-plan.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H1-D1 — Accept a normative disposition for JSON, YAML, TOML, XML
  mixed-content values, and XML dirty prefixes, including the supported input
  subset and explicit rejection cases for each.
- H1-D2 — Define the generic report/path/source extension strategy and the
  public Rust, Python, and CLI format-selection shape without implementing it.
- H1-D3 — Define the malformed-input, duplicate-key, null/type, ambiguity,
  provenance, size-limit, security, and cross-format recovery-hint policies
  needed by H.2 through H.7, including the current XML-specific constructors
  in `crates/sc-composer/src/extract/error.rs`.
- H1-D4 — Amend H.6 and H.7 Exact Targets to include
  `crates/sc-composer/src/extract/error.rs`, and update FR-16, architecture,
  ADR-0012, the Phase-H plan, and the project index so they agree on
  dependencies, non-goals, and exit gates.

## Required Work

- Review each issue #193 reproduction and map it to a requirement, ADR rule,
  owning module, and planned sprint.
- Decide whether JSON placeholders are restricted to string values or may
  represent complete JSON values, and define object/array occurrence paths.
- Decide the canonical representation and provenance for XML mixed content.
- Specify a bounded XML preamble grammar; do not define generic first-`<`
  recovery.
- Define YAML and TOML parser boundaries, including duplicate and typed-value
  behavior, while retaining rendered-string output.
- Define a format-neutral extraction error taxonomy and recovery-hint policy;
  existing XML-specific recovery text must not be copied into YAML or TOML
  diagnostics without an explicit contract decision.
- Record explicit non-closure for unknown-template identification, loops,
  branches, typed recovery, and Jinja evaluation.

## Explicit Code Samples

The accepted design must make the eventual API shape unambiguous. The sample
is illustrative contract text, not an executable artifact:

```rust
pub enum ExtractFormat {
    Xml,
    Json,
    Yaml,
    Toml,
}

pub struct ExtractRequest<'a> {
    pub template: &'a str,
    pub rendered: &'a str,
    pub format: ExtractFormat,
    pub include: &'a [VariableName],
    pub exclude: &'a [VariableName],
}
```

The sprint must also name the format-specific path/source variants that
instantiate the existing generic `ExtractionReport` without creating a second
report model.

## This Sprint Does Not Close

- No Rust, Python, CLI, parser, or test implementation.
- No change to Phase-G XML scalar behavior or shared delimiter decisions.
- No claim that JSON, YAML, TOML, mixed-content XML, or dirty-prefix input is
  supported before the relevant implementation sprint passes.

## Acceptance Criteria

- FR-16, architecture, ADR-0012, the Phase-H plan, and the project index are
  mutually consistent and link to the same eight contiguous sprints.
- Every issue #193 gap has one disposition, one owner sprint, explicit
  supported/rejected behavior, and a testable contract.
- The accepted contract preserves rendered-string output, ambiguity safety,
  library/adapter boundaries, and fail-closed malformed-input handling.
- H.2 through H.7 can implement their scope without inventing semantics in
  code or reopening H.1 decisions.
- The document explicitly states that this sprint produces no executable
  artifact.

## Required Validation

- `git diff --check`
- `rg -n "H\.[1-8]|ADR-0012|Phase-H" docs/requirements.md docs/architecture.md docs/adrs/0012-phase-h-reverse-extraction-extension-gates.md docs/phase-H docs/project-plan.md`
