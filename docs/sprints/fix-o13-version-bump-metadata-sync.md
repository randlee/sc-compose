---
id: FIX-O13
sprint: Phase O (ending) - version bump follow-up
status: assigned
---

# FIX-O13: Sync README/CHANGELOG/project-plan for the 1.4.1 waiver

## Context

PR #438 bumps all workspace/package manifests to 1.4.1 and records an explicit,
user-authorized release waiver in `docs/phase-O/evidence/o5-release-corpus.md`
(commit `14f6a3b`) for the O.5 release-corpus gate. QA-438PR-RECHECK-RELEASE-WAIVER
confirmed the waiver itself is genuinely recorded, but found 4 remaining
metadata-sync defects the version bump left behind.

## Findings

### 1. Blocking (ATM-QA-004) — README.md still shows 1.4.0

`README.md:159` (`sc-composer = "1.4.0"`) and `README.md:171` (`| Version | 1.4.0 |`)
were never touched by this branch. This repo has an established practice of
syncing README on every version bump (see commit `d5b0367`).

**Fix**: update both occurrences to `1.4.1`.

### 2. Important (ATM-QA-002, reopened) — CHANGELOG still entirely `[Unreleased]`, contradicts the waiver

`CHANGELOG.md` has no dated `## [1.4.1]` section. `CHANGELOG.md:41` literally
says "the current 1.4.1 recommendation is conditional pending migration,"
which directly contradicts the new waiver recorded in `o5-release-corpus.md`.

**Fix**: promote the `## [Unreleased]` section's Phase O content into a dated
`## [1.4.1] - 2026-08-13` section. Reword the stale "conditional pending
migration" sentence to reflect the recorded waiver — e.g., note that 1.4.1
ships now under the documented waiver (see `o5-release-corpus.md`'s "Release
waiver" section), with legacy-mode fallback and diagnostics
(`WARN_JSON_LEGACY_ESCAPE_MODE`, `ERR_JSON_MODE_INCLUDE_CONFLICT`) covering
downstream consumers until they migrate. Do not claim the migration itself is
complete — only that the release proceeds under a recorded waiver.

### 3. Important (ATM-QA-005) — no Follow-on Fix Sprint entry for O.10

Every other fix branch (FIX-390, FIX-373, FIX-434, FIX-O9) has a corresponding
entry in `docs/project-plan.md`'s Follow-on Fix Sprint list. O.10 (this version
bump) has none.

**Fix**: add a `FIX-O13` (or `O.10`, matching whichever ID convention
`project-plan.md` uses for this list — check the existing entries) row/entry
following the same format as the other fix-sprint entries, linking to this
sprint doc.

### 4. Important (ATM-QA-006) — stale Phase O "planned" status blurb

`docs/project-plan.md`'s "### Phase O Sprint Plans" Status block (~line 843)
still says "planned" while the same document already records FIX-434 (and by
now FIX-O9, O.10, O.11/O.12) as complete a few lines below — an internal
inconsistency, pre-existing but surfaced by this recheck.

**Fix**: update the Status block to reflect that Phase O's core implementation
and the fix-sprint sequence are complete, while the O.5 cross-repository
migration disposition remains open per the waiver recorded in
`o5-release-corpus.md`. Do not overstate this as "fully closed" — the
migration tracking itself is intentionally still open.

## Out of scope

- Do not re-open or alter the substance of the release waiver itself.
- Do not touch `phase-O-plan.md`'s acceptance checklist checkboxes — those
  intentionally remain unchecked pending the actual downstream migration.
- Do not touch `tests/fixtures/sc-lint/bootstrap/Cargo.lock` (unrelated local
  artifact drift).

## Acceptance criteria

- `README.md` shows `1.4.1` in both locations.
- `CHANGELOG.md` has a dated `## [1.4.1]` section containing the Phase O
  content, with the stale "conditional" sentence corrected to reference the
  recorded waiver.
- `docs/project-plan.md` has a Follow-on Fix Sprint entry for this work and an
  updated (not overstated) Phase O status blurb.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D
  warnings`, `cargo test --workspace`, and `git diff --check` all clean.
- No unrelated files touched.
