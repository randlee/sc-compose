---
id: FIX-278
status: dispatched
branch: fix/278-control-chars-survive-escape-filter
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/278-control-chars-survive-escape-filter
target: develop
---

# Sprint FIX-278 — Escape/Strip XML-Illegal Control Characters On The HTML/XML Escaping Path

## Problem

Issue #278: a raw C0 control byte (e.g. NUL) embedded in a var-file string
value passes straight through both the explicit `{{ value | e }}` escape
filter and `AutoEscape::Html` auto-escaping, unescaped and unstripped. This
is distinct from issue #268 (default, non-explicitly-escaped path) — here
the template author does everything right (explicitly opts into escaping)
and the output is still not well-formed XML/XHTML, because XML 1.0 forbids
most C0 control characters (`0x00`–`0x08`, `0x0B`, `0x0C`, `0x0E`–`0x1F`;
tab/LF/CR remain legal) in character data, while HTML text content has no
such restriction and the escaper only targets markup-class characters
(`& < > " '`).

Repro (issue #278, `templates/benchmark-report/benchmark-run.xhtml.j2` in
`atm-core` — **this template does not exist in this repo**; the underlying
engine defect (`sc-composer`'s escape filter / `AutoEscape::Html`) is
in-repo and is what this sprint fixes):

```
sc-compose render --mode file --file templates/benchmark-run.xhtml.j2 \
  --var-file vars/benchmark-run_nullbyte.json --root .
```

`title` containing an embedded NUL byte renders with exit 0, the literal
control byte still present in the "escaped" text, and the output fails
`xml.etree.ElementTree` well-formedness parsing.

Note: `crates/sc-composer/src/renderer.rs::legacy_auto_escape_callback`
only maps a `template_name` suffix of `html`/`htm`/`xml` (after stripping
`.j2`/`.jinja2`/`.jinja`) to `AutoEscape::Html` — `xhtml` is not currently
in that match arm, so the in-repo auto-escape path would not even trigger
for a literal `*.xhtml.j2` name today. That naming gap is real but is a
separate, narrower defect from the one in this issue (which is about the
**explicit** `| e` filter, invoked by template authors regardless of the
auto-escape name dispatch, and about `AutoEscape::Html`'s escaping content
itself once triggered by any `html`/`htm`/`xml`-named template). Confirm
and record both findings in Closeout Evidence; fixing the `xhtml` naming
gap is in scope only as a one-line addition to the existing match arm
(low risk, same file, same list) — do not restructure the naming dispatch
mechanism beyond that for this sprint.

## Root cause

Minijinja's built-in `escape`/`e` filter and `AutoEscape::Html` share the
same underlying HTML escaper, which only escapes `& < > " '`. Neither path
strips or numeric-character-references XML-illegal C0 control bytes. This
is the same class of defect FIX-272 already solved for JSON/CDATA/Turtle
output shapes (no format-aware escaping beyond minijinja's generic HTML
escaper) — XML/XHTML's stricter character-data legality rules are the
still-open case in that same family.

## Fix design (recommended; comp may adjust within these constraints)

Follow the FIX-272/FIX-274 precedent: a narrowly-scoped minijinja filter
plus (where it can be done safely) wiring into the existing auto-escape
dispatch, not a blanket change to minijinja's built-in escaper.

1. Add a new `xml_char_safe` filter in `crates/sc-composer/src/renderer.rs`
   (alongside `cdata_escape_filter`/`turtle_escape_filter`/
   `frontmatter_safe`) that, given a string value:
   - Applies the existing markup escaping (`& < > " '`) first (reuse
     minijinja's escaper or replicate it exactly — do not diverge from
     existing HTML-escaping behavior for markup characters).
   - Then replaces every XML-1.0-illegal C0 control byte (`0x00`–`0x08`,
     `0x0B`, `0x0C`, `0x0E`–`0x1F`) with its numeric character reference
     (e.g. `\x00` -> `&#x0;`), leaving tab (`0x09`), LF (`0x0A`), and CR
     (`0x0D`) untouched since those are legal XML character data.
   - Returns `JinjaValue::from_safe_string(...)` (same requirement FIX-274
     called out as the easy-to-forget step from the FIX-272 precedent).
2. Add a unit test proving the two-step order matters: a value containing
   both a markup character and a control byte (e.g. `"<\x00>"`) must come
   out with the markup escaped AND the control byte NCR-encoded, not just
   one or the other.
3. Wire `AutoEscape::Html`'s effective behavior for `html`/`htm`/`xml`
   (and, once fixed, `xhtml`) named templates to the same control-byte
   handling, so a template author who relies on auto-escape (not the
   explicit filter) gets the same protection. Investigate whether
   minijinja's `set_formatter`/`AutoEscape::Custom` (or equivalent in the
   pinned minijinja version — check `Cargo.toml`) lets a custom formatter
   intercept both the implicit auto-escape path and the explicit `e`/
   `escape` filter with one implementation; if the two paths cannot be
   unified with the pinned minijinja version, implement both explicitly
   (the new filter for `| e`-style explicit use, and a custom auto-escape
   formatter for the implicit path) rather than leaving one path
   unprotected — do not silently narrow scope to "filter only" without
   recording why in Closeout Evidence.
