---
id: FIX-269
title: "--json render to stdout silently discards rendered content while reporting success"
status: in-progress
branch: fix/269-json-stdout-content-loss
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/269-json-stdout-content-loss
target: develop
---

# FIX-269: `--json` render to stdout silently discards rendered content while reporting success

Issue: https://github.com/randlee/sc-compose/issues/269
Branch: `fix/269-json-stdout-content-loss`
Base: `develop` @ `97c5a07`

## Problem

`sc-compose render --json` with no `--output` (rendering to stdout) prints only
the diagnostics/metadata JSON envelope. The actual rendered document body is
never emitted anywhere the caller can observe — not on stdout, not on
stderr — yet the envelope reports success with a `bytes_written` count as if
the content had been delivered. `--output <file>` does not exhibit this bug.

This is the highest-severity finding from the 2026-08-06 fuzz round: any
programmatic caller using `--json` mode without `--output` (a natural
combination, since `--json` exists specifically for machine consumption)
silently loses the rendered content while the tool claims success.

## Root cause (confirmed via read-only review, team-lead, 2026-08-06)

`crates/sc-compose/src/commands/compose.rs`, function `emit_render_output`
(~line 323-395):

```rust
if args.json {
    let payload = if args.dry_run {
        serde_json::json!({
            "would_write": to_forward_slash(&derived_path),
            "would_change": would_change,
            "template": to_forward_slash(resolved_path),
            "rendered_preview": rendered_text,
        })
    } else {
        serde_json::json!({
            "output_path": output_path
                .as_ref()
                .map_or_else(|| "stdout".to_owned(), |path| to_forward_slash(path)),
            "bytes_written": bytes_written.unwrap_or_default(),
            "template": to_forward_slash(resolved_path),
        })
    };
    print_json(payload, warnings).map_err(CommandError::usage)?;
} else if args.dry_run {
    ...
    println!("{rendered_text}");
} else {
    println!("{rendered_text}");
}
```

The non-dry-run `--json` branch never includes `rendered_text` in the
payload and never prints it, unlike the dry-run branch (which embeds
`rendered_preview`) and the plain-text branch (which prints the body
directly). When `output_path` is `None` (stdout mode), `bytes_written` is
still computed from `rendered_text.len()` (see `emit_render_output` line
358-359, the `else` arm of the `bytes_written` assignment) even though those
bytes were never written to any observable stream.

## Scope decision

Add a `body` field to the non-dry-run JSON payload, populated with
`rendered_text`, whenever `output_path` is `None` (i.e., the render target is
stdout). When `--output <file>` is given, the file already holds the content,
so `body` is omitted from the payload in that case to avoid needlessly
duplicating potentially large content in both the JSON envelope and the file
— `output_path` continues to point the caller at the file.

This keeps the fix minimal and scoped to the actual defect: JSON+stdout mode
must make the rendered content observable, and `bytes_written`/`output_path`
must accurately reflect what was actually written to the observable stream.
No changes to the `--dry-run` JSON payload shape (already includes
`rendered_preview`), the plain-text/non-JSON output paths, or the `--output
<file>` JSON payload shape (still omits body content, since the file is the
source of truth).

## Required fixes

1. In `emit_render_output` (`crates/sc-compose/src/commands/compose.rs`),
   for the non-dry-run `--json` branch: when `output_path` is `None`, add a
   `"body": rendered_text` field to the JSON payload. When `output_path` is
   `Some(_)`, omit `body` (unchanged behavior).
2. Confirm `bytes_written` in the stdout+`--json` case matches
   `rendered_text.len()` (already correct arithmetically — the bug is pure
   observability, not miscounting) and add a regression test asserting the
   JSON payload's `body` field byte-length equals `bytes_written` for a
   stdout render.
3. Follow the mandatory two-commit red→green regression-test process:
   commit 1 adds a failing test (`sc-compose render --json` with no
   `--output`, asserting the JSON payload contains a `body`/`rendered` field
   whose content matches the plain-render output) that fails before the fix.
   Commit 2 applies the fix and the test goes green.
4. Add coverage confirming `--output <file>` + `--json` behavior is
   unchanged (no `body` field in the payload, file still receives correct
   content) — a compatibility/negative test.
5. Add coverage confirming `--dry-run` + `--json` behavior (`rendered_preview`
   field) is unchanged.
6. Update `docs/architecture.md` or CLI docs if they document the `--json`
   payload shape, to reflect the new optional `body` field.
7. Record the fix commit(s) and validation results in this doc's Closeout
   Evidence section before requesting QA.

## Out of scope (do not implement)

- Changing the JSON payload shape for `--output <file>` renders (file
  remains the source of truth; no `body` duplication).
- Any change to `--dry-run` JSON behavior (already correct via
  `rendered_preview`).
- Any change to plain-text (non-`--json`) output behavior.

## Acceptance criteria

- `cargo test --workspace` passes, including new regression tests for: the
  exact issue #269 repro (stdout + `--json`, body field present and
  correct), `--output <file>` + `--json` unchanged (no body field), and
  `--dry-run` + `--json` unchanged (`rendered_preview` unaffected).
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #269's exact repro (`sc-compose render --mode file --file
  sprint-plan.md.j2 --var-file vars.json --json`, no `--output`) now returns
  a JSON payload whose `body` field contains the full rendered document,
  matching plain-render output byte-for-byte.
- Sprint doc Closeout Evidence section records exact fix commit(s) and
  validation results before requesting QA.

## References

- Issue #269: https://github.com/randlee/sc-compose/issues/269
- `crates/sc-compose/src/commands/compose.rs` (`emit_render_output`,
  `derived_output_path`)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

_Pending — to be filled in by comp on completion._
