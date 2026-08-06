---
id: FIX-274
status: complete
branch: fix/274-spoofed-frontmatter-delimiter
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/274-spoofed-frontmatter-delimiter
target: develop
---

# Sprint FIX-274 — Neutralize Injected Frontmatter Delimiters In Interpolated Values

## Problem

Issue #274: a caller-supplied free-text value interpolated directly inside a
template's *output* YAML-frontmatter-shaped header can contain a literal
bare `---` line (plus arbitrary `key: value` text), spoofing extra
frontmatter delimiters and attacker-controlled keys in the rendered
document. A downstream tool that naively reads "the first frontmatter
block" (splits on the first pair of `---` lines) will attribute the
injected keys to the document itself rather than to interpolated content.

Repro (issue #274, `.claude/skills/codex-orchestration/sprint-plan.md.j2`):

```json
{
  "id": "1.2",
  "title": "Injected frontmatter break\n---\nmalicious: true\n---",
  "branch": "main",
  "target": "develop"
}
```

Rendered output contains 6 `---` lines instead of the expected 2, with
`malicious: true` appearing to sit inside the document's real frontmatter
header. Reproduced 3/3 by the fuzz probe.

This is the same class of defect as issue #272 (format-aware output
escaping) but for a structural markdown/YAML-frontmatter shape rather than
JSON/XML/Turtle: `title: {{ title }}` interpolates the raw value with no
escaping, and the value happens to land immediately inside a hand-authored
`---`-delimited block in the template body.

The issue also names `docs/templates/architecture-adr.md.j2` and
`docs/templates/boundary-record.md.j2` as similarly-shaped templates —
**neither file exists in this repo**; per `CLAUDE.md`'s ATM boundary rules
this repo must not read `ATM_HOME` or assume ATM-specific paths, so those
two templates (if real) live in `atm-core` and are out of scope here, same
precedent as the Turtle-escape filter in FIX-272. Confirm this file
non-existence at sprint start and note it in Closeout Evidence; do not go
looking in ATM_HOME to "find" them.

## Root cause

