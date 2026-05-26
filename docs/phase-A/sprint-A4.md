---
id: A4
title: Semantic Diagram Spec Contract
status: planned
---

# Sprint A4 — Semantic Diagram Spec Contract

## Goal

Define the long-term semantic source model for state-machine and SQL-query
reports so Mermaid can become one renderer or migration input rather than
remain the long-term source of truth.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](./sprint-A1.md)
- [docs/phase-A/sprint-A3.md](./sprint-A3.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A4.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Deliverables

- one typed semantic spec contract for at least:
  - `state_machine`
  - `sql_query`
- one transitional rule that Mermaid may still be emitted as an output during
  migration but is not the long-term semantic source model
- one validation direction for semantic QA against the typed spec rather than
  only string-comparing rendered Mermaid text
- one explicit extension point so repos can add new report-spec kinds later
  without rewriting the catalog or producer contracts

## Explicit Code Samples

```yaml
kind: state_machine
id: save-message
title: Save Message
states:
  - id: accepted
  - id: validated
transitions:
  - from: accepted
    to: validated
    event: validate_ok
metadata:
  renderer_targets:
    - mermaid
    - html
    - json
```

## This Sprint Does Not Close

- shared panel chrome/copy behavior
- latest/archive output policy
- publish-manifest behavior
- bundled example migration or UI shell design

## Acceptance Criteria

- the plan defines Mermaid as a renderer/output during transition rather than
  the long-term semantic source of truth
- the plan names the required semantic fields for `state_machine` and
  `sql_query` reports
- the plan makes semantic QA possible against structured diagram specs
- the plan preserves room for future report-spec kinds beyond the first two

## Required Validation

- `cargo fmt --all --check`
