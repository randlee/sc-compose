---
id: FIX-253
title: "Fix doubled message text when empty custom variable delimiters are rejected"
status: complete
branch: fix/253-doubled-delimiter-error-message
worktree: ../sc-compose-worktrees/fix/253-doubled-delimiter-error-message
target: develop
---

## Root Cause

Reproduced directly against a debug build of the CLI (see below for the
exact command and output). Passing empty strings to `--variable-delimiters`
produces:

```
ERR_CONFIG_PARSE: template rendering failed: invalid custom delimiters: invalid custom delimiters
```

The message text `invalid custom delimiters` appears twice in a single
error line. This is a formatting bug, not a duplicated validation call —
the underlying minijinja rejection only happens once.

The doubling comes from how the error is constructed and displayed across
three layers:

1. `crates/sc-composer/src/renderer.rs:111-122` — `Renderer::with_delimiters`
   builds a minijinja `SyntaxConfig` from the caller-supplied open/close
   strings. When both are empty, minijinja's builder rejects them and the
   error is wrapped via `RenderError::render(source)`
   (`crates/sc-composer/src/error.rs:401-409`), which stores the
   minijinja error both as `self.message` (`source.to_string()`) and as
   `self.source` (`Box::new(source)`, the same underlying error object).

2. `crates/sc-composer/src/error.rs:429-433` —
   `impl fmt::Display for RenderError`:
   ```rust
   impl fmt::Display for RenderError {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           write!(f, "template rendering failed: {}", self.source)
       }
   }
   ```
   This already embeds the full text of the underlying minijinja error
   (`"invalid custom delimiters"`) into `RenderError`'s own `Display`
   output.

3. `crates/sc-composer/src/error.rs:435-439` — `impl StdError for
   RenderError` also exposes `fn source(&self) -> Option<&(dyn StdError +
   'static)> { Some(self.source.as_ref()) }` — the *same* underlying
   minijinja error, not a distinct outer-vs-inner pair.

4. `crates/sc-compose/src/command_error.rs:95-106` —
   `impl fmt::Display for CommandError` renders the error with anyhow's
   alternate/`{:#}` formatter:
   ```rust
   if let Some(code) = self.diagnostic_code {
       write!(f, "{}: {:#}", code.as_str(), self.error)?;
   }
   ```
   `self.error` is `anyhow!(ComposeError::Render(render_error))`
   (via `CommandError::compose`, `command_error.rs:50-64`). anyhow's `{:#}`
   format walks the full `source()` chain and concatenates each link with
   `": "`. Because `RenderError::fmt` (layer 2) already prints
   `"template rendering failed: <source text>"`, and `RenderError::source()`
   (layer 3) hands anyhow that *same* `<source text>` again as the next
   link in the chain, anyhow's alternate formatter prints:
   `"template rendering failed: invalid custom delimiters" + ": " + "invalid custom delimiters"`
   — the doubled output observed.

This is specific to `RenderError`: its `Display` impl inlines the source's
text *and* its `source()` impl exposes that same object to callers that walk
the chain (anyhow's `{:#}`, `CommandError`'s only caller of this pattern).
Other error types in `error.rs` (e.g. `ConfigError`, confirmed by reading
`error.rs:441-` onward) do not exhibit this because `CommandError::compose`
only reaches `RenderError`'s doubled path for `ComposeError::Render`
variants — no other diagnostic-quality issue in this queue touches this
code path.

### Reproduction

```
$ sc-compose render --file t.md.j2 --variable-delimiters "" ""
Exit code 3
ERR_CONFIG_PARSE: template rendering failed: invalid custom delimiters: invalid custom delimiters
```

Call chain for this repro: `crates/sc-compose/src/commands/compose.rs:239`
(`custom_variable_delimiters`) accepts the two empty strings unchanged
(no empty-string validation exists there — clap's `num_args = 2` on
`--variable-delimiters`, `crates/sc-compose/src/cli/schema.rs:178-185`,
only requires exactly two values, not non-empty ones), then
`compose.rs:263` calls `Renderer::with_delimiters(&open, &close)`, which
fails as described above.

## Exact Target

