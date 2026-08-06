---
id: FIX-268
title: "XML/HTML auto-escape is bypassed on the default render path"
status: complete
branch: fix/268-xml-format-autoescape-not-applied
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/268-xml-format-autoescape-not-applied
target: develop
---

# FIX-268: XML/HTML auto-escape is bypassed on the default render path

Issue: https://github.com/randlee/sc-compose/issues/268
Branch: `fix/268-xml-format-autoescape-not-applied`
Base: `develop` @ `65b90e8`

## Problem

`sc-compose render --file <template>.xml.j2 ...` (the default, non-custom-delimiter
CLI render path, and the equivalent library `compose()`/`compose_with_observer()`
entry points) produces non-well-formed XML when an interpolated variable
contains literal `<`, `>`, or bare `&`. No auto-escaping is applied, even
though this project's own convention — confirmed by the user — is to name
XML-emitting templates `<name>.xml.j2` (see every template in
`.claude/skills/codex-orchestration/*.xml.j2`, all rendered via this exact
CLI path). Any free-text field interpolated into those templates (findings,
descriptions, triage records) can currently produce malformed XML.

## Root cause (confirmed via read-only review, comp, 2026-08-06)

`crates/sc-composer/src/renderer.rs` already has an extension-based
auto-escape mechanism:

```rust
fn legacy_auto_escape_callback(name: &str) -> AutoEscape {
    let mut name = name;
    for extension in [".j2", ".jinja2", ".jinja"] {
        if let Some(stripped) = name.strip_suffix(extension) {
            name = stripped;
            break;
        }
    }
    match name.rsplit('.').next() {
        Some("html" | "htm" | "xml") => AutoEscape::Custom("sc-compose-html"),
        _ => AutoEscape::None,
    }
}
```

This callback is wired into every `Environment` via `configure_environment`.
The renderer's custom formatter applies the XML/HTML-required escapes while
deliberately preserving `/` for readable paths and URLs. Templates loaded
through `Renderer::render_named(name, ...)` therefore retain the real
filename-aware behavior without Minijinja's stock `AutoEscape::Html` slash
encoding.

The bug: the render call sites that the CLI's default path actually uses call
`Renderer::render(...)`, which is a thin wrapper —
`renderer.rs:129-134` — that always names the template the literal string
`"inline"`:

```rust
pub fn render<T: serde::Serialize>(&self, template: &str, context: T) -> Result<String, RenderError> {
    self.render_named("inline", template, context)
}
```

`"inline"` never matches `.xml`/`.html`/`.htm`, so `AutoEscape::None` is
always selected on this path, regardless of the template's actual file name
or its `format: xml` frontmatter (which is not parsed or consumed anywhere
in `sc-composer` — confirmed by grep, there is no `OutputFormat` type or
`"format"` frontmatter key handling in the crate today; `format:` is
currently pure documentation with zero effect on rendering).

Three call sites hit this literal-`"inline"` path with a real resolved file
path available in scope but not passed through:

1. `crates/sc-composer/src/composer.rs:99-107` — the single-pass render call
   inside `compose_with_observer` (the default `compose()` path; this is what
   `sc-compose render --file x.xml.j2` without `--variable-delimiters` hits).
2. `crates/sc-composer/src/composer.rs:280-330`
   (`render_all_with_observer`) — the multi-pass render loop, called both by
   `compose_with_observer` (same default path, when a template has more than
   one frontmatter pass) and by the public `render_all()` API
   (`composer.rs:141-147`, re-exported from `lib.rs:42`, consumed directly by
   `bindings/python/src/functions.rs:143-148` and by
   `crates/sc-composer/tests/integration.rs`).
3. `crates/sc-compose/src/commands/compose.rs:230-280`
   (`execute_custom_delimiter_render`) — the `--variable-delimiters`/
   `--brace-count` custom-delimiter render path, same bug, same missing name.

## Scope decision

Fix via the **existing filename-extension convention**
(`legacy_auto_escape_callback`), not by adding a new `format:` frontmatter
field to the composer. Rationale:
- The extension mechanism already exists, is already tested, and matches this
  project's actual template-naming convention (`*.xml.j2`, `*.html.j2`). The
  renderer uses a small project formatter rather than stock
  `AutoEscape::Html`, because slash encoding is not required for XML/HTML
  well-formedness and harms readability of paths and URLs in protocol files.
- Adding real `format:` frontmatter parsing/consumption would be a much
  larger, separately-scoped change (new frontmatter field, validation,
  precedence rules against filename, docs) that is not needed to fix the
  concrete, reproducible bug in #268.
- `render_all()` is a public API consumed by `bindings/python` and has its
  own test suite (`crates/sc-composer/tests/integration.rs`) that asserts
  `"inline"`-named, unescaped behavior for arbitrary in-memory template
  strings with no associated file path. That public API's documented
  behavior and existing tests must not change — only the *internal* call
  from `compose_with_observer` (which does have a real resolved path) gets
  the real name.

## Required fixes

1. In `crates/sc-composer/src/composer.rs`, thread the resolved template's
   file name (e.g. `validation_report.resolve_result.resolved_path`'s
   `file_name()`, as a `&str`, falling back to `"inline"` if the path has no
   file-name component) into the render call at the single-pass site
   (~line 99-107): call `renderer.render_named(name, ...)` instead of
   `renderer.render(...)`.
