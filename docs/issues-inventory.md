# Issues Inventory

## Status

Active follow-on issue inventory for post-close cleanup on `integrate/phase-B`.

This inventory records the accepted production-readiness findings that remain
open after Phase B feature completion. It does not reopen Phase B feature
scope; it maps each accepted cleanup issue to the sprint that closes it.

## Phase B Cleanup Findings

| ID | Finding | Status | Closing sprint | Notes |
| --- | --- | --- | --- | --- |
| PHB-CLEANUP-001 | Normative doc/API drift remains between shipped `sc-composer` public APIs and the Phase B source-of-truth docs. | Planned | `B11` | Covers `Renderer::new()`, `validate()`, and renderer-ownership narrative alignment. |
| PHB-CLEANUP-002 | Remaining JSON and JSONL path fields are not all normalized to forward slashes on Windows-sensitive surfaces. | Planned | `B12` | Known opening surface is `templates add --json`; any additional touched validation-related JSON/JSONL emitters must be enumerated in the implementation diff. |
| PHB-CLEANUP-003 | Production observability code still contains runtime-reachable panic paths. | Planned | `B13` | Covers runtime-variable panic removal only; `CliObserver::new()` and `main.rs` stay out of scope per ADR-0001. |
| PHB-CLEANUP-004 | Oversized CLI files still hide ownership boundaries and slow review on the shipped Phase B branch. | Planned | `B14` | Closes CLI-only extraction debt without changing the command surface. |
| PHB-CLEANUP-005 | Reporting runtime still carries dead seams, duplicated path helpers, and over-exposed constants after the CLI extraction line. | Planned | `B15` | Depends on `B14` so reporting cleanup stays separate from CLI extraction. |

## Inventory Rules

- Every accepted follow-on cleanup issue must appear here with one owning
  sprint.
- A cleanup issue may move from `Planned` to `Closed` only when its owning
  sprint lands and validates on the target implementation branch.
- New cleanup findings discovered during later review must be added here before
  the related sprint closes.
