---
id: FEAT-HELP-MANUAL-CORE
title: Core scaffolding for sc-compose help manuals (FR-22)
status: complete
branch: feat/help-manual-core
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/help-manual-core
target: develop
---

## Root Cause

FR-22 (docs/requirements.md, plan PR #396) specifies a `sc-compose help
[topic]` conceptual-help subsystem, modeled on ATM's `atm help <topic>`
pattern: static in-binary manual pages per CLI feature. This was motivated
by FIX-390 shipping a real customer-facing exit-code contract change
(FR-7b) with zero discoverability from the installed CLI — no `--help`
mention, no man page, only `docs/requirements.md` in the repository.

This sprint builds only the scaffolding and the `exit-codes` topic (the
original motivating gap). The remaining per-feature topics (render,
resolve, validate, verify, extract, template-init, frontmatter-init, init,
examples, templates, reports) are out of scope here — they will be added
by follow-on sprints once this registry exists, to avoid three developers
touching the same new files at once.

## Fix Design

Manual content lives as real markdown files, not hand-authored Rust string
literals, and is discoverable two ways: (a) by reading the repo/shipped
docs starting from `README.md`, and (b) via `sc-compose help <topic>`,
which embeds the exact same files at compile time. One source of truth,
progressive discovery from `README.md` down to one document per feature.

1. New directory `docs/manual/`:
   - `docs/manual/README.md` — the manual index: one line per topic, each
     linking to that topic's `.md` file. This is the "one document per
     major feature system" index that later sprints extend.
   - `docs/manual/exit-codes.md` — the first topic (the original
     motivating gap from FIX-390): documents the FR-7b contract (0/1/2/3,
     meaning of each, note that `verify` is the only user of exit code 1).
   - Top-level `README.md`: add one bullet to the existing "Documentation"
     section pointing to `docs/manual/README.md` ("CLI feature manuals —
     also available via `sc-compose help <topic>`"). This is the
     progressive-discovery entry point from the top-level README.
2. New module `crates/sc-compose/src/help_topics/` (registered in
   `main.rs`'s module list, alphabetically after `exit_codes`):
   - `mod.rs` exposing:
     - `pub(crate) const TOPICS: &[(&str, &str)]` — an ordered registry of
       `(topic_name, manual_text)` pairs, where each `manual_text` is
       `include_str!("../../../../docs/manual/<topic>.md")` (adjust the
       relative path to the actual crate-root distance — verify with
       `cargo build` rather than trusting this literal path depth). Each
       topic is registered by adding one entry to this array — this is the
       deliberate seam that lets later sprints add topics without editing
       each other's files, beyond this one shared array (and the
       `docs/manual/README.md` index line).
     - `pub(crate) fn topic_names() -> Vec<&'static str>`
     - `pub(crate) fn find(topic: &str) -> Option<&'static str>`
     - `pub(crate) fn index() -> String` — human-readable topic listing,
       used by both `sc-compose help` (no topic) and `sc-compose help
       --list`.
3. `crates/sc-compose/src/cli/schema.rs`:
   - Add `Help(HelpArgs)` to the `Command` enum with about text
     `"Show a feature manual, or list available manual topics"`.
   - `HelpArgs`: optional positional `topic: Option<String>`, plus a
     `--list` boolean flag (`--list` and a positional topic are mutually
     exclusive via `conflicts_with`).
   - Add `#[command(after_help = "Detailed feature manuals ship with this
     CLI — run \`sc-compose help\` (or \`sc-compose help <topic>\`) to read
     them, starting from the exit-code contract.")]` to the root `Cli`
     struct. Wording must affirmatively signal that manuals are bundled
     in the binary (not just "run this command to see more"), since the
     original motivating gap was zero discoverability of shipped docs.
4. `crates/sc-compose/src/commands/dispatch.rs` (or a small new
   `crates/sc-compose/src/commands/help.rs` following the pattern of other
   command modules — developer's choice, follow existing conventions):
   - `Command::Help(args)`:
     - No topic, no `--list`: print `help_topics::index()` to stdout,
       return exit code 0.
     - `--list`: print `help_topics::topic_names()` one per line to
       stdout (stable, scriptable form — no decorative text), return exit
       code 0.
     - Topic given and found: print the topic's manual text to stdout,
       return exit code 0.
     - Topic given and not found: fail closed via
       `CommandError::usage_with_code(anyhow!(...), DiagnosticCode::ErrConfigParse)`
       (exit code 3 per FR-7b/`exit_codes::USAGE_FAIL`), with an error
       message that lists the valid topic names (reuse
       `help_topics::topic_names()`).
   - `help` does not need `observe_command`/observability wiring — it has
     no rendering side effects. Follow whatever the simplest existing
     command (e.g. `observability-health`) does for the minimal case if
     dispatch.rs's structure expects every arm to route through
     `observe_command`; otherwise a direct match arm returning
     `Ok(exit_code)` is fine. Match the surrounding code's existing style.

## Required Changes / Tests

- `docs/manual/README.md` (new): topic index, one line per topic linking to
  that topic's `.md` file. Only `exit-codes` has a real entry this sprint;
  note in the index that more topics are coming so the link chain never
  dead-ends for a reader following it today.
- `docs/manual/exit-codes.md` (new): the exit-codes manual content.
- Top-level `README.md`: one new bullet in the existing "Documentation"
  section pointing to `docs/manual/README.md`, worded to say manuals ship
  with the CLI and are also reachable via `sc-compose help <topic>` — this
  is the progressive-discovery entry point (`README.md` ->
  `docs/manual/README.md` -> per-topic `.md` file -> same content via
  `sc-compose help <topic>`).
- `crates/sc-compose/src/help_topics/mod.rs` (new) — no separate per-topic
  Rust source files; each topic is a `docs/manual/<topic>.md` file pulled
  in via `include_str!`, not a hand-written Rust constant.
- `crates/sc-compose/src/cli/schema.rs`: `Help` command + `HelpArgs` +
  root `after_help`.
- `crates/sc-compose/src/commands/dispatch.rs` (and/or new
  `commands/help.rs`): dispatch wiring.
- `crates/sc-compose/src/main.rs`: register `mod help_topics;`.
- Tests in `crates/sc-compose/tests/cli.rs` (or a new
  `crates/sc-compose/tests/cli/help.rs` if the test suite is already split
  that way — check current layout):
  - `sc-compose help` with no args exits 0 and lists at least `exit-codes`.
  - `sc-compose help --list` exits 0 and prints `exit-codes` on its own
    line.
  - `sc-compose help exit-codes` exits 0 and prints content mentioning all
    four exit codes (0, 1, 2, 3).
  - `sc-compose help not-a-real-topic` exits 3 and stderr/stdout (per
    existing usage-error conventions) names `exit-codes` as a valid topic.
  - `sc-compose --help` output contains the after_help pointer line, and
    the assertion must check for language that affirmatively signals
    shipped/bundled documentation (e.g. "ship" / "manuals"), not merely
    that *some* after-help text is present — a vague pointer line would
    pass a weaker assertion without fixing the discoverability gap.

## Out of Scope

- Per-feature manual content beyond `exit-codes` (render, resolve,
  validate, verify, extract, template-init, frontmatter-init, init,
  examples, templates, reports) — tracked as follow-on sprints
  (FEAT-HELP-MANUAL-TOPICS-1/2/3) dispatched once this lands.
- Any change to `docs/requirements.md` (already landed via PR #396) or to
  the exit-code *values* themselves.
- No man page or installed-docs packaging (no ATM dependency, no build.rs
  changes) — the manuals are `docs/manual/*.md` files embedded verbatim via
  `include_str!` at compile time, not hand-written Rust string constants.

## Acceptance Criteria

- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features --
  -D warnings`, and `cargo test --workspace` all pass.
- All behaviors in "Required Changes / Tests" above are implemented and
  covered by a passing test.
- The `help_topics::TOPICS` registry is documented (a doc comment on the
  const) as the deliberate single-touch-point seam for adding topics, so
  follow-on sprints know where to add their entries.

## References

- FR-22, docs/requirements.md (PR #396)
- FIX-390 (PR #391), FIX-390-FOLLOWUP (PR #393): the motivating exit-code
  contract gap.
- ATM's `atm help <topic>` command (design reference only — no code or
  runtime dependency on ATM; boundary rules in CLAUDE.md still apply).

## Priority

Medium — customer-facing documentation gap, not release-blocking.

## Closeout Evidence

- implementation commit: `7cd42a3` (`feat: add bundled help manual core`)
- `sc-compose help` prints the bundled manual index, `help --list` emits the
  stable topic names, and `help exit-codes` prints the four-code contract;
  unknown topics return usage failure `3` and list valid topics.
- root `sc-compose --help` points users to the shipped manuals.
- CI evidence: [PR #397 checks](https://github.com/randlee/sc-compose/pull/397/checks)
  for commit `01b589e` are green, including the workspace test job.
- validation PASS: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  and `cargo test --workspace`.
