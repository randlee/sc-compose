---
id: FIX-272
title: "No format-aware escaping for JSON / CDATA output; Turtle escaping unavailable"
status: complete
branch: fix/272-format-aware-escaping
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/272-format-aware-escaping
target: develop
---

# FIX-272: Format-aware escaping for JSON, CDATA, and Turtle output

Issue: https://github.com/randlee/sc-compose/issues/272
Branch: `fix/272-format-aware-escaping`
Base: `develop` @ `3ff9c8d`

## Problem

Fuzz round 2 found three distinct output-format breakage/injection modes,
all with the same root cause: `sc-compose` has no format-aware escaping
except for the HTML/XML auto-escape path fixed in FIX-268.

1. **JSON breakage/injection**: 296/597 fuzz cases against 6 `format: json`
   templates broke JSON structure or injected a spoofed top-level key when a
   variable value contained `"`, `\`, control characters, or crafted content
   (e.g. a `sprint_id` value like `x", "injected": true, "y": "x` becomes a
   real second key in the emitted object).
2. **CDATA breakage**: a literal `]]>` inside a variable value
   (e.g. embedded in `reviewer_findings_json` or `previous_step_json`)
   prematurely closes a `<![CDATA[ ... ]]>` block in
   `.claude/skills/plan-hardening/{01-plan-scope-review,
   02-sprint-scope-hardening,03-consistency-hardening}.xml.j2`, corrupting
   the surrounding XML.
3. **Turtle breakage** (reported against an atm-core template,
   `triaging-findings/triage-record.ttl.j2`, which does **not exist in this
   repo** — confirmed via `find . -iname '*.ttl.j2'`, empty): unescaped `"`,
   `\`, or newline in a Turtle string literal breaks `rdflib` parsing
   downstream.

## Root cause

`crates/sc-composer/src/renderer.rs`'s `legacy_auto_escape_callback` only
maps `.html`/`.htm`/`.xml` to an escaping `AutoEscape` variant (`Custom
("sc-compose-html")`, since FIX-268); every other extension, including
`.json`, gets `AutoEscape::None` — no escaping at all. There is no
CDATA-aware escaping anywhere (CDATA-splitting is not something an
HTML/XML-style auto-escaper does, and isn't should not be, since CDATA
content is not markup). There is no Turtle-string escaping anywhere.

## Scope decision

Three independent, narrowly-scoped fixes, landed together since they share
a worktree and touch the same file (`renderer.rs`) and its filter-wiring
surface:

1. **JSON**: extend `legacy_auto_escape_callback` to map `.json` →
   `AutoEscape::Json`. minijinja's `AutoEscape::Json` variant already exists
   and is available at zero new-dependency cost — the workspace already
   enables minijinja's `"json"` Cargo feature
   (`crates/sc-composer/Cargo.toml`... actually workspace root `Cargo.toml`
   line ~19: `minijinja = { version = "2.12", features = ["custom_syntax",
   "json"] }`). `format_sc_compose_markup` (added in FIX-268) already
   delegates to minijinja's built-in `escape_formatter` for any
   `AutoEscape` variant other than `Custom("sc-compose-html")`, so
   `AutoEscape::Json` needs no new formatter code — `escape_formatter`
   handles it natively (JSON-string-escapes values including `"`, `\`,
   control characters).
2. **CDATA**: add a new minijinja filter, `cdata_escape`, registered via
   `env.add_filter("cdata_escape", cdata_escape_filter)` in
   `configure_environment` (`crates/sc-composer/src/renderer.rs`). This is
   the first filter registration in this codebase — no prior pattern to
   follow, but minijinja's `Environment::add_filter` takes any
   `Fn(&str) -> String`-shaped function, so a plain free function is
   sufficient (see minijinja docs / existing `format_sc_compose_markup`
   formatter registration as the nearest existing example of wiring a
   function into the `Environment`). Apply the filter **explicitly** in the
   3 affected templates (`{{ reviewer_findings_json | cdata_escape }}`,
   `{{ previous_step_json | cdata_escape }}`) rather than through
   auto-escape, since CDATA-escaping only applies to the specific fields
   wrapped in `<![CDATA[ ... ]]>`, not to the whole template body.
