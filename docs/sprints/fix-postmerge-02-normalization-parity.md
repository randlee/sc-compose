---
status: complete
branch: fix/checked-render-format-normalization-parity
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/checked-render-format-normalization-parity
---

# FIX-POSTMERGE-02: Template-suffix normalization not generalized (FUZZ-001)

## Source

quality-mgr adjudication of comp's post-merge critical review, develop @
`1b0f9a9` (`comp-postmerge-findings-adjudication.txt`), item 2 (HIGH,
independently confirmed via grep).

## Problem

`renderer.rs` and `validation/diagnostics.rs` used the old single-strip,
case-sensitive `strip_template_suffix` helper instead of the stacked-suffix,
case-insensitive JSON-path-detection logic already added to `render_check.rs`.
That made JSON escape-mode selection and validation diagnostics diverge from
checked rendering for paths such as `foo.JSON.j2` or `foo.json.j2.j2`.

## Required Fix

- Generalize the suffix/case-insensitive JSON-detection helper into one shared
  implementation.
- Converge `render_check.rs`, `renderer.rs`,
  `validation/diagnostics.rs`, `template_lint.rs`, and `template_init.rs` on
  that implementation.
- Add regression tests proving renderer and validation agree with checked
  rendering for stacked-suffix and case-insensitive paths.

## Acceptance Criteria

- All JSON-path callers use the shared, generalized detection helper.
- Regression tests cover `foo.JSON.j2`, `foo.json.J2`, and
  `foo.json.j2.j2` without colliding on case-insensitive filesystems.
- `cargo test --workspace`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo fmt --all --check` pass.

## References

- `/Users/randlee/.atm/.config/atm/share/sc-compose/comp-postmerge-findings-adjudication.txt`
- `crates/sc-composer/src/renderer.rs`
- `crates/sc-composer/src/validation/diagnostics.rs`
- `crates/sc-composer/src/render_check.rs`
