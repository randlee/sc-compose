---
id: FEAT-HELP-MANUAL-TOPICS-2
title: Help-manual content, group 2 (frontmatter-init/init/examples/templates/reports)
status: complete
branch: feat/help-manual-topics-2
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/help-manual-topics-2
target: feat/help-manual-core
---

## Root Cause

FR-22 (docs/requirements.md, PR #396) requires a manual page per major CLI
feature. FEAT-HELP-MANUAL-CORE (PR #397, branch `feat/help-manual-core`)
builds the `docs/manual/` structure, the `help_topics` registry
(`include_str!`-based, `TOPICS: &[(&str, &str)]`), and the `sc-compose
help <topic>` command — but ships content for only one topic
(`exit-codes`). This sprint fills in five of the remaining topics.

This sprint depends on `feat/help-manual-core` and targets that branch,
not `develop` — the `help_topics` registry it extends does not exist on
`develop` yet. A sibling sprint, FEAT-HELP-MANUAL-TOPICS-1, is running
concurrently on its own branch (also based on `feat/help-manual-core`) for
the other six topics. Do not touch any topic owned by that sprint.

## Scope — topics owned by this sprint

- `frontmatter-init`
- `init`
- `examples`
- `templates`
- `reports`

## Fix Design

1. Before starting, `git fetch origin && git rebase origin/feat/help-manual-core`
   (or merge, if rebase conflicts with in-flight work — use judgment, but
   the goal is: your branch must contain comp's latest pushed commits on
   `feat/help-manual-core` before you write code against `help_topics`).
   If `feat/help-manual-core` has not been pushed yet or the registry
   module/functions described below don't exist yet, wait and re-check
   rather than re-inventing the registry shape yourself.
2. For each topic above, add `docs/manual/<topic>.md` — real prose
   documentation of that CLI subcommand: what it does, its required and
   optional flags/args, one or two runnable examples, and common failure
   modes with the diagnostic code(s) involved (check the corresponding
   command's existing `--help` text and `crates/sc-compose/src/cli/schema.rs`
   Args struct for accuracy — do not invent flags). Match the tone and
   depth of `docs/manual/exit-codes.md` (from `feat/help-manual-core`) —
   read it first for the expected format. Note that `reports` may cover a
   small family of subcommands (check `run_reports_command` in
   `commands/dispatch.rs`) — document the whole family in one page unless
   that reads poorly, in which case use judgment and note the split.
3. Add one line per topic to `docs/manual/README.md`'s index (the file
   already exists on `feat/help-manual-core`; you are appending entries,
   not creating the file).
4. Add one entry per topic to the `help_topics::TOPICS` registry array in
   `crates/sc-compose/src/help_topics/mod.rs` (the exact seam described in
   that sprint's doc comment) — each entry is
   `("init", include_str!("../../../../docs/manual/init.md"))` (adjust the
   relative path depth to whatever `feat/help-manual-core` actually used;
   verify with `cargo build`, don't assume the depth from this text).
5. Add a test per topic in the existing help-command test module (added by
   `feat/help-manual-core` — find it via `grep -r "help_topics\|fn.*help" crates/sc-compose/tests/`)
   asserting `sc-compose help <topic>` exits 0 and prints content that
   plausibly matches that command's real behavior (e.g. mentions the
   command's key flag or purpose — not a placeholder assertion).

## Required Changes / Tests

- `docs/manual/frontmatter-init.md`, `init.md`, `examples.md`,
  `templates.md`, `reports.md` (new).
- `docs/manual/README.md`: five new index lines (edit, not create).
- `crates/sc-compose/src/help_topics/mod.rs`: five new `TOPICS` entries
  (edit, not create).
- Tests: `sc-compose help <topic>` for each of the five topics, exit 0,
  content sanity-checked against that command's real flags/purpose.

## Out of Scope

- The `help_topics` registry, `Help` command, CLI wiring, `docs/manual/README.md`
  creation, or the `exit-codes` topic — all owned by `feat/help-manual-core`
  (FEAT-HELP-MANUAL-CORE). If any of these are missing when you start, stop
  and report rather than building them yourself.
- Topics owned by FEAT-HELP-MANUAL-TOPICS-1: `render`, `resolve`,
  `validate`, `verify`, `extract`, `template-init`.
- Any change to `docs/requirements.md` or exit-code values.

## Acceptance Criteria

- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features --
  -D warnings`, and `cargo test --workspace` all pass.
- All five topics render real, accurate content via `sc-compose help <topic>`
  and are listed in `docs/manual/README.md`.
- No file owned by FEAT-HELP-MANUAL-CORE or FEAT-HELP-MANUAL-TOPICS-1 is
  modified beyond the shared append points (`docs/manual/README.md`,
  `help_topics::TOPICS`) called out above.

## References

- FR-22, docs/requirements.md (PR #396)
- FEAT-HELP-MANUAL-CORE, docs/sprints/feat-help-manual-core.md (PR #397)
- Sibling sprint: FEAT-HELP-MANUAL-TOPICS-1 (branch `feat/help-manual-topics-1`)

## Priority

Medium — customer-facing documentation gap, not release-blocking.

## Closeout Evidence

Status: **complete**.

- Core dependency: rebased onto `7cd42a3` and `01b589e` from
  `feat/help-manual-core`.
- Implementation: `feb7ca6` (`feat(help): add workspace and report manuals`).
  The branch adds all five topic pages, their ordered registry entries,
  manual-index links, and one content sanity test per topic.
- The five manuals describe the real command schemas, runnable examples,
  report layout or template-pack requirements where applicable, and common
  diagnostic codes.
- Validation: `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check`: PASS.