3. **Turtle**: add a general-purpose minijinja filter, `turtle_escape`,
   registered the same way, implementing standard Turtle string-literal
   escaping (see Required fixes item 3 for the exact character map). This
   filter is added to the rendering **engine** (available to any template
   that opts in via `| turtle_escape`) but is **not** applied to any
   template in this repo, since no `.ttl.j2` template exists in
   `sc-compose` — the actual repro template
   (`triaging-findings/triage-record.ttl.j2`) lives in `atm-core`, which is
   out of this repo's boundary per `CLAUDE.md` ("This repo is intentionally
   independent from ATM... Any ATM integration belongs in ATM adapters, not
   in this repo"). Providing the filter here, unused, is in scope; wiring
   it into the atm-core template is explicitly out of scope for this repo
   and must be filed as a separate issue against atm-core.

## Required fixes

1. In `legacy_auto_escape_callback`
   (`crates/sc-composer/src/renderer.rs`), add a match arm:
   `Some("json") => AutoEscape::Json`. Do not change the `.html`/`.htm`/
   `.xml` arm (still `AutoEscape::Custom("sc-compose-html")`) or the
   fallback (`AutoEscape::None`).
2. Confirm (via a test, not just inspection) that `format_sc_compose_markup`
   correctly falls through to `escape_formatter` for `AutoEscape::Json` —
   it already does structurally (the `if state.auto_escape() !=
   AutoEscape::Custom("sc-compose-html")` guard), but this must be covered
   by a test, not assumed.
3. Add `cdata_escape` filter function: given input text, replace every
   literal occurrence of `]]>` with `]]]]><![CDATA[>` (the standard
   CDATA-splitting escape: close the current CDATA section after the first
   `]]`, immediately reopen a new CDATA section, then continue with the
   trailing `>`). Register via `env.add_filter("cdata_escape", ...)` in
   `configure_environment`.
4. Add `turtle_escape` filter function implementing Turtle string-literal
   escaping (per the Turtle grammar's `ECHAR` production): escape (in this
   order, backslash first) `\` → `\\`, `"` → `\"`, newline → `\n`,
   carriage return → `\r`, tab → `\t`. Register via
   `env.add_filter("turtle_escape", ...)` in `configure_environment`.
5. Apply `| cdata_escape` to `reviewer_findings_json` and
   `previous_step_json` wherever they appear inside `<![CDATA[ ... ]]>`
   blocks in `.claude/skills/plan-hardening/01-plan-scope-review.xml.j2`,
   `02-sprint-scope-hardening.xml.j2`, and `03-consistency-hardening.xml.j2`.
6. Mandatory two-commit red→green process: commit 1 adds all failing tests
   below (fail against current `develop`); commit 2 applies the fix and all
   tests go green.
7. Required tests (add to `crates/sc-composer/src/renderer.rs`'s
   `#[cfg(test)] mod tests`, plus a CLI-level test in
   `crates/sc-compose/tests/` if appropriate):
   - Rendering a `payload.json.j2` template with a value containing `"`,
     `\`, and a control character (e.g. `\n`) via `{{ value }}` produces
     structurally valid JSON (parse the output with `serde_json::from_str`
     and assert it succeeds and round-trips the original string).
   - Issue #272's exact JSON-injection repro (a `sprint_id`-style value
     crafted to look like `x", "injected": true, "y": "x`) no longer
     produces a spoofed extra top-level key — the rendered value appears
     only as an escaped string, not as raw JSON syntax.
   - `cdata_escape` filter: input containing `]]>` mid-string round-trips
     safely inside a real `<![CDATA[ ... ]]>` wrapper (assert the output,
     when embedded in `<![CDATA[{{ value | cdata_escape }}]]>` and parsed
     as XML via a real XML parser, yields back the original unescaped
     text as the CDATA section's logical content — do not just
     string-match the escape sequence).
   - `cdata_escape` filter: input with no `]]>` is unchanged (identity for
     the common case).
   - `turtle_escape` filter: input containing `"`, `\`, and a newline
     produces the expected escaped output per the character map in
     Required fixes item 4 (unit test on the filter function directly —
     no `.ttl.j2` template exists in this repo to render against).
   - Regression: existing HTML/XML auto-escape tests
     (`renderer_preserves_slashes_in_markup_auto_escape` and friends from
     FIX-268) still pass unchanged.
