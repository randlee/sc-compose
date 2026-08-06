---
id: FIX-277
status: complete
branch: fix/277-bytes-written-off-by-one
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/277-bytes-written-off-by-one
target: develop
---

# Sprint FIX-277 — `payload.bytes_written` Off-By-One On The Stdout `--json` Path

## Problem

Issue #277: `sc-compose render --json` reports `payload.bytes_written` one byte
short of what stdout actually receives when rendering to stdout (no
`--output`). Reproduced 3/3 by the reporter: a plain (non-`--json`) stdout
render of the same input is 4843 bytes per `wc -c`, while `--json` mode's
`payload.bytes_written` reports 4842 for the identical rendered content. The
`--output <file>` path is unaffected — file byte count and reported
`bytes_written` agree exactly (4842 both).

## Root cause

`crates/sc-compose/src/commands/compose.rs::emit_render_output` (line ~358-360):

```rust
} else {
    Some(rendered_text.len())
};
```

This is the stdout branch (`output_path.is_none()`, not dry-run). It reports
the raw rendered-text byte length with no accounting for a trailing newline.
Contrast with the plain (non-`--json`) stdout path at line ~398,
`println!("{rendered_text}")`, which always appends a trailing `\n` — so a
plain-mode stdout render of the same content is `rendered_text.len() + 1`
bytes. The file-write path (line ~336, `std::fs::write(output,
rendered_text)`) writes no extra newline, so its `bytes_written` (derived from
`std::fs::metadata(output).len()` at line ~343) is correctly
`rendered_text.len()` with no adjustment needed — this path is explicitly
confirmed correct in the issue and is out of scope for this fix.

The `--json` stdout branch's `bytes_written` field is meant to describe "bytes
the caller would receive from this render target" (per the issue's Expected
section: "should equal the actual number of bytes emitted to the render
target, including any trailing newline written to stdout"), and the
reporter's basis for comparison is the plain-mode stdout byte count. The field
must therefore include the same trailing-newline accounting used by the
plain-mode stdout path, even though `--json` mode itself does not literally
`println!(rendered_text)` to stdout (it emits a JSON envelope via
`print_json`) — the field documents the logical render-target byte count, not
the literal JSON envelope's own byte count, consistent with how the
file-write branch already reports the target file's bytes rather than the
JSON envelope's bytes.

## Fix design (recommended; comp may adjust within these constraints)

1. In `emit_render_output`, change the stdout (non-dry-run, no `--output`)
   branch from `Some(rendered_text.len())` to
   `Some(rendered_text.len() + 1)` to account for the trailing newline the
   stdout render target receives, matching the plain-mode stdout path's
   actual byte count.
   - Do not change the `--output <file>` branch (lines ~335-357) — already
     confirmed correct by the issue reporter and by
     `render_json_reports_actual_bytes_written_for_output_file` (existing
     test at `crates/sc-compose/tests/json_cli/render.rs:520`).
   - Do not change the dry-run branch (`None`) — dry-run writes nothing.
2. Add a focused regression test proving the `--json` stdout
   `payload.bytes_written` equals `rendered_text.len() + 1` for a real
   rendered template, distinct from the existing
   `crates/sc-compose/tests/json_cli/render.rs:81` assertion
   (`body.len() as u64 == bytes_written`) — that existing assertion is
   actually part of what needs to change: `body` in the JSON payload is the
   raw `rendered_text` (line ~375, `"body": rendered_text`), so once
   `bytes_written` becomes `rendered_text.len() + 1`, `body.len() as u64 ==
   bytes_written` becomes false by construction. Update or replace that
   assertion to compare against `body.len() as u64 + 1` (or an equivalent
   documented delta), and add a comment or test name that makes the +1
   trailing-newline accounting explicit so a future reader does not
   "fix" it back to equality.
3. Add a CLI-level regression mirroring the issue's own repro shape: render
   the same template both in plain (non-`--json`) mode to stdout, capturing
   `wc -c`-equivalent byte length in-test, and in `--json` mode, asserting
   `payload.bytes_written` equals the plain-mode stdout byte count exactly.

## Required tests (two-commit red green process: commit 1 = all failing, commit 2 = fix)

1. Unit/CLI test: `--json` stdout render reports `bytes_written ==
   rendered_text.len() + 1`.
2. Update the existing `crates/sc-compose/tests/json_cli/render.rs:81`
   assertion (`body.len() as u64 == bytes_written`) to account for the
   trailing-newline delta rather than exact equality, with a comment
   documenting why.
3. CLI-level regression: plain-mode stdout byte count (via an in-test
   equivalent of `wc -c`) equals `--json` mode's reported `bytes_written` for
   the same template/vars.
4. Confirm `render_json_reports_actual_bytes_written_for_output_file`
   (`crates/sc-compose/tests/json_cli/render.rs:520`, the `--output <file>`
   path) is unaffected and still passes unmodified — this is a regression
   guard proving the fix is scoped to the stdout branch only.
5. Dry-run `--json` path (`payload.would_write`/`would_change`, no
   `bytes_written` key at all) unaffected — confirm via existing tests, no
   new test needed unless coverage is missing.

## Out of scope

- The `--output <file>` byte-count path — already correct, do not touch.
- Any change to plain (non-`--json`) stdout rendering behavior itself
  (`println!("{rendered_text}")` at line ~398) — only the `--json` metadata
  field is wrong, not the actual bytes written in plain mode.
- The more severe, separately-reported `--json` stdout content-loss bug
  (issue #269, already fixed) — this sprint is the distinct 1-byte
  metadata-accounting issue only.

## Acceptance criteria

- `cargo test --workspace` passes, including all new/updated tests above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #277's exact repro shape (`--json` stdout render of a real template)
  reports `payload.bytes_written` equal to the actual byte count stdout would
  receive in plain mode for the same content.
- No new dependency added to `Cargo.toml`.
- Sprint doc Closeout Evidence records exact fix commit(s), validation
  results, and confirmation that the `--output <file>` path and dry-run path
  remain unaffected.

## References

- Issue #277: https://github.com/randlee/sc-compose/issues/277
- `crates/sc-compose/src/commands/compose.rs` (`emit_render_output`)
- `crates/sc-compose/tests/json_cli/render.rs` (lines 81, 520)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

- Red regression tests and the bundled FIX270-QA-003 citation correction were
  committed and pushed at `31b7cba` (`test: reproduce stdout bytes-written
  off-by-one`). The red tests demonstrated the old one-byte-short metadata
  for JSON stdout renders.
- The implementation was committed and pushed at `07e4ca0` (`fix: include
  stdout newline in byte count`). Only the non-dry-run stdout branch now adds
  one byte for the `println!` trailing newline.
- Focused validation passed: JSON stdout `body.len() + 1`, plain-vs-JSON
  stdout byte parity, and the unchanged `--output` file-byte regression.
- Full validation passed: `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check`.
- The `--output <file>` branch remains metadata-derived from the file size and
  was not changed. The dry-run branch still reports no `bytes_written` field
  and was covered by the existing workspace tests.
- Bundled FIX270-QA-003 is a one-line documentation correction: the FIX-270
  closeout citation in `docs/project-plan.md` now references `6e61f7c`.
- No dependency was added.