Fix `RenderError`'s `Display` impl so it does not duplicate the source
error's text when the caller (here, `CommandError`'s anyhow `{:#}`
formatting) also walks the `source()` chain. The minimal, narrow fix is to
stop embedding the source's rendered text directly in `RenderError::fmt`
and let the `": "`-joined chain (already produced correctly by anyhow's
`{:#}` via `source()`) be the single place the underlying message appears:

In `crates/sc-composer/src/error.rs`, change:

```rust
impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "template rendering failed: {}", self.source)
    }
}
```

to:

```rust
impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "template rendering failed")
    }
}
```

`StdError::source()` (`error.rs:435-439`) is left unchanged — it already
correctly returns `Some(self.source.as_ref())`, and this remains the
single place the underlying minijinja message text is surfaced. Any
formatter that walks the `source()` chain (anyhow's `{:#}`, `{:?}`, or a
manual "Caused by:" loop) still gets the full detail; the plain,
non-alternate `Display` of a bare `RenderError` (e.g. `{}` with no source
walking) now shows only `"template rendering failed"` without embedding
the message twice when composed with chain-walking formatters.

This directly fixes the observed CLI output — instead of

```
ERR_CONFIG_PARSE: template rendering failed: invalid custom delimiters: invalid custom delimiters
```

it becomes

```
ERR_CONFIG_PARSE: template rendering failed: invalid custom delimiters
```

## This Sprint Does NOT Change

- `RenderError::message()` (`error.rs:422-426`, still returns
  `self.message`, i.e. `source.to_string()`) and the `message` field
  itself — untouched. Some callers may use `.message()` directly rather
  than `Display`; this sprint does not change what that returns.
- `RenderError::source()` / `StdError for RenderError` — untouched; it is
  the correct mechanism and is not the bug.
- `ConfigError`, `IncludeError`, `ResolveError`, or any other error type in
  `error.rs` — not touched; the doubling is specific to `RenderError`'s
  `Display` impl embedding source text that `source()` also exposes.
- `CommandError`'s `Display` impl (`command_error.rs:95-106`) — untouched;
  it is the caller correctly using anyhow's standard alternate-format
  chain-walking, not itself buggy.
- `Renderer::with_delimiters` (`renderer.rs:111-122`) and its validation
  logic, delimiter parsing, or minijinja's own error text — untouched;
  this sprint only fixes how the resulting error is displayed, not how or
  when it is produced.
- No change to `--variable-delimiters`'s clap definition
  (`cli/schema.rs:178-185`) or `custom_variable_delimiters`
  (`commands/compose.rs:299-314`) — empty strings remain a valid CLI input
  that is rejected downstream by minijinja with a normalized diagnostic,
  consistent with how other invalid-delimiter inputs are already handled
  (see the existing `with_delimiters_rejects_invalid_syntax_with_typed_error`
  test, `renderer.rs:303-311`, which already covers one non-empty invalid
  case, `("", "}}")`, at the `Renderer` level).

## Required Test Matrix

