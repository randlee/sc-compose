---
id: FIX-243
title: Frontmatter parser silently drops adjacent plain-YAML block instead of treating it as body
status: complete
branch: fix/243-frontmatter-silent-data-loss
worktree: ../sc-compose-worktrees/fix/243-frontmatter-silent-data-loss
target: develop
---

# Sprint FIX-243 — Frontmatter parser silently drops adjacent plain-YAML block

## Goal

Fix GitHub issue #243: a `---`-delimited block that is plain YAML (or empty)
and immediately follows the config-frontmatter block is silently consumed
and discarded by the stacked-header loop in `split_frontmatter` — it never
becomes part of the rendered body, and no diagnostic is emitted. This is
silent data loss / corruption of template body content.

## Hard Dependencies

- `develop` branch at HEAD (no blocked sprints)

## Root Cause

`split_frontmatter` in `crates/sc-composer/src/frontmatter/parser.rs`
continues consuming subsequent `---`-delimited blocks as additional
config-frontmatter passes as long as the candidate block does not contain
Jinja syntax (`FIX-238`'s `contains_jinja_syntax` check, PR #239 — a
deliberate but narrowly-scoped fix per its own code comment). There is no
check for whether the block is actually *intended* as a second config pass.

`RawFrontmatter` (`crates/sc-composer/src/frontmatter/model.rs`) has no
`#[serde(deny_unknown_fields)]`, so almost any plain-YAML mapping — including
one with completely unrecognized keys like `a: b` — deserializes
successfully into a mostly-default `RawFrontmatter`. That means ordinary
plain-YAML body content immediately following the config block (e.g. an
example config snippet inside documentation) gets silently swallowed as an
empty/near-empty extra frontmatter pass instead of being preserved as body
text.

## Exact Targets

- `crates/sc-composer/src/frontmatter/parser.rs` (`split_frontmatter`)
- `crates/sc-composer/src/frontmatter/model.rs` (`RawFrontmatter`, if the fix
  needs a "does this look like real frontmatter" check based on which fields
  are present)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint,
the sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- Root cause fixed: the stacked-header loop must only continue consuming a
  subsequent `---`-delimited block as an additional config-frontmatter pass
  when that block is recognizably frontmatter — i.e. it uses only the known
  frontmatter keys (`pass`, `required_variables`, `variables`, `defaults`,
  `input_defaults`, `metadata`). A block containing any unrecognized
  top-level YAML key must NOT be consumed as a frontmatter pass; stacking
  stops (same as the existing Jinja-syntax break) and that block — including
  its own `---` delimiters — becomes part of the rendered body verbatim.
- The existing FIX-238 stacked-header behavior for *genuine* multi-pass
  frontmatter (blocks using only recognized frontmatter keys, no Jinja) must
  keep working unchanged — do not narrow this sprint into breaking
  intentional multi-pass config.
- The existing FIX-238 Jinja-syntax break behavior (an output document's own
  frontmatter containing `{{ }}`/`{% %}`/`{# #}` becomes body, not a config
  pass) must keep working unchanged.
- No diagnostic regression: this remains a silent, correct reclassification
  (block becomes body) — do not add a new error/warning diagnostic for this
  case unless required_variables validation elsewhere would already produce
  one.
- Regression test
  `adjacent_plain_yaml_frontmatter_block_is_not_silently_consumed_as_a_second_pass`
  was authored as a new, unignored test alongside the implementation in
  commit `01a1e5c`; it was not promoted from another worktree or an ignored
  test.
- No change to single-frontmatter-block parsing behavior, and no change to
  the first (always-present) config-frontmatter block's permissive
  unknown-key tolerance — verify by re-running the existing frontmatter
  module test suite and CLI integration tests clean.

## Required Work

- Reproduce the bug locally first using the exact repro below, confirming
  the code path in `split_frontmatter` that misclassifies the plain-YAML
  block as a second frontmatter pass.
- Implement a "is this candidate block recognizable frontmatter" check
  (only known keys present) that gates whether the stacking loop continues,
  without changing how the *first* header is parsed/validated.
- Recreate and un-ignore the regression test listed above in
  `crates/sc-compose/tests/fuzz_regressions.rs` in this worktree (the file
  does not yet exist on `develop`; create it fresh with just this test plus
  a `#[path = "support/mod.rs"] mod support;` import and the existing
  `sc_compose`/`temp_root`/`write_file` helpers used elsewhere in this test
  suite family).
- Add unit tests directly in `crates/sc-composer/src/frontmatter/` (or
  extend existing ones) covering: (a) single frontmatter block unchanged,
  (b) genuine stacked multi-pass frontmatter (only known keys, no Jinja)
  still stacks, (c) Jinja-containing second block still becomes body
  (FIX-238 baseline), (d) plain-YAML-with-unrecognized-keys second block now
  becomes body instead of being silently consumed (this bug).
- Add or extend a CLI integration test exercising the exact repro end-to-end
  via `sc-compose render`.

## Explicit Code Sample

Repro (must go from silently dropping the second block to preserving it):

```
---
{}
---
---
a: b
---
BODY
```

`sc-compose render --file t.j2 --root <ROOT>`

Before fix: renders to just `"BODY\n"` — the entire `---\na: b\n---\n` block
silently vanishes, no diagnostic.

After fix: rendered output must contain the literal text `---` and `a: b` —
the second block remains part of the body, verbatim, including its own
delimiters.

## This Sprint Does Not Close

- No change to the single-frontmatter-block parsing behavior
- No change to genuine multi-pass config-frontmatter stacking semantics
  (blocks using only recognized frontmatter keys)
- No change to the FIX-238 Jinja-syntax break behavior
- No broader frontmatter schema changes beyond the stacking-continuation
  check

## Acceptance Criteria

- `cargo test --workspace` passes, including the new/promoted regression
  test (no longer `#[ignore]`d)
- The explicit code sample above behaves exactly as described after the fix
- Existing frontmatter/CLI test suites remain green (no regression to
  single-block or genuine multi-pass templates)
- GitHub issue #243 can be closed referencing the merged PR

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`

## Closeout Evidence

- Implementation commit: `01a1e5c` (`fix: preserve adjacent plain yaml body blocks`).
- The exact repro now preserves the adjacent `---\na: b\n---\n` block and
  `BODY` in rendered output.
- Frontmatter unit tests: PASS (17/17).
- CLI fuzz regression tests: PASS (3/3, including the two existing FIX-246
  regressions).
- Workspace tests: PASS (`cargo test --workspace`).
- Clippy: PASS (`cargo clippy --all-targets --all-features -- -D warnings`).
- Formatting and whitespace checks: PASS (`cargo fmt --all --check` and
  `git diff --check`).
