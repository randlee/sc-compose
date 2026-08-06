---
id: FIX-244
title: "Jinja whitespace-control markers ({%- -%}) parsed as phantom undeclared variable \"-\""
status: complete
branch: fix/244-whitespace-control-phantom-dash
worktree: ../sc-compose-worktrees/fix/244-whitespace-control-phantom-dash
target: develop
---

## Goal

Fix issue #244: a template using only standard Jinja whitespace-control
syntax (`{%- ... %}`, `{% ... -%}`, `{%- ... -%}`) fails `--strict`
validation with a phantom undeclared variable literally named `-`.

## Hard Dependencies

None. This sprint is independent of FIX-246/FIX-243/FIX-245 (all already
merged to `develop`).

## Root Cause

**Correction (superseding the file path originally stated below in this
doc's first revision):** ground-truth confirmed directly in
`crates/sc-composer/src/discovery.rs` on `develop@8d0ef48` — this is the
real location of the scanner on `develop`. `validation.rs` as a single file
does not exist there (it was split into a `validation/` module directory
at some point after this sprint doc's author last checked out `develop`
into a stale secondary checkout); `discover_tokens`/`collect_identifiers`
live in `discovery.rs`. Thanks to comp for independently catching this
mismatch before making any scope changes.

`discover_tokens_with_delimiters` (discovery.rs:49) walks the template text
for `{{ ... }}` (or custom-delimiter equivalents, since FIX-246) and
`{% ... %}` pairs:

```rust
let after_start = &cursor[start + start_delimiter.len()..];
let end = match delimiter {
    Delimiter::Expression => find_expression_close(after_start, end_delimiter),
    Delimiter::Statement => after_start.find(end_delimiter),
};
let Some(end) = end else { break };
let expression = after_start[..end].trim();
```

(discovery.rs:74-80). It slices the raw content strictly between the two
delimiters and only then trims whitespace. It never strips Jinja's
whitespace-control markers (`-` immediately adjacent to a delimiter, e.g.
`{%-`, `-%}`, `{{-`, `-}}`) before that trim.

For input `{%- if true %}`:
- `after_start` = `"- if true %}..."`
- `expression` = `"- if true".trim()` = `"- if true"`

This is not a `for` loop and not `endfor`, so it falls through to
`collect_identifiers("- if true", ...)` (discovery.rs:208). That function
masks quoted literals, then splits the expression on any character that is
*not* `is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')`
(discovery.rs:242-244). Since `-` is itself in the *allowed* set, splitting
on the space between `-` and `if` yields tokens `["-", "if", "true"]`. `"-"`
is not a keyword, not a bound loop name, not a loop-context name, and
`VariableName::new("-")` succeeds (`types.rs:142-154` — unchanged, still a
single file — accepts any non-empty string composed of `[A-Za-z0-9_.-]`,
including a bare `-`) — so `"-"` is inserted into `tokens` as a phantom
referenced variable, which then fails `--strict` as undeclared.

**`-` must stay in `collect_identifiers`'s allowed character set.** The
project's `VariableName` intentionally supports kebab-case variable names
(e.g. `task-id`), and removing `-` from the split allowlist would break
legitimate hyphenated variable discovery — do not do this. The correct fix
is narrower: strip a leading/trailing whitespace-control `-` from the raw
delimiter content *before* it is trimmed and handed to
`parse_for_loop_scope`/`collect_identifiers`, for both `{{ }}` and `{% %}`
pairs.

## Exact Target

`crates/sc-composer/src/discovery.rs::discover_tokens_with_delimiters`
(around discovery.rs:74-80), where `expression` is currently computed as:

```rust
let after_start = &cursor[start + start_delimiter.len()..];
let end = match delimiter {
    Delimiter::Expression => find_expression_close(after_start, end_delimiter),
    Delimiter::Statement => after_start.find(end_delimiter),
};
let Some(end) = end else { break };
let expression = after_start[..end].trim();
```

Change to strip a single leading `-` (if the raw content begins with it,
i.e. it was written as `{%-`/`{{-`) and a single trailing `-` (if the raw
content ends with it, i.e. it was written as `-%}`/`-}}`) from the raw
content **before** trimming:

```rust
let after_start = &cursor[start + start_delimiter.len()..];
let end = match delimiter {
    Delimiter::Expression => find_expression_close(after_start, end_delimiter),
    Delimiter::Statement => after_start.find(end_delimiter),
};
let Some(end) = end else { break };
let raw_content = &after_start[..end];
let without_markers = raw_content
    .strip_prefix('-')
    .unwrap_or(raw_content)
    .strip_suffix('-')
    .unwrap_or_else(|| raw_content.strip_prefix('-').unwrap_or(raw_content));
let expression = without_markers.trim();
```

(Any equivalent implementation is fine as long as it strips at most one
leading and one trailing `-` that is directly adjacent to the delimiter
before trimming — do not use a generic `trim_matches('-')`, which would
also eat legitimate leading/trailing hyphens from something like
`{{ -foo }}` if such a token existed, and would over-strip repeated `-`.)

Apply this to **both** `Delimiter::Expression` (`{{ }}`) and
`Delimiter::Statement` (`{% %}`) cases — the current code computes
`expression` once, before the `match delimiter { ... }` branch, so a single
fix at that shared computation site covers both delimiter kinds.

## This Sprint Does NOT Change

- `collect_identifiers`'s allowed character set (`-` stays allowed) — do
  not touch discovery.rs:242-244.
- `VariableName::new`'s accepted character set in `types.rs` — do not
  touch.
- `mask_quoted_literals` / `is_loop_context_name` behavior — unrelated to
  this bug, do not touch.
- Arithmetic-minus token behavior (e.g. `{{ x - y }}` producing a phantom
  `"-"` token) is a **pre-existing, separate** issue, out of scope for
  FIX-244. Do not attempt to fix it here; do not regress it either (it
  should behave exactly as it does today, since neither operand is
  directly adjacent to a delimiter).

## Explicit Repro (must behave as described post-fix)

```
{%- if true %}Hi{% endif %}
```

Pre-fix: `sc-compose render --file t.j2 --strict --json --root <ROOT>`
exits 2 with `ERR_VAL_UNDECLARED_TOKEN: "undeclared referenced token: -"`.

Post-fix: the same command must exit 0 and render `Hi`.

Also verify these variants all resolve cleanly post-fix:
- `{% if true -%}Hi{% endif %}` (trailing marker only)
- `{%- if true -%}Hi{%- endif -%}` (markers on every tag)
- `{{- name -}}` with `name` declared (expression delimiter markers)

And verify this is unaffected (still legitimate, still discovered):
- `{{ task-id }}` with `task-id` declared as a kebab-case variable — must
  still be discovered as `task-id`, not broken by the marker-stripping
  logic (there is no `-` directly adjacent to `{{`/`}}` here, so nothing
  should be stripped).

## Required Test Matrix

Add unit tests directly in `crates/sc-composer/src/discovery.rs`'s
existing `#[cfg(test)] mod tests` block (co-located with the existing
`discover_tokens`/`discover_tokens_with_delimiters` tests), covering:

(a) `{%- if true %}` — leading statement marker does not produce a
    phantom `-` token.
(b) `{% if true -%}` — trailing statement marker does not produce a
    phantom `-` token.
(c) `{%- if true -%}` — both markers together on one tag.
(d) `{{- name -}}` — expression-delimiter markers stripped correctly,
    `name` still discovered.
(e) `{{ task-id }}` — kebab-case variable name still discovered intact as
    `task-id` (regression guard: marker-stripping must not eat legitimate
    hyphens that aren't adjacent to a delimiter).

## Mandatory Process (standing requirement, confirmed working on FIX-245)

Per quality-mgr's explicit recommendation after FIX-245's unanimous 5/5
QA PASS (0 findings, after two consecutive sprints — SC-QA-255-001,
SC-QA-256-001 — flagged fabricated red→green provenance narratives), this
exact two-commit process is now **mandatory** for this sprint:

1. **Commit 1**: land ONLY the new, currently-failing regression test —
   `crates/sc-compose/tests/fuzz_regressions.rs::whitespace_control_tag_markers_do_not_produce_a_phantom_dash_variable_under_strict`
   (the ignored test named in issue #244; it does not currently exist on
   `develop` — the earlier fuzz-sweep worktree's version was untracked and
   never committed, so this is a fresh addition, not a promotion). Mark it
   `#[ignore = "FIX-244 red test; enable with the whitespace-control-marker fix"]`.
   The test must assert the exact repro above: input
   `"{%- if true %}Hi{% endif %}"` (or equivalent) rendered with
   `--strict` currently fails with `ERR_VAL_UNDECLARED_TOKEN` referencing
   `-`.
2. Team-lead independently confirms commit 1 genuinely fails
   (`cargo test --workspace -- --ignored <test_name>` against that exact
   commit) before dev proceeds.
3. **Commit 2**: the actual fix in `discovery.rs` (marker-stripping) plus
   removing the single `#[ignore]` line from the test added in commit 1.
   No other test-logic changes in this commit — the test body itself must
   not change between commit 1 and commit 2, only the `#[ignore]`
   attribute is removed.
4. The sprint-doc closeout narrative must describe this honestly as a
   same-branch red→green trail (test created fresh on this branch, not
   promoted from any prior branch/worktree).

## Deliverables

- Fix in `discovery.rs::discover_tokens_with_delimiters` as described above
  (marker stripping only; `collect_identifiers` and `VariableName`
  untouched).
- New regression test in `fuzz_regressions.rs` (per the mandatory two-commit
  process above), passing.
- Unit tests (a)-(e) in `discovery.rs`, passing.
- No regression to any existing `discover_tokens`/`collect_identifiers`
  test in `discovery.rs`.
- Sprint doc closeout narrative appended with accurate, verifiable
  provenance (no fabricated "promoted from" language).

## Acceptance Criteria

- `cargo test --workspace` passes, including the new regression test
  (no longer ignored) and unit tests (a)-(e).
- The explicit repro and both stated variants behave exactly as described
  post-fix.
- The kebab-case regression guard (e) passes, confirming no collateral
  damage to legitimate hyphenated variable names.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #244 can be closed referencing the merged PR.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Manual repro check: `sc-compose render --file <repro>.j2 --strict --json`
  exits 0 pre-merge, matching the sprint doc's stated post-fix behavior.

## Closeout Evidence

This branch contains the complete red-to-green trail for the fresh regression
test; it was created on this branch and was not promoted from another
worktree.

- `ca4b735` — added the ignored regression test. Running it with `--ignored`
  against the pre-fix code failed because the strict render did not succeed
  due to the phantom delimiter dash. Team-lead independently re-ran the
  workspace ignored-test command and confirmed the same failure.
- `ac7c139` — stripped one delimiter-adjacent leading/trailing marker in the
  shared discovery path, added the five unit tests, and removed only the
  regression test's `#[ignore]` attribute. The regression then passed.
- Discovery unit matrix: PASS (5/5).
- Fuzz regression suite: PASS (5/5).
- Workspace tests: PASS (`cargo test --workspace`).
- Clippy: PASS (`cargo clippy --all-targets --all-features -- -D warnings`).
- Formatting and whitespace checks: PASS (`cargo fmt --all --check` and
  `git diff --check`).
- Manual strict-render matrix: PASS for leading, trailing, and combined
  statement markers plus expression markers; the `task-id` kebab-case guard
  remains covered by the discovery unit test.

## References

- GitHub issue #244
- `crates/sc-composer/src/discovery.rs` (`discover_tokens_with_delimiters`,
  `collect_identifiers`, ~lines 49-260)
- `crates/sc-composer/src/types.rs` (`VariableName::new`, lines 142-154)
- `crates/sc-compose/tests/fuzz_regressions.rs`
