# Issues Inventory

## Status

Active follow-on issue inventory for post-close cleanup and maintainability work
after Phase B, including planned work targeting `develop`.

This inventory records the accepted production-readiness findings that remain
open after Phase B feature completion. It does not reopen Phase B feature
scope; it maps each accepted cleanup issue to the sprint that closes it.

## Phase B Cleanup Findings

| ID | Finding | Status | Closing sprint | Notes |
| --- | --- | --- | --- | --- |
| PHB-CLEANUP-001 | Normative doc/API drift remains between shipped `sc-composer` public APIs and the Phase B source-of-truth docs. | Closed | `B11` | Covers `Renderer::new()`, `validate()`, and renderer-ownership narrative alignment. |
| PHB-CLEANUP-002 | Remaining JSON and JSONL path fields are not all normalized to forward slashes on Windows-sensitive surfaces. | Closed | `B12` | Known opening surface is `templates add --json`; any additional touched validation-related JSON/JSONL emitters must be enumerated in the implementation diff. |
| PHB-CLEANUP-003 | Production observability code still contains runtime-reachable panic paths. | Closed | `B13` | Covers runtime-variable panic removal only; `CliObserver::new()` and `main.rs` stay out of scope per ADR-0001. |
| PHB-INT-003 | `docs/migration-notes.md` still carries the `sc-observability 1.2` cutover section without an explicit owning cleanup sprint after Phase B close. | Closed | `B13` | Keeps the remaining `sc-observability 1.2` migration-note alignment with the observability cleanup line instead of leaving it untracked. |
| PHB-CLEANUP-004 | Oversized CLI files still hide ownership boundaries and slow review on the shipped Phase B branch. | Closed | `B14` | Closes CLI-only extraction debt without changing the command surface. |
| PHB-CLEANUP-005 | Reporting runtime still carries dead seams, duplicated path helpers, and over-exposed constants after the CLI extraction line. | Closed | `B15` | Depends on `B14` so reporting cleanup stays separate from CLI extraction. |

## Phase J Cleanup Findings

| ID | Finding | Status | Closing sprint | Notes |
| --- | --- | --- | --- | --- |
| PHJ-CLEANUP-001 | Repowise issue #212 identifies maintainability hotspots in `crates/sc-compose/src/cli.rs`, `crates/sc-composer/src/validation.rs`, and `crates/sc-composer/src/frontmatter.rs`. | Planned | `J.1`-`J.4` | Phase J performs behavior-preserving structural decomposition with no new public surface; see `docs/phase-J/phase-J-plan.md` and ADR-0014. |

## Phase K Cleanup Findings

| ID | Finding | Status | Closing sprint | Notes |
| --- | --- | --- | --- | --- |
| PHK-CLEANUP-001 | Repowise issue #311 identifies eight behavior-preserving maintainability hotspots for structural decomposition. | Planned | `K.1`-`K.8` | `catalog.rs` and `resolver.rs` remain explicit follow-on candidates; see `docs/phase-K/phase-K-plan.md`, ADR-0015, and the Phase K boundary contract. |

## Inventory Rules

- Every accepted follow-on cleanup issue must appear here with one owning
  sprint.
- A cleanup issue may move from `Planned` to `Closed` only when its owning
  sprint lands and validates on the target implementation branch.
- New cleanup findings discovered during later review must be added here before
  the related sprint closes.
