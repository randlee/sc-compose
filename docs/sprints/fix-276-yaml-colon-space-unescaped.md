---
id: FIX-276
status: complete
branch: fix/276-yaml-colon-space-unescaped
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/276-yaml-colon-space-unescaped
target: develop
---

# Sprint FIX-276 — YAML-Safe Scalar Quoting Filter For `key: value` Interpolation

## Problem

Issue #276: templates that interpolate a caller-supplied value directly into
a `key: value` line inside a document that is meant to be parsed as YAML
(either a fenced ` ```yaml ` block or, as this sprint additionally found, a
document's own frontmatter header) break YAML parsing whenever the value
contains a `: ` (colon-space) sequence, because the value is emitted
unquoted.

Repro (issue #276, `docs/templates/architecture-adr.md.j2` in `atm-core` —
**this template does not exist in this repo**):

```
adr_id: {{ adr_id }}
```

with `{"adr_id": "ADR-001: with colon"}` renders `adr_id: ADR-001: with colon`
(unquoted) — `PyYAML safe_load` fails with `mapping values are not allowed
here` at the second colon. Reproduced 3/3 on `architecture-adr.md.j2`, once
on `docs/templates/boundary-record.md.j2` (both out-of-repo, atm-core); the
issue also names `docs/templates/sprint-plan.md.j2` as very likely affected
by the identical pattern but not independently reproduced there.

**In-repo instance found this sprint**: `sprint-plan.md.j2` exists in this
repo at `.claude/skills/codex-orchestration/sprint-plan.md.j2` and its own
frontmatter header uses exactly this unquoted `key: {{ value }}` pattern for
`id`, `title`, `status`, `branch`, `worktree`, `target` (lines ~41-46). A
`title` or `worktree` value containing `: ` breaks this file's own YAML
frontmatter the same way, independent of the FIX-274 `frontmatter_safe`
filter already applied to `title`/`worktree` there — `frontmatter_safe`
only neutralizes standalone `---`/`...` delimiter lines, it does not
YAML-escape colon-space sequences within a value. This is a real, in-repo,
in-scope call site for this sprint's fix (unlike FIX-272/274/278's
engine-only precedent).

## Root cause

No filter in `crates/sc-composer/src/renderer.rs` performs YAML scalar
quoting/escaping. `frontmatter_safe` (FIX-274) is delimiter-injection-scoped
only; `cdata_escape`/`turtle_escape`/`xml_char_safe`/`md_table_safe` all
target different output shapes. There is currently no way for a template
author to safely place an arbitrary caller-controlled string as a YAML
mapping value.

## Fix design (recommended; comp may adjust within these constraints)

Follow the FIX-272/274/278/275 precedent: a single narrowly-scoped minijinja
filter registered alongside the existing filters in
`crates/sc-composer/src/renderer.rs`.

1. Add `yaml_safe_filter` that, given a string value, produces a
   YAML 1.1/1.2-safe **double-quoted scalar**:
   - Always wrap the result in `"` ... `"` (double-quoted style is
     unconditionally safe and simplest to implement correctly, unlike
     conditionally-plain-vs-quoted logic) — return `String`, following the
     `turtle_escape_filter` plain-`String`-return style (no
     `from_safe_string` needed; this is plain text output, not markup).
   - Escape backslash (`\` -> `\\`) and double-quote (`"` -> `\"`) first
     (order matters — mirror the FIX-278 two-step-order test pattern), then
     escape control characters relevant to YAML double-quoted scalars:
     `\n` -> `\n` (literal two-char escape), `\t` -> `\t`, `\r` -> `\r`.
   - Do not attempt full YAML 1.1 double-quoted-scalar escape-sequence
     coverage (e.g. `\x`, `\u` numeric escapes for arbitrary control bytes)
     — scope this to the characters that actually appear in caller-supplied
     free text for this repo's templates: backslash, double-quote, and the
     three whitespace control characters above. Note this scoping decision
     explicitly in Closeout Evidence so a future sprint can extend it if a
     real gap is found (matches the FIX-278 precedent of documenting a
     deliberate scope decision with rationale).
2. Register: `env.add_filter("yaml_safe", yaml_safe_filter)`.
3. Apply the filter to `.claude/skills/codex-orchestration/sprint-plan.md.j2`'s
   frontmatter header: `id: {{ id | yaml_safe }}`, `title: {{ title |
   yaml_safe }}` (in addition to, not instead of, the existing
   `frontmatter_safe` — the two filters address different injection shapes
   and both apply to the same value; verify the composition order:
   `frontmatter_safe` first to strip standalone delimiter lines, then
   `yaml_safe` to quote the result, since `yaml_safe`'s quoting would
   otherwise make a standalone `---` line search moot only for that one
   line — comp should write a test proving the composed order is safe
   against both injection shapes simultaneously), `status`, `branch`,
   `worktree`, `target`. Since `yaml_safe` always double-quotes, drop the
   conditional `{% if worktree %}` bare-line formatting only insofar as
   still needed to omit the line entirely when `worktree` is empty (keep
   the existing `{% if worktree %}` guard; just add the filter inside it).
4. Add unit tests proving: colon-space is safe once quoted (no bare `: `
   ambiguity since the whole value is inside the quotes), backslash/quote
   escaping order, and an ordinary value round-trips through
   `yaml.safe_load` unchanged in content.
5. Audit every other bundled `.md.j2`/`.yaml.j2`-shaped template under
   `.claude/skills/` and `examples/` for the same unquoted
   `key: {{ value }}` pattern inside a YAML-parsed region (frontmatter or a
   fenced ` ```yaml ` block) — apply the filter to any found, or document
   "none found" in Closeout Evidence, same audit discipline as FIX-274/275.

## Required tests (two-commit red green process: commit 1 = all failing, commit 2 = fix)

1. Unit test: `yaml_safe_filter("ADR-001: with colon")` returns a
   double-quoted string that `serde_yaml`/round-trip-parses back to the
   exact original value.
2. Unit test: a value containing both `\` and `"` is escaped in the correct
   order (mirrors FIX-278's two-step-order test pattern).
3. Unit test: a value containing embedded `\n` renders as a literal `\n`
   escape sequence inside the quoted scalar, not a raw newline (which would
   break the single-line mapping entry).
4. Real-template regression: render `sprint-plan.md.j2` with a `title`
   containing `: ` and parse the resulting frontmatter block with a YAML
   parser, asserting it parses successfully and `title` round-trips exactly.
5. CLI-level regression: render via `sc-compose render` end-to-end and
   confirm the output frontmatter block is valid YAML via a parse check.
6. Bundled-template audit test/assertion (or documented manual audit)
   confirming coverage of every `key: {{ value }}`-shaped call site found.

## Out of scope

- `docs/templates/architecture-adr.md.j2` and `boundary-record.md.j2` — do
  not exist in this repo (`atm-core`), out of ATM boundary scope per
  `CLAUDE.md`.
- Full YAML 1.1 double-quoted-scalar escape coverage beyond
  backslash/quote/`\n`/`\t`/`\r` (see Fix design step 1's scoping note).
- Switching to YAML block/literal scalar styles (`|`, `>`) or single-quoted
  style — double-quoted is the simplest unconditionally-safe choice and is
  sufficient for this issue's scope.
- Any change to `crates/sc-compose` CLI argument handling.

## Acceptance criteria

- `cargo test --workspace` passes, including all new tests above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #276's exact repro shape (a `: `-containing value interpolated into
  a YAML `key: value` line) parses successfully as YAML once passed through
  `yaml_safe`.
- No new dependency added to `Cargo.toml` unless comp determines a
  hand-written escaper cannot reliably cover the scoped character set above
  — if so, stop and report back before adding one, don't add one silently.
- Sprint doc Closeout Evidence records exact fix commit(s), validation
  results, the bundled-template audit outcome, and the
  `frontmatter_safe` + `yaml_safe` composition-order decision for
  `sprint-plan.md.j2`.

## References

- Issue #276: https://github.com/randlee/sc-compose/issues/276
- `crates/sc-composer/src/renderer.rs` (`cdata_escape_filter`,
  `turtle_escape_filter`, `frontmatter_safe`, `xml_char_safe`,
  `md_table_safe` — precedent from FIX-272/274/278/275)
- `.claude/skills/codex-orchestration/sprint-plan.md.j2` (in-repo call site)
- PR #284 (FIX-272), PR #287 (FIX-274), PR #288 (FIX-278), PR #289 (FIX-275)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

- Red tests committed and pushed at `d6e7684` (`test: reproduce YAML
  colon-space interpolation breakage`). Focused tests failed before the
  filters were registered with Minijinja unknown-filter errors.
- The original green implementation was committed at `a2b672a`, but that
  commit independently duplicated `frontmatter_safe` because the then-local
  develop baseline did not contain FIX-274. The branch was subsequently
  rebased onto current develop, which already contains the canonical
  `frontmatter_safe` implementation from FIX-274; the final branch retains
  only the `yaml_safe` implementation and its call-site changes.
- Focused validation passed: five `sc-composer` YAML-safety tests, the
  frontmatter/YAML composition-order test, the real sprint-plan regression,
  and the CLI YAML-parse regression.
- Full validation passed: `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check`.
- The sprint-plan template applies `frontmatter_safe` first and
  `yaml_safe` second. The first neutralizes standalone `---`/`...` lines;
  the second quotes the complete scalar and escapes its YAML-sensitive
  characters, so both protections remain effective.
- Bundled-template audit found the sprint-plan frontmatter fields as the
  string-valued call sites requiring this fix. `examples/service-config.yaml.j2`
  also has unquoted `port` and `enabled` interpolations, but those are typed
  numeric/boolean values; applying `yaml_safe` would change their YAML types,
  so they remain intentionally unquoted. No other `.md.j2`/`.yaml.j2`
  YAML-region string call sites were found under `.claude/skills/` or
  `examples/`.
- No dependency was added. Full YAML escape coverage beyond backslash,
  double-quote, `\n`, `\t`, and `\r` remains intentionally out of scope.

### QA-276-002 Simplification Follow-up

- The branch was rebased onto `origin/develop` at `5cfb287`. The rebased
  implementation is `937c443` (`fix: add YAML-safe scalar filter`), with the
  final branch state at `75e51d9` (`style: satisfy YAML assertion lint`).
- The duplicate `frontmatter_safe_filter` definition, registration, and
  sprint-plan dependency were removed during rebase. The final diff contains
  exactly one canonical `frontmatter_safe` definition and registration, both
  inherited from develop, while `yaml_safe` remains the only FIX-276 engine
  addition.
- Validation after the rebase passed: `cargo test --workspace` (0 failures),
  `cargo fmt --all --check`, `cargo clippy --all-targets --all-features --
  -D warnings`, and `git diff --check`.