(a) **Red-baseline regression test**: assert that
`format!("{:#}", CommandError::compose(ComposeError::Render(render_error)))`
(or the equivalent path exercised by the CLI's `report_error`) for a
`RenderError` produced by `Renderer::with_delimiters("", "")` contains the
substring `"invalid custom delimiters"` **exactly once**, not twice. This
currently fails (the message appears twice). Place this in
`crates/sc-composer/src/error.rs`'s existing `#[cfg(test)] mod tests` next
to the other `RenderError`/`ComposeError` display tests (see
`error.rs:670-740` referenced in prior sprints for this module's test
conventions).

(b) A CLI-level integration test (in `crates/sc-compose/tests/cli/render.rs`,
matching that file's existing conventions) invoking
`sc-compose render --file <template> --variable-delimiters "" ""` and
asserting the stderr output contains `"invalid custom delimiters"` exactly
once — covers the full path end-to-end, not just the library-level
`Display` impl.

(c) The existing `with_delimiters_rejects_invalid_syntax_with_typed_error`
test (`renderer.rs:303-311`) continues to pass — it only asserts on
`RenderError`'s typed fields via `Renderer::with_delimiters("", "}}")
.unwrap_err()`, not on rendered `Display` text, so it is unaffected by this
change; confirm it still compiles and passes after the fix.

(d) A non-delimiter `RenderError` case (e.g. a genuine template
syntax error from `Renderer::render`, not `with_delimiters`) still produces
a sensible, non-empty `CommandError` display string via the same
`{:#}`-chain path — confirms the `Display` impl change does not silently
drop useful detail for other `RenderError` causes, it just stops
duplicating it.

## Mandatory Process (two-commit red -> green, standing requirement)

Confirmed clean 3/3 on FIX-245, FIX-244, and FIX-247. This fix's red
baseline is a **normal in-process assertion failure** (a string-equality /
substring-count assertion), not a process abort — unlike FIX-247, no
special crash-detection verification mode is needed here.

1. **First commit**: land test (a) above as `#[ignore]`d in `error.rs`'s
   test module. Team-lead independently runs
   `cargo test --workspace -p sc-composer -- --ignored <test_name>
   --nocapture` and confirms it genuinely fails (the doubled-text
   assertion does not hold against current `main`/`develop` behavior)
   before any fix code is written.
2. **Second commit**: land the `RenderError::fmt` fix, tests (b)-(d), and
   remove the single `#[ignore]` line from test (a). No other test-logic
   changes in this commit. Team-lead independently re-runs the same
   command and confirms it now passes, then runs the full
   `cargo test --workspace` to confirm no regression elsewhere.
3. Sprint-doc closeout narrative (if amended) must state accurate,
   verifiable provenance — the regression test is created fresh on this
   branch.

## Acceptance Criteria

- `cargo test --workspace` passes, including the now-unignored test (a)
  and new tests (b)-(d).
- `sc-compose render --file <template> --variable-delimiters "" ""`
  prints `ERR_CONFIG_PARSE: template rendering failed: invalid custom
  delimiters` (message appearing exactly once), not the doubled form.
- The pre-existing `with_delimiters_rejects_invalid_syntax_with_typed_error`
  test still passes unmodified.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #253 can be closed referencing the merged PR.

## Closeout Evidence

Status: **complete**.

- Red baseline: `a886a52` (`test: reproduce doubled delimiter error message`).
  The fresh regression reproduced two occurrences of `invalid custom
  delimiters` before the display fix.
- Green implementation: `1c47392` (`fix: avoid duplicated render error text`).
  `RenderError` now leaves source detail to the error chain, while the CLI
  regression and non-delimiter coverage preserve useful diagnostics.
- Follow-up stale-test corrections: `fda87e8`, `e12e0f4`, and `be71ec1`.
  These update pre-existing tests to inspect `source()` rather than expecting
  source detail in bare `Display`; no production logic changed in them.
- Formatting-only follow-up: `3e203a8`.
- Round-1 render-many source-chain follow-up: `09f0097` added
  `RenderManyError::source()` so the real underlying failure remains
  recoverable.
- Round-2 QA fix: `a44487e` makes `RenderManyError::Display`
  contextual-only, adds exact-occurrence and all-variant source coverage, and
  records the validated fix provenance.
- Full validation at the final branch state: `cargo test --workspace`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo fmt --all --check`, and `git diff --check`: PASS.

The sprint plan's claim that the existing renderer test was unaffected was
incorrect. Its Display-text assertion was edited in `1c47392` as a direct and
necessary consequence of the `RenderError::Display` change; the later test
follow-up updated the assertion to follow the source chain. All regression tests were created or
corrected on this branch with verifiable local provenance.

## References

- GitHub issue #253
- `crates/sc-composer/src/renderer.rs` (`Renderer::with_delimiters`,
  lines 111-122; existing test `with_delimiters_rejects_invalid_syntax_with_typed_error`,
  lines 303-311)
- `crates/sc-composer/src/error.rs` (`RenderError`, lines 388-439)
- `crates/sc-compose/src/command_error.rs` (`CommandError::compose`,
  lines 50-64; `impl fmt::Display for CommandError`, lines 95-106)
- `crates/sc-compose/src/commands/compose.rs` (`custom_variable_delimiters`,
  lines 299-314; call site at line 263)
- `crates/sc-compose/src/cli/schema.rs` (`--variable-delimiters` definition,
  lines 178-185)
