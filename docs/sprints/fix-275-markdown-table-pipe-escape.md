---
id: FIX-275
status: complete
branch: fix/275-markdown-table-pipe-escape
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/275-markdown-table-pipe-escape
target: develop
---

# Sprint FIX-275 — Markdown-Table-Safe `|` Escaping Filter

## Problem

Issue #275: a caller-controlled string value interpolated into a Markdown
table cell (`| {{ value }} |`) that itself contains a literal `|` character
corrupts the table's column structure — the embedded pipe is indistinguishable
from a real column delimiter to any Markdown renderer, silently shifting or
splitting the row.

Repro (issue #275, `templates/coverage-report/mac.md.j2` in `atm-core` —
**this template does not exist in this repo**; the underlying engine defect
(`sc-composer` has no markdown-table-safe escaping mechanism at all) is
in-repo and is what this sprint fixes):

```
| Metric | Value |
| --- | --- |
| {{ label }} | {{ value }} |
```

with `{"label": "cache|hit", "value": "12"}` renders a 3-column row
(`| cache | hit | 12 |`) instead of the intended 2 columns — table structure
corruption, not a rendering crash, so it is easy to miss in casual review.

Structurally the same mechanism also reproduces against `win.md.j2`,
`smoke.md.j2`, `smoke-thorough.md.j2`, and `smoke-fast.md.j2` (all in
`atm-core`, all out of repo bounds — see Out of scope).

## Root cause

None of `sc-composer`'s existing minijinja filters (`cdata_escape`,
`turtle_escape`) or the `AutoEscape::Custom("sc-compose-html")` HTML-markup
formatter address Markdown table-cell content at all — HTML escaping targets
`& < > " '`, none of which is the Markdown table delimiter. There is
currently no filter, in `crates/sc-composer/src/renderer.rs` or elsewhere,
that neutralizes a literal `|` for safe placement inside a `| ... | ... |`
table row. This is a missing capability, not a bug in existing logic — the
same "no format-aware escaping exists for this output shape" gap already
identified and fixed per-format by FIX-272 (CDATA/Turtle), FIX-274
(frontmatter delimiters), and FIX-278 (XML control characters).

## Fix design (recommended; comp may adjust within these constraints)

Follow the FIX-272/FIX-274/FIX-278 precedent: a single narrowly-scoped
minijinja filter, registered alongside `cdata_escape_filter` /
`turtle_escape_filter` in `crates/sc-composer/src/renderer.rs` — not a
change to auto-escape dispatch, since Markdown has no `AutoEscape` variant
in `legacy_auto_escape_callback` today and adding one is out of scope.

1. Add `md_table_safe_filter` in `crates/sc-composer/src/renderer.rs`,
   following the plain-`String`-return style of `turtle_escape_filter`
   (no `JinjaValue::from_safe_string` wrapping is needed here — unlike
   `cdata_escape_filter`, the output is plain text, not markup that must
   bypass the HTML formatter):
   - Replace every literal `|` with `\|` (the standard CommonMark/GFM
     table-cell escape for a literal pipe).
   - Also replace embedded newlines (`\n`, `\r`) with a single space —
     a raw newline inside a table cell breaks the row entirely regardless
     of pipe-escaping, and this mirrors the "single output line" cell
     constraint that Markdown table syntax requires. Keep this
     conservative: only `\n`/`\r`, no other whitespace normalization.
   - Leave every other character untouched (do not also HTML-escape;
     Markdown table cells are not HTML output and over-escaping here would
     visibly corrupt plain-text values that happen to contain `<`/`&`).
2. Register the filter: `env.add_filter("md_table_safe", md_table_safe_filter)`
   next to the existing two `add_filter` calls in `configure_environment`.
3. Add a unit test proving both escapes apply together (a value with both
   a `|` and an embedded `\n`) and a unit test proving an ordinary value
   with neither is byte-identical after the filter (no over-escaping).
