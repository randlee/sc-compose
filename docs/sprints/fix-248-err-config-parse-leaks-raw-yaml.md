---
id: FIX-248
title: "Stop leaking raw serde_yaml error text through ERR_CONFIG_PARSE's CLI text output"
status: complete
branch: fix/248-err-config-parse-leaks-raw-yaml
worktree: ../sc-compose-worktrees/fix/248-err-config-parse-leaks-raw-yaml
target: develop
---

## Root Cause

`crates/sc-composer/src/frontmatter/parser.rs::parse_template_document`
(lines 25-34) already wraps a `serde_yaml::from_str::<RawFrontmatter>`
failure in a stable, sc-compose-owned message:

```rust
let raw = serde_yaml::from_str::<RawFrontmatter>(frontmatter_text).map_err(|error| {
    ConfigError::new(
        DiagnosticCode::ErrConfigParse,
        "failed to parse YAML frontmatter",
    )
    .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
        key: "frontmatter".to_owned(),
    }))
    .with_source(error)
})?;
```

The `message` field (`"failed to parse YAML frontmatter"`) is already
stable and version-independent. The leak is not in the message text — it
is that `.with_source(error)` (line 33) attaches the raw
`serde_yaml::Error` as `ConfigError`'s `source`, and `ConfigError`'s
`Display` impl unconditionally walks and prints that source chain
regardless of caller:

`crates/sc-composer/src/error.rs:14-31` (`write_error_display`, used by
`ConfigError::fmt` at `error.rs:502-506`):

```rust
fn write_error_display(
    f: &mut fmt::Formatter<'_>,
    message: &str,
    source: Option<&(dyn StdError + 'static)>,
    backtrace: &Backtrace,
) -> fmt::Result {
    write!(f, "{message}")?;
    if let Some(source) = source {
        writeln!(f)?;
        write!(f, "caused by:")?;
        let mut current = Some(source);
        while let Some(error) = current {
            write!(f, "\n- {error}")?;
            current = error.source();
        }
    }
    write!(f, "\nbacktrace:\n{backtrace}")
}
```

This produces two divergent behaviors depending on which CLI output path
renders the error:

- **JSON path** (`--json`): clean. `crates/sc-compose/src/command_error.rs`,
  `compose_error_diagnostics()` (lines 137-160) builds the `Diagnostic` sent
  to the user from `config.message()` (`error.rs:490-494`, which returns
  only `self.message`, never touching `source`). No leak here.
- **Default (non-JSON) path**: leaks. `main.rs:114`
  (`eprintln!("{error}")`, `error: &CommandError`) uses
  `CommandError`'s `Display` impl (`command_error.rs:98-107`), which
  formats `self.error` — an `anyhow::Error` wrapping the original
  `ComposeError` — and `ComposeError`'s `Display`
  (`error.rs:545-...`) for the `Config` variant delegates straight to
  `ConfigError::fmt`, i.e. `write_error_display`. Because `.source()` is
  `Some(serde_yaml::Error)`, the "caused by:\n- <raw serde_yaml message>"
  line (plus a full backtrace dump) is appended to the stderr text the CLI
  actually prints for any non-`--json` invocation. `serde_yaml::Error`'s
  `Display` text (line/column phrasing, quoting style, etc.) is internal
  to that dependency and not covered by sc-compose's own stability
  contract — an unrelated `serde_yaml` version bump can silently change
  what users see on stderr today.

Reproduction: a frontmatter block whose opening delimiter has trailing
whitespace before terminating content that is not valid YAML (e.g. a
duplicate mapping key, or any other `serde_yaml` syntax error) triggers
this exact path — `parser.rs:25` is reached, `serde_yaml::from_str` fails,
and the default CLI render prints the raw dependency error text on
stderr. (Trailing-whitespace delimiters are simply how the fuzz probe
first reached this parse-error branch; the leak itself applies to *any*
YAML syntax error in a frontmatter block, not specifically to
whitespace-related ones.)

## Exact Target

Stop attaching the raw `serde_yaml::Error` as this `ConfigError`'s
`source` at the one call site that produces `ERR_CONFIG_PARSE` for
frontmatter YAML syntax errors. The message is already stable; removing
`.with_source(error)` here removes the only thing that made `Display`
print raw dependency text for this diagnostic code, without touching
`ConfigError`, `write_error_display`, or any other `ConfigError`
construction site's behavior.

