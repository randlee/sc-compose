---
id: FIX-385
title: "template-init followed by render silently corrupts round-trip JSON via double-quoting auto-escape"
status: assigned
branch: fix/385-template-init-json-round-trip
worktree: ../sc-compose-worktrees/fix/385-template-init-json-round-trip
target: develop
---

## Root Cause

`crates/sc-compose/src/commands/template_init.rs` performs literal substring
replacement on a concrete `*.json` file, preserving the value's original
surrounding quote characters verbatim (e.g.
`"worktree_path": "{{ worktree_path }}"`).

`crates/sc-composer/src/renderer.rs`'s `legacy_auto_escape_callback` applies
`AutoEscape::Json` to any template whose stripped name ends in `.json`, which
unconditionally wraps every substituted string value in its own quotes.

Running `template-init` on a JSON file with a string value, then `render`ing
the generated template with the same value, does not round-trip:

```
template-init output: "worktree_path": "{{ worktree_path }}"
render output:        "worktree_path": "\"/tmp/wt\""
```

This is invalid, double-quoted JSON, produced with exit 0 and zero
diagnostics.

## Fix design

In `template_init.rs`, when the target file's stripped extension is `json`,
detect and consume the surrounding quote characters as part of the
replacement span so the generated token is bare
(`"worktree_path": {{ worktree_path }}`), matching the auto-escape contract
already exercised by `renderer.rs`'s existing unit tests. Do not change
`renderer.rs`'s auto-escape behavior — it is correct for the bare-placeholder
contract; `template_init.rs` is the side with the mismatched assumption.

## Required changes / tests

1. `crates/sc-composer/src/renderer.rs::renderer_json_auto_escape_does_not_double_quote_a_pre_quoted_string_placeholder`
   (already added as a RED regression test) goes GREEN without modification
   to its assertions.
2. Add a `template_init.rs` unit or CLI test asserting that running
   `template-init` on a concrete `*.json` file containing a quoted string
   value, then `render`ing the generated template with the same value,
   reproduces the original document byte-for-byte.
3. `cargo test --workspace`, `cargo fmt --all --check`,
   `cargo clippy --all-targets --all-features -- -D warnings`: PASS.

## Out of scope

- `crates/sc-compose/src/cli/schema.rs` / `pass_input.rs` clap-error-vs-`--json`
  bypass (issue #386, FUZZ-002/FUZZ-003) — separate root cause, separate branch.
- Any change to the JSON auto-escape contract for hand-written (non-generated)
  templates.

## Acceptance criteria

- Round-trip regression test passes; existing renderer auto-escape tests are
  unaffected.
- `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`: PASS.
- Closeout Evidence records the fix commit(s).
- Planning index gate: `docs/project-plan.md` includes the sprint entry
  before closeout.

## References

- Issue #385: https://github.com/randlee/sc-compose/issues/385
- Fuzz finding FUZZ-001, campaign report `site/reports/20260811-3-fuzz-report.json`
- `docs/requirements.md` FR-8a

## Priority

Fuzz-discovered production bug; dispatched immediately, comp assigned as the
harder of the two 2026-08-11 fuzz-campaign fixes.