4. Apply the filter at real point-of-use: none of this repo's own bundled
   `.md.j2` templates currently interpolate raw caller-controlled values
   into a table cell (confirmed by the same template-family audit pattern
   FIX-274 used) — grep every `.md.j2` under `.claude/skills/` and
   `examples/` for a `{{ ... }}` expression inside a line matching
   `^\s*\|.*\|\s*$` and confirm none are found, or apply the filter to any
   that are. Document the audit result (found or not found) in Closeout
   Evidence, same as FIX-274's 8-template audit.

## Required tests (two-commit red green process: commit 1 = all failing, commit 2 = fix)

1. Unit test: `md_table_safe_filter("cache|hit")` returns `"cache\\|hit"`.
2. Unit test: a value containing both `|` and `\n` (e.g. `"a|b\nc"`) has both
   escaped/replaced in one pass (`"a\\|b c"`).
3. Unit test: ordinary text with neither `|` nor a newline is byte-identical
   after the filter (regression guard against over-escaping).
4. Template-level test: render a minimal inline `| {{ v | md_table_safe }} |`
   table with `v = "cache|hit"` and assert the rendered row has exactly the
   expected column count when split naively on unescaped `|` (i.e. the
   escaped pipe is not counted as a delimiter by a naive Markdown-table
   splitter used in the test assertion).
5. CLI-level regression: render a template exercising `md_table_safe`
   through `sc-compose render` end-to-end and assert the literal `\|`
   sequence appears in the output file.
6. Bundled-template audit test/assertion (or documented manual audit,
   consistent with FIX-274's approach) confirming no existing `.md.j2`
   template in this repo interpolates an unescaped caller-controlled value
   into a table cell.

## Out of scope

- `templates/coverage-report/mac.md.j2`, `win.md.j2`, `smoke.md.j2`,
  `smoke-thorough.md.j2`, `smoke-fast.md.j2` and their var-files — do not
  exist in this repo (`atm-core`), out of ATM boundary scope per
  `CLAUDE.md`.
- Adding a Markdown `AutoEscape` variant to `legacy_auto_escape_callback`
  so the filter applies implicitly by template-name suffix — this sprint
  ships an explicit opt-in filter only, matching the issue's proposed fix
  and avoiding an ambiguous "which `.md.j2` output shapes want table
  escaping vs. plain text" auto-detection problem.
- General Markdown escaping beyond table-cell `|`/newline safety (e.g.
  `*`, `_`, `` ` ``, `[`, `]` emphasis/link-syntax escaping) — not named in
  issue #275, and a distinct problem class from column-structure
  corruption.

## Acceptance criteria

- `cargo test --workspace` passes, including all new tests above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #275's exact repro (a `|`-containing value in a table cell) no
  longer corrupts column structure when the value is passed through
  `md_table_safe`.
- No new dependency added to `Cargo.toml`.
- Sprint doc Closeout Evidence section records exact fix commit(s),
  validation results, and the bundled-template audit outcome.

## References

- Issue #275: https://github.com/randlee/sc-compose/issues/275
- `crates/sc-composer/src/renderer.rs` (`cdata_escape_filter`,
  `turtle_escape_filter`, `configure_environment` — precedent from
  FIX-272/FIX-274/FIX-278)
- PR #284 (FIX-272), PR #287 (FIX-274), PR #288 (FIX-278)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

- Red tests committed and pushed at `6e90d1e` (`test: reproduce markdown table
  pipe corruption`). Before the filter was registered, the focused unit and
  CLI tests failed with Minijinja's `unknown filter: md_table_safe` error.
- Green implementation committed and pushed at `f4ad48d` (`fix: add markdown
  table safe filter`), with the clippy cleanup committed and pushed at
  `b5d225c` (`fix: satisfy clippy for markdown table filter`).
- Focused validation passed: `cargo test -p sc-composer md_table_safe --
  --nocapture` (3 passed) and `cargo test -p sc-compose --test cli
  render_markdown_table_safe_cli_regression -- --nocapture` (1 passed).
- Full validation passed: `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check`.
- Bundled-template audit found no `.md.j2` file under `.claude/skills/` or
  `examples/` with a Jinja interpolation on a Markdown table-row line, so no
  in-repository callsite required a change. The five referenced coverage
  templates are outside this repository and were not accessed.
- No dependency was added. The fix remains an explicit opt-in filter; no
  Markdown auto-escape mode was introduced.