In `crates/sc-composer/src/frontmatter/parser.rs`, lines 25-34:

```rust
let raw = serde_yaml::from_str::<RawFrontmatter>(frontmatter_text).map_err(|_error| {
    ConfigError::new(
        DiagnosticCode::ErrConfigParse,
        "failed to parse YAML frontmatter",
    )
    .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
        key: "frontmatter".to_owned(),
    }))
})?;
```

The only diff is dropping `.with_source(error)` (and renaming the closure
parameter to `_error` since it is now unused — do not remove the
`.map_err` closure itself, other call sites in this file follow the same
`map_err` shape and this keeps the diff minimal). `DiagnosticCode::ErrConfigParse`,
the message text, and the recovery hint are all unchanged.

## This Sprint Does NOT Change

- `ConfigError`, `write_error_display`, or `ComposeError`'s `Display`
  impls (`error.rs`) — untouched. The leak is fixed by not feeding a raw
  dependency error into the existing (correct, general-purpose) source-chain
  printer, not by changing how that printer works.
- The other ~10 call sites across `init_workspace.rs`, `frontmatter_init.rs`,
  `resolver.rs`, and `verify.rs` that also call `ConfigError::new(...).with_source(error)`
  for other diagnostic codes (e.g. I/O errors during workspace init). Those
  wrap `std::io::Error`, not `serde_yaml::Error`, are a different failure
  class from what issue #248 reports, and are out of scope for this
  narrowly-targeted fix. (If the same concern applies to them, that is a
  separate follow-up issue, not silently bundled into this one.)
- `RawFrontmatter`'s deserialization schema, `normalize_frontmatter`, or
  any other frontmatter-parsing logic in this file — only the error
  construction at the `serde_yaml::from_str` call site changes.
- The `--json` output path (`compose_error_diagnostics` in
  `command_error.rs`) — it already only used `.message()` and was never
  affected by this leak; this fix does not change JSON output at all.
- `DiagnosticCode::ErrConfigParse`'s value, the message text
  `"failed to parse YAML frontmatter"`, or the `ReviewConfiguration`
  recovery hint — all unchanged.

## Required Test Matrix

New CLI-level test in `crates/sc-compose/tests/fuzz_regressions.rs`
(this bug is CLI-reachable via the default, non-`--json` render path —
unlike FIX-247, a `crates/sc-compose/tests/` test is the right level).

(a) **Red-baseline regression test (mandatory `#[ignore]`d test — see
Process section)**: render a template whose frontmatter is a `---`
block containing YAML that fails to parse (e.g. a duplicate mapping key,
which `serde_yaml` rejects with an internal, dependency-specific message)
without `--json`. Assert the process exits non-zero (`ErrConfigParse`'s
exit code) and that **stderr does not contain any of `serde_yaml`'s
internal error vocabulary** — concretely, assert stderr does NOT contain
the substring `"caused by"` (the literal string `write_error_display`
prepends before dumping the source chain) and does NOT contain
`"backtrace:"`. Also assert stderr DOES contain the stable message
`"failed to parse YAML frontmatter"`. **Before the fix, this test
fails** (a normal in-process assertion failure — stderr does contain
`"caused by"` followed by raw `serde_yaml` text and a backtrace dump; no
crash-mode verification needed here, unlike FIX-247).

(b) The same non-`--json` render, with `--json` added: stderr/stdout
JSON envelope still contains the diagnostic `message` field equal to
`"failed to parse YAML frontmatter"` (or containing it) — confirms the
already-clean JSON path is unaffected by this change (regression guard,
not new behavior).

(c) A frontmatter block with a genuinely well-formed, parseable YAML
document (regardless of schema validity) does not hit this error path at
all — confirms the `.map_err` closure change didn't alter the success
path. This can reuse an existing passing fixture/test rather than adding
a new one if one already covers a clean frontmatter render; note in the
sprint closeout which existing test serves this role if so.

(d) The existing `closing_delimiter_with_trailing_whitespace_still_fails`
unit test in `parser.rs` (lines 190-195) continues to pass unmodified —
it asserts on `"no closing delimiter was found"`, a different
`ErrConfigParse` construction site (`parser.rs:66-74`) that never called
`.with_source()` in the first place, so it is unaffected either way; it
serves as a regression guard that this sprint didn't accidentally touch
that call site.

