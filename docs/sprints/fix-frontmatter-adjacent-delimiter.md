---
id: FIX-238
title: Frontmatter parser fails on adjacent `---` delimiters + Jinja tags
status: complete
branch: fix/frontmatter-parser-adjacent-delimiter
worktree: ../sc-compose-worktrees/fix/frontmatter-parser-adjacent-delimiter
target: develop
---

# Sprint FIX-238 — Frontmatter parser fails on adjacent `---` delimiters + Jinja tags

## Goal

- Fix the 1.3.0 regression tracked in GitHub issue #238: `sc-compose validate`/`render`
  fails with `ERR_CONFIG_PARSE: failed to parse YAML frontmatter` on templates where
  the config-frontmatter block's closing `---` is immediately followed by another
  `---` (an output-document frontmatter block with zero content between the two
  delimiters), and the span before the *next* `---` contains any Jinja control tag
  (`{% if %}`, `{% for %}`, etc).
- This is not a hypothetical case — it currently breaks this repo's own
  `.claude/skills/codex-orchestration/sprint-plan.md.j2`, which both `validate`
  and `render` fail on today.

## Hard Dependencies

- `develop` branch at HEAD (no blocked sprints)

## Exact Targets

- `crates/sc-composer/src/frontmatter/parser.rs`
- `crates/sc-composer/src/frontmatter/mod.rs` (if delimiter-scanning boundaries live here instead)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- Root cause fixed: a `---` line immediately following the config-frontmatter
  block's closing `---` must NOT be treated as opening a second YAML
  frontmatter block. It is ordinary template body content once the first
  (and only) config-frontmatter block has closed.
- `sc-compose validate --file .claude/skills/codex-orchestration/sprint-plan.md.j2`
  passes (only expected `ERR_VAL_MISSING_REQUIRED`/`INFO_VAL_DEFAULT_USED`
  diagnostics for unset variables — no `ERR_CONFIG_PARSE`).
- `sc-compose render --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --var-file <vars>`
  succeeds and produces correct output, including the second (output-document)
  frontmatter block and its `{% if worktree %}...{% endif %}` conditional
  rendering correctly with `worktree` both set and unset.
- Regression test(s) added covering: (a) single frontmatter block + Jinja tags
  in body (must keep working), (b) adjacent double `---` with no Jinja tags
  between them (must keep working), (c) adjacent double `---` WITH a Jinja
  tag between them (the bug — must now succeed instead of `ERR_CONFIG_PARSE`).
- No change to the documented behavior of a single frontmatter block, or to
  templates that don't use the adjacent-`---` pattern (verify by re-running
  the existing `frontmatter` module test suite and the CLI integration tests
  clean).

## Required Work

- Read the isolation notes below and reproduce the bug locally first,
  confirming the exact parser code path that misinterprets the second `---`
  as opening a new YAML frontmatter block.
- Fix the delimiter-scanning logic so only the single config-frontmatter
  block (the first `---...---` pair) is ever YAML-parsed; everything after
  its closing `---` is template body, regardless of whether it happens to
  start with another `---`.
- Add unit tests directly in `crates/sc-composer/src/frontmatter/` covering
  the three isolation cases above.
- Add or extend a CLI integration test (`crates/sc-compose/tests/cli.rs` or
  `json_cli.rs`) that validates/renders a template with the adjacent-`---` +
  Jinja-tag pattern end-to-end.
- Confirm `.claude/skills/codex-orchestration/sprint-plan.md.j2` validates
  and renders cleanly after the fix (this is a real repo file, not synthetic
  — do not just fix the synthetic repro and stop).

## Explicit Code Samples

Minimal repro that must go from failing to passing:

```
---
name: t
required_variables:
  - id
---
---
id: {{ id }}
{% if worktree %}worktree: {{ worktree }}
{% endif %}target: x
---
body
```

Before fix: `sc-compose validate --file repro.md.j2 --json` returns
`ERR_CONFIG_PARSE: failed to parse YAML frontmatter` (YAML parser chokes on
`{` in `{% if worktree %}` — confirmed via
`sc-compose render --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --var-file <minimal-vars>`
erroring with: `found character that cannot start any token at line 5 column 2,
while scanning for the next token`, where "line 5" is relative to what the
parser incorrectly treats as a second frontmatter block).

After fix: same file validates/renders with no `ERR_CONFIG_PARSE`; only
diagnostics should be the expected required/default-variable ones.

## This Sprint Does Not Close

- No change to the single-frontmatter-block parsing behavior (must remain
  byte-for-byte compatible for the common case)
- No change to the `winget` publish gap (tracked separately, deferred)
- No broader frontmatter schema or `defaults`/`required_variables` semantic
  changes — this is strictly a delimiter-scanning bug fix

## Acceptance Criteria

- `cargo test --workspace` passes, including new regression tests for this bug
- `sc-compose validate --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --json`
  produces zero `ERR_CONFIG_PARSE` diagnostics
- `sc-compose render --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --var-file <vars-with-worktree-set>` and
  `--var-file <vars-with-worktree-unset>` both succeed and the `{% if worktree %}`
  conditional renders correctly in each case
- Existing frontmatter/CLI test suites remain green (no regression to
  single-block templates)
- GitHub issue #238 can be closed referencing the merged PR

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`

## Closeout Evidence

- Implementation commit: `226ebbc` on
  `fix/frontmatter-parser-adjacent-delimiter`.
- `cargo test --workspace`: PASS.
- `cargo clippy --all-targets --all-features -- -D warnings`: PASS.
- `cargo fmt --all --check`: PASS.
- `git diff --check`: PASS.
- The canonical sprint-plan template validates without `ERR_CONFIG_PARSE`;
  its expected missing-variable diagnostics remain when no input file is
  supplied.
- The canonical sprint-plan template renders successfully with `worktree`
  set and unset, and the conditional output is correct in both cases.

## Addendum: Patch Version Bump (1.3.0 -> 1.3.1)

Added after initial closeout, still on this same branch/worktree/PR (#239).
This is a patch-level bump since FIX-238 is a bug fix with no API/behavior
change beyond the corrected parsing.

- Bump `version` in the workspace `Cargo.toml` (`[workspace.package]`) from
  `1.3.0` to `1.3.1`.
- Update `README.md` so it stays in sync with the new version: the
  `sc-composer` dependency example, the Status table `Version` row, and the
  Status table `Stability` row (minor-version form, e.g. `1.3`).
- Add a `CHANGELOG.md` entry for `1.3.1` describing the frontmatter parser
  fix (issue #238).
- Run `python3 scripts/release_artifacts.py verify-readme-version
  --workspace-toml Cargo.toml --readme README.md` (or `scripts/release_gate.sh`)
  to confirm README/version sync passes before pushing.
- Commit and push to `fix/frontmatter-parser-adjacent-delimiter`; this lands
  in the same PR #239, no new branch/worktree needed.

### Addendum Closeout Evidence

- Workspace and package version metadata are aligned at `1.3.1`.
- README dependency, Version, and Stability references are aligned with the
  `1.3.1` patch release.
- CHANGELOG contains the `1.3.1` FIX-238 entry.