4. Fix the `xhtml` naming gap in `legacy_auto_escape_callback`: add
   `"xhtml"` to the existing `Some("html" | "htm" | "xml")` match arm.
   One-line change, same file.

## Required tests (two-commit red green process: commit 1 = all failing, commit 2 = fix)

1. Unit test: `xml_char_safe` NCR-encodes every XML-illegal C0 control
   byte in the forbidden ranges listed above, leaves tab/LF/CR untouched,
   and still HTML-escapes `& < > " '` in the same pass.
2. Unit test: ordinary text with no control bytes and no markup characters
   renders byte-identical (no unwanted escaping).
3. Auto-escape-path test (mirrors
   `renderer_keeps_auto_escape_scoped_to_html_like_names`): a template
   named `report.xml.j2` (or `.html.j2`) rendered with a control-byte value
   via plain `{{ value }}` (no explicit filter) produces NCR-encoded output
   if design step 3 unifies the paths; if design step 3 concludes the paths
   cannot be unified, this test instead documents and asserts the
   deliberately narrower scope.
4. Explicit-filter test: `{{ value | xml_char_safe }}` on a control-byte
   value produces well-formed output.
5. `xhtml` naming-gap regression: a template named `report.xhtml.j2`
   now receives the same `AutoEscape::Html`-family treatment as `.xml.j2`.
6. CLI-level regression: render a template with a NUL-byte-containing var
   through `sc-compose render`, feed the output to a Python
   `xml.etree.ElementTree.fromstring` well-formedness check (subprocess or
   equivalent in-test check), and assert it parses cleanly — mirrors the
   verification method used in the original issue repro.

## Out of scope

- Rejecting the render outright with a diagnostic instead of
  escaping/stripping (the issue names this as an alternative; this sprint
  follows the escape-don't-reject precedent already established by
  FIX-272/FIX-274 for consistency).
- `templates/benchmark-report/benchmark-run.xhtml.j2` and its var-file —
  does not exist in this repo (`atm-core`), out of bounds per `CLAUDE.md`.
- Any restructuring of `legacy_auto_escape_callback`'s naming-dispatch
  mechanism beyond adding the `xhtml` arm.
- General JSON/YAML/plain-text output escaping (already covered or
  explicitly out of scope per FIX-272/FIX-274).

## Acceptance criteria

- `cargo test --workspace` passes, including all new tests above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- A control-byte value rendered through the explicit `| e`-equivalent new
  filter (and, if unified, through plain auto-escape) on an XML/HTML/XHTML
  -declared template produces output that passes XML well-formedness
  parsing.
- Sprint doc Closeout Evidence records exact fix commit(s), validation
  results, whether the explicit-filter and auto-escape paths were unified
  or implemented separately (with rationale if separate), and confirmation
  that the `xhtml` naming gap was fixed.

## References

- Issue #278: https://github.com/randlee/sc-compose/issues/278
- `crates/sc-composer/src/renderer.rs` (`legacy_auto_escape_callback`,
  `cdata_escape_filter`, `turtle_escape_filter`, `frontmatter_safe` —
  precedent from FIX-272/FIX-274)
- PR #284 (FIX-272), PR #287 (FIX-274)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)