2. In `render_all_with_observer` (~line 280-330), add a `template_name: &str`
   parameter and use `Renderer::with_delimiters(...)`'s renderer via
   `render_named(template_name, ...)` instead of `.render(...)` for each
   pass. Update its one internal caller inside `compose_with_observer` to
   pass the same resolved file name from fix (1). Update the public
   `render_all()` wrapper (~line 141-147) to keep passing the literal
   `"inline"` name explicitly, so its existing public contract and tests
   (`crates/sc-composer/tests/integration.rs`) are unchanged.
3. In `crates/sc-compose/src/commands/compose.rs`
   (`execute_custom_delimiter_render`, ~line 230-280), apply the same
   pattern: pass the resolved file name into
   `Renderer::with_delimiters(...).render_named(name, ...)` instead of
   `.render(...)`.
4. Preserve each call site's existing error handling, diagnostics, and
   `CommandError`/`RenderError` wrapping exactly — only the template naming
   changes, not error behavior.
5. Follow the mandatory two-commit red→green regression-test process:
   commit 1 adds a failing test reproducing the exact issue #268 repro
   (`.xml.j2` template, variable containing `<`, `>`, bare `&`, rendered via
   the default `sc-compose render --file` path with no custom delimiters;
   assert the output is NOT well-formed XML pre-fix, i.e. write the test to
   assert the *fixed* (escaped, well-formed) behavior so it fails before the
   fix and passes after — do not commit a test that asserts today's broken
   output). Commit 2 applies fixes 1-3 and the test goes green.
6. Add equivalent coverage for the custom-delimiter path (fix 3) and for
   `.html`/`.htm`-named templates (not just `.xml`), reusing the existing
   `render_named` test pattern in `renderer.rs`'s test module as a reference.
7. Add a negative/compatibility test proving:
   - `render_all()`'s public behavior (no file path, `"inline"` naming, no
     escaping) is unchanged — the existing integration tests in
     `crates/sc-composer/tests/integration.rs` must continue to pass
     unmodified.
   - A non-`.xml`/`.html`-named template (e.g. `payload.json.j2`,
     `notes.md.j2`) rendered through the default `compose()` path remains
     unescaped after this fix (no regression to JSON/YAML/plain-text
     output).
8. Confirm the exact issue #268 repro now round-trips as well-formed XML:
   `python3 -c "import xml.etree.ElementTree as ET; ET.parse('out.xml')"`
   must succeed for the issue's `repro.xml.j2` + `vars.json` example.

## Out of scope (do not implement)

- Parsing or consuming a `format:` frontmatter field. Not required for this
  fix; would be a separately-scoped enhancement.
- Adding a dedicated `xmlescape`/`x` Jinja filter. The existing built-in
  `escape`/`e` filters already work correctly as an opt-in today (confirmed
  by comp's review) and remain available for template authors who need
  explicit escaping in a template not named `.xml`/`.html`.
- Attribute-context-specific escaping rules beyond what minijinja's built-in
  `AutoEscape::Html` mode already provides.

## Acceptance criteria

- `cargo test --workspace` passes, including new regression tests for: the
  exact issue #268 repro (default path), the custom-delimiter path, an
  `.html`-named template, `render_all()`'s unchanged public behavior, and a
  non-XML/HTML template remaining unescaped.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- `python3 -c "import xml.etree.ElementTree as ET; ET.parse('out.xml')"`
  succeeds against the issue's exact repro rendered on this branch.
- No change to `render_all()`'s public signature or documented behavior;
  `bindings/python` continues to build and its existing tests pass unchanged.
- Sprint doc Closeout Evidence section records exact fix commit(s) and
  validation results before requesting QA.

## References

- Issue #268: https://github.com/randlee/sc-compose/issues/268
- comp's read-only review (REVIEW-268, 2026-08-06): confirmed reproducible
  3/3 on `develop@65b90e8`/`2f1bc92`; root cause and findings above are
  taken directly from that review.
- `crates/sc-composer/src/renderer.rs` (`legacy_auto_escape_callback`,
  `Renderer::render`/`render_named`, existing `render_named` tests)
- `crates/sc-composer/src/composer.rs` (`compose_with_observer`,
  `render_all_with_observer`, public `render_all`)
- `crates/sc-compose/src/commands/compose.rs`
  (`execute_custom_delimiter_render`)
- `crates/sc-composer/tests/integration.rs` (public `render_all()` tests —
  must remain passing unmodified)
- `bindings/python/src/functions.rs:143-148` (`render_all` Python binding —
  must remain unaffected)

## Closeout Evidence

- Status: **complete**.
- Red baseline: `6621e0c` (`test: reproduce XML autoescape bypass`) failed on
  the pre-fix default CLI path because the rendered XML retained raw `<`, `>`,
  and `&` characters.
- Green implementation: `5761c1b3` (`fix: preserve filename-aware autoescape
  on render paths`) threads the resolved filename through the single-pass,
  multi-pass, and custom-delimiter render paths. It also adds XML, custom
  delimiter, HTML, non-markup, and public `render_all()` regression coverage.
- TL-VERIFY-268-001 resolution: selected direction (a). The follow-up
  formatter fix uses the filename convention to escape `&`, `<`, `>`, `"`,
  and `'`, but leaves `/` unchanged. Existing protocol-template assertions
  therefore remain raw and continue to prove readable paths, branch names,
  and URLs; no test was weakened to accept slash corruption.
- Exact issue #268 reproduction (`repro.xml.j2` + `vars.json`) rendered to
  `out.xml`; `python3 -c "import xml.etree.ElementTree as ET; ET.parse('out.xml')"`
  passed.
- Validation passed: `cargo fmt --all --check`, `cargo test --workspace`
  (90 unit, 146 CLI, 51 extraction-integration, and 14 composer-integration
  tests), `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check`.