8. Re-run issue #272's exact JSON-injection repro and confirm the injected
   key no longer appears in the rendered output.
9. Record the fix commit(s) and validation results in this doc's Closeout
   Evidence section before requesting QA.

## Out of scope (do not implement)

- Wiring `turtle_escape` into any atm-core template
  (`triaging-findings/triage-record.ttl.j2`) — that template does not exist
  in this repo; file a separate issue against atm-core if the filter should
  be adopted there.
- Auto-detecting CDATA blocks and escaping their contents automatically —
  `cdata_escape` is opt-in per-field via the `|` filter, consistent with
  the scope decision above (only specific fields are wrapped in CDATA, not
  the whole template).
- Any change to the `.html`/`.htm`/`.xml` auto-escape behavior established
  by FIX-268.
- YAML-specific escaping (not part of issue #272's repros; issue #276
  in the fuzz queue covers a related but distinct YAML fenced-block
  problem and is tracked separately).

## Acceptance criteria

- `cargo test --workspace` passes with 0 failures, including all new tests
  listed above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #272's exact JSON-injection repro produces structurally valid JSON
  with no spoofed extra key.
- A `]]>`-containing value rendered through `cdata_escape` inside a real
  `<![CDATA[ ... ]]>` block parses correctly via a real XML parser and
  round-trips the original content.
- `turtle_escape` filter is registered and unit-tested, with no template in
  this repo required to exercise it (per the Turtle scope decision above).
- Sprint doc Closeout Evidence section records exact fix commit(s) and
  validation results before requesting QA.

## References

- Issue #272: https://github.com/randlee/sc-compose/issues/272
- `crates/sc-composer/src/renderer.rs` (`legacy_auto_escape_callback`,
  `format_sc_compose_markup`, `configure_environment`)
- `.claude/skills/plan-hardening/01-plan-scope-review.xml.j2`,
  `02-sprint-scope-hardening.xml.j2`, `03-consistency-hardening.xml.j2`
  (CDATA fields to apply `cdata_escape` to)
- FIX-268 sprint doc / commit `116ae72a` (precedent: `AutoEscape::Custom`
  + `set_formatter`, the extension point this fix reuses)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

- Red tests committed and pushed at `4e65462` (`test: reproduce
  format-aware escaping gaps`); the new JSON, CDATA, and Turtle cases failed
  against the pre-fix implementation.
- Implementation committed and pushed at `0ccff88` (`fix: add format-aware
  output escaping`). It adds `.json` filename-aware `AutoEscape::Json`, the
  opt-in `cdata_escape` and `turtle_escape` filters, and CDATA protection to
  all affected plan-hardening fields. Existing HTML/XML behavior remains
  covered by the renderer regression tests.
- The exact JSON injection reproduction now parses successfully and contains
  no injected top-level key. The CDATA case round-trips through
  `quick_xml::Reader`, and Turtle escaping is verified directly against the
  required character map.
- Validation on `0ccff88`: `cargo test --workspace` passed with zero failures;
  `cargo fmt --all --check`, `cargo clippy --all-targets --all-features
  -- -D warnings`, and `git diff --check` passed. No new dependency was
  added.
- Merge-forward commit `b5972a8` incorporated the then-current `develop`
  fixes for FIX-270 and FIX-242-271. QA follow-up fix `5d156da` adds an
  end-to-end regression using the real plan-hardening template's CDATA block
  and marks `cdata_escape` output safe so the XML formatter preserves the
  reopened CDATA marker. Formatting and strict-lint cleanup landed at
  `4936318` and `28f8aa2`.
- Final validation at merge-forward HEAD `28f8aa2`: `cargo test --workspace`
  passed with 0 failures (including 224 library/CLI tests, 51 extraction
  integration tests, and 15 integration tests); `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` all passed.
