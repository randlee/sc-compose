# Anchors - phase-u

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-u/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | values stay data | 2026-11-03 |

## ANC-001

Anchor: Values containing {{ }}, {{{ }}} or {% %} must stay data and must never be re-parsed.

> Values containing {{ }}, {{{ }}} or {% %} stay data and are never re-parsed.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: values stay data

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: supply values containing each delimiter and read back identical literal text from the text passed to bd