## Mandatory Process (two-commit red -> green, standing requirement, confirmed clean 3/3 on FIX-245/244/247)

1. **First commit**: land test (a) above as `#[ignore]`d in
   `crates/sc-compose/tests/fuzz_regressions.rs`. Team-lead independently
   confirms it genuinely fails before any fix code is written — this is a
   normal `cargo test -- --ignored <test_name>` assertion failure (stderr
   contains `"caused by"` / raw `serde_yaml` text pre-fix), not a process
   abort, so standard (not crash-mode) verification applies.
2. **Second commit**: land the `parser.rs` fix (drop `.with_source(error)`,
   rename the closure param to `_error`) plus tests (b)-(c) if new, and
   remove the single `#[ignore]` line from test (a). No other test-logic
   changes in this commit. Team-lead independently re-runs the same
   command from step 1 and confirms it now passes.
3. Sprint-doc closeout narrative must state accurate, verifiable
   provenance — the regression test is created fresh on this branch,
   never described as promoted from elsewhere.

## Acceptance Criteria

- `cargo test --workspace` passes, including the now-unignored test (a)
  and any new tests from (b)-(c).
- A frontmatter YAML syntax error rendered without `--json` prints only
  the stable message `"failed to parse YAML frontmatter"` (plus the
  existing recovery-hint line) on stderr — no `"caused by"` section, no
  raw `serde_yaml`-internal text, no backtrace dump.
- The `--json` output path is unchanged (still uses `.message()`, which
  was already clean).
- `closing_delimiter_with_trailing_whitespace_still_fails` and all other
  pre-existing `parser.rs` unit tests continue to pass unmodified.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #248 can be closed referencing the merged PR.

## Closeout Evidence

The regression test was created fresh on this branch and was not promoted
from another worktree.

- `eecad77` introduced the ignored CLI red-baseline test; `308d7a6` is the
  corrected red state with the pre-existing opening-delimiter test restored
  to its original exit-code assertion. Before the parser fix, the ignored
  test failed normally because stderr contained `caused by` and raw
  serde_yaml error text.
- `4a85d49` removes only the serde_yaml source attachment from the
  frontmatter parse-error construction, removes the single `#[ignore]`, and
  adds the JSON-path regression guard. `c65ba50` fixes the new test comment's
  Clippy documentation lint.
- Default text output retains the stable `failed to parse YAML frontmatter`
  message and recovery hint without the `caused by` source-chain section or
  raw serde_yaml text.
- JSON output remains structured with `ERR_CONFIG_PARSE` and the stable
  diagnostic message. Existing parseable-frontmatter coverage in
  `adjacent_plain_yaml_frontmatter_block_is_not_silently_consumed_as_a_second_pass`
  serves as the success-path guard.
- Targeted fuzz regressions: PASS (7/7); parser unit tests: PASS (4/4).
- Workspace tests: PASS (`cargo test --workspace`).
- Clippy: PASS (`cargo clippy --all-targets --all-features -- -D warnings`).
- Formatting and whitespace checks: PASS (`cargo fmt --all --check` and
  `git diff --check`).

### Scope discrepancy requiring review

The sprint's original test wording also required default stderr to omit
`backtrace:` while forbidding changes to `ConfigError` and
`write_error_display`. That formatter unconditionally emits the existing
`backtrace:` block even when no source is attached, and its existing unit
tests lock in that behavior. The implementation therefore leaves the
backtrace behavior unchanged and verifies the issue-specific leak: removal
of the raw serde_yaml source chain. Team-lead should reconcile that wording
before final issue closure.

## References

- GitHub issue #248
- `crates/sc-composer/src/frontmatter/parser.rs` (`parse_template_document`,
  lines 14-44)
- `crates/sc-composer/src/error.rs` (`write_error_display`, lines 14-31;
  `ConfigError`, lines 441-514)
- `crates/sc-compose/src/command_error.rs` (`CommandError::fmt`, lines
  98-107; `compose_error_diagnostics`, lines 137-160)
- `crates/sc-compose/src/main.rs` (`report_error`, lines 110-116)