`sc-composer`'s renderer has no concept of a "frontmatter-shaped output"
format. `configure_environment`/`legacy_auto_escape_callback`
(`crates/sc-composer/src/renderer.rs`) picks an `AutoEscape` mode from the
template's declared `format:` (added in FIX-272: `json` -> `AutoEscape::
Json`, `xml`/`html`/`htm` -> the custom `sc-compose-html` formatter). `
sprint-plan.md.j2` declares `format: markdown`, which maps to no escaping
at all (plain interpolation) — correct for markdown body content, but the
specific `title: {{ title }}` line sits inside a literal `---`-delimited
header block the template author wrote directly into the body, not
something the renderer's format dispatch is aware of.

## Fix design (recommended; comp may adjust within these constraints)

Follow the FIX-272 precedent: a narrowly-scoped minijinja filter, applied
only at the known call site(s), not a blanket format change.

1. Add a new `frontmatter_safe` filter in `crates/sc-composer/src/
   renderer.rs` (alongside `cdata_escape_filter`/`turtle_escape_filter`)
   that neutralizes any line within the value that, standalone, would be
   interpreted as a YAML frontmatter delimiter or a `key: value` pair:
   - Split the value on `\n`.
   - For any line that, after trimming, is exactly `---` (or `...`, YAML's
     alternate document-end marker), prefix it with a zero-width-safe
     escape that breaks the exact-match — e.g. prepend a backslash-escaped
     form (`\-\-\-`) if the surrounding renderer context is markdown, since
     a literal `---` inside inline text does not need special escaping in
     Markdown itself but a *standalone* `---` line does when it opens or
     closes a document; escaping just the line-initial `-` sequence is
     sufficient and matches how Markdown authors already escape unwanted
     horizontal-rule/frontmatter lines by hand.
   - Do **not** attempt full YAML-safety (quoting, key escaping) — this
     sprint only needs to prevent the value from being *indistinguishable*
     from a real delimiter line to a naive "split on first `---` pair"
     downstream parser. Escaping the delimiter line itself is sufficient
     and matches the CDATA-splitting precedent (surgical, not general).
   - Return `JinjaValue::from_safe_string(...)` so the (non-)AutoEscape
     context for markdown doesn't re-mangle it (mirrors the FIX-272
     `cdata_escape_filter` fix — the original bug there was forgetting this
     step; do not repeat it here).
2. Apply the new filter at the exact repro call site: `title: {{ title |
   frontmatter_safe }}` in `.claude/skills/codex-orchestration/sprint-
   plan.md.j2`. Audit the frontmatter *header* block only (the lines
   between the opening and closing `---` of that template's own output
   header, i.e. `id`, `title`, `status`, `branch`, `worktree`, `target`) for
   any other field that interpolates a caller-controlled free-text value
   directly into that header — apply the same filter there too if found
   (`status` and `target` are effectively enum-like/internally-controlled
   today; confirm with a grep of call sites before deciding whether they
   need it — don't apply blindly to every field).
3. Audit these additional in-repo `.md.j2` templates (found via `grep -rl
   '^---$' --include='*.md.j2'` whose rendered output opens with a literal
   `---` line) for the same shape — a free-text, caller-controlled value
   interpolated directly inside the output's own frontmatter-shaped header
   block — and apply `frontmatter_safe` there too if the shape matches:
   - `.claude/skills/quality-management-gh/findings-report.md.j2`
   - `.claude/skills/quality-management-gh/quality-report.md.j2`
   - `.claude/skills/sprint-report/report-detailed.md.j2`
   - `.claude/skills/sprint-report/report.md.j2`
   - `examples/hello.md.j2`
   - `examples/jagged-array-values.md.j2`
   - `examples/changelog-categories.md.j2`
   - `examples/frontmatter-demo.md.j2`

   Do not modify a template's body content unless the vulnerable shape
   (free-text value landing inside the literal output frontmatter header)
   is actually present — several of these only use `---` as a mid-body
   horizontal rule or table separator, which is not in scope.

## Required tests (two-commit red green process: commit 1 = all failing, commit 2 = fix)

1. Real-template round trip (mirrors FIX-272's `real_plan_scope_template_
   round_trips_cdata_payload` pattern): render the actual `sprint-plan.md.j2`
   via `include_str!` with `title` set to the exact issue #274 repro value
   (`"Injected frontmatter break\n---\nmalicious: true\n---"`), then count
   `---` lines in the output and assert exactly 2 (the real opening/closing
   delimiters), with the injected content still present but neutralized
   (not silently dropped) in the rendered body.
2. Unit test for `frontmatter_safe` directly: input containing a
   standalone `---` line is escaped; input containing `---` as part of a
   longer line (e.g. `a---b`) is left untouched (only exact-match delimiter
   lines are in scope, matching the "Exact-Match Delimiter Scanning Across
   Passes" standing decision referenced in the repo's architecture notes).
3. Unit test for the `...` YAML document-end marker, same treatment as
   `---`.
4. Regression guard: a `title` value with no `---`/`...` content renders
   byte-identical to the pre-fix output (no unwanted escaping of ordinary
   text).
5. CLI-level regression: `sc-compose render --file .claude/skills/codex-
   orchestration/sprint-plan.md.j2 --var-file <repro-vars>.json` exits 0
   and the resulting file, when split naively on the first `---`/`---`
   pair, yields a frontmatter block containing only the real `id`/`title`/
   `status`/`branch`/`target` keys — no `malicious` key.
6. For each additional template modified per design step 3, at least one
   targeted regression test confirming the specific vulnerable field is
   now safe, following the same real-template-round-trip pattern (not just
   a unit test on the filter in isolation).

## Out of scope

- Full YAML value quoting/escaping (this sprint only breaks delimiter-line
  exact matches, not general YAML injection into a header value that a
  strict YAML parser would already reject or mis-key on).
- Any template whose `---` usage is a mid-body horizontal rule or table
  separator, not part of the document's own output frontmatter header.
- `docs/templates/architecture-adr.md.j2` / `boundary-record.md.j2` — do
  not exist in this repo; out of bounds per `CLAUDE.md`.
- Changing how `sc-composer` parses a template's *own* declaration
  frontmatter (`crates/sc-composer/src/frontmatter.rs`) — this sprint is
  about caller-controlled values landing in *rendered output* that happens
  to look like frontmatter, not the engine's input-side parser.

## Acceptance criteria

- `cargo test --workspace` passes, including all new regression tests
  listed above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #274's exact repro no longer produces a spoofed second frontmatter
  block when rendered through the real `sprint-plan.md.j2` template.
- Every additional template actually modified per design step 3 has a
  passing real-template regression test; templates found *not* to have the
  vulnerable shape are explicitly listed as audited-and-clean in Closeout
  Evidence (do not silently skip the audit).
- Sprint doc Closeout Evidence section records exact fix commit(s),
  validation results, the audit result for each candidate template in
  design step 3, and any deviation from the recommended design above with
  rationale.

## References

- Issue #274: https://github.com/randlee/sc-compose/issues/274
- `crates/sc-composer/src/renderer.rs` (`cdata_escape_filter`,
  `turtle_escape_filter`, `configure_environment`,
  `legacy_auto_escape_callback` — precedent from FIX-272)
- `.claude/skills/codex-orchestration/sprint-plan.md.j2`
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)
- PR #284 (FIX-272, format-aware escaping precedent):
  https://github.com/randlee/sc-compose/pull/284

## Closeout Evidence

- Red regression tests committed at `39b9c77` (`test: cover frontmatter
  delimiter injection`). Before the fix, the unit tests failed because the
  filter was unknown, and the real-template and CLI tests observed six
  standalone `---` lines instead of the two real header delimiters.
- Implementation committed at `2145245` (`fix: neutralize frontmatter
  delimiter values`). It adds and registers `frontmatter_safe`, which
  preserves ordinary text and mid-line `---`/`...` sequences while replacing
  standalone delimiter lines with `\-\-\-` or `\.\.\.`. The sprint-plan
  output header and repeated Markdown title interpolation both use the filter
  so the complete rendered document does not reintroduce raw delimiter lines.
- The additional-template audit found no matching vulnerable output header:
  `quality-management-gh/findings-report.md.j2`,
  `quality-management-gh/quality-report.md.j2`,
  `sprint-report/report-detailed.md.j2`, `sprint-report/report.md.j2`,
  `examples/hello.md.j2`, `examples/jagged-array-values.md.j2`,
  `examples/changelog-categories.md.j2`, and
  `examples/frontmatter-demo.md.j2` all use their `---` pairs for input
  declaration frontmatter; their rendered bodies do not open a separate
  literal frontmatter header containing interpolated values, so no additional
  template was modified and no per-template regression test was required.
- QA follow-up `FIX274-QA-001` identified `worktree` as a second
  caller-controlled free-text header field. The field now uses
  `frontmatter_safe`, with a real-template regression covering an injected
  delimiter. The output-header audit therefore covers `title` and `worktree`;
  `id`, `branch`, `status`, and `target` remain constrained/internal fields at
  their call sites.
- `docs/templates/architecture-adr.md.j2` and
  `docs/templates/boundary-record.md.j2` are absent from this repository and
  were not searched for outside the repository boundary.
- Validation on `2145245` plus the closeout documentation changes: `cargo test
  --workspace` passed with zero failures; `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` all passed.
- QA follow-up implementation and regression are included in the subsequent
  worktree-field fix commit; its targeted test confirms the same two-real-
  delimiter invariant for caller-controlled `worktree` values.
