---
id: FIX-SIMP-001
title: "Duplicated .j2/.jinja2/.jinja suffix-stripping heuristic between template_init.rs and renderer.rs"
status: complete
branch: fix/simp-001-shared-suffix-helper
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/simp-001-shared-suffix-helper
target: crates/sc-composer/src/renderer.rs, crates/sc-compose/src/commands/template_init.rs
---

## Root Cause

Two independent copies of the same `[".j2", ".jinja2", ".jinja"]`
suffix-stripping heuristic exist:

- `crates/sc-composer/src/renderer.rs::legacy_auto_escape_callback` (private,
  lines ~54-68) — strips the suffix before checking the remaining extension
  for HTML/XML auto-escape decisions.
- `crates/sc-compose/src/commands/template_init.rs::is_json_template_path`
  (added in FIX-385) — strips the same suffix list before checking for a
  `.json` extension.

Flagged as SIMP-001 (Minor, non-blocking) during FIX-385 QA
(`docs/sprints/fix-385-template-init-json-round-trip.md` / QA verdict):
different callers/purposes today, but future drift risk if a third caller
needs the same heuristic and only one copy gets updated.

## Fix Design

Lift the suffix-stripping into one shared helper in `sc-composer` (e.g.
`sc_composer::template_ext::strip_template_suffix(name: &str) -> &str`, or a
similarly-named small pub function/module), and have both
`legacy_auto_escape_callback` and `is_json_template_path` call it. Per the
crate boundary rules, `sc-compose` may depend on `sc-composer` only, so the
shared helper must live in `sc-composer` and be exported for `sc-compose` to
use — not the other direction.

## Required Changes / Tests

- Add the shared helper to `crates/sc-composer` (new small module or existing
  one, whichever fits current conventions) and export it.
- Update `legacy_auto_escape_callback` (renderer.rs) to use it.
- Update `is_json_template_path` (template_init.rs) to use it.
- Keep existing behavior byte-for-byte identical — this is a pure
  refactor, not a behavior change. Existing renderer/template_init tests
  must continue to pass unmodified; add one small unit test for the new
  shared helper itself if not already covered.
- Add a `docs/project-plan.md` Follow-on Fix Sprint entry for this sprint.

## Out of Scope

- Any change to auto-escape behavior, JSON-template detection behavior, or
  which extensions are recognized.
- Any change to `sc-composer`'s public API surface beyond exporting this one
  helper.

## Acceptance Criteria

1. Only one copy of the `.j2`/`.jinja2`/`.jinja` suffix-stripping logic
   exists in the codebase; both callers use it.
2. No behavior change: existing tests for both `renderer.rs` and
   `template_init.rs` pass unmodified.
3. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
   and `cargo test --workspace` are clean.
4. `docs/project-plan.md` gets a Follow-on Fix Sprint entry for this sprint.

## References

- FIX-385 QA verdict, finding SIMP-001
- `docs/sprints/fix-385-template-init-json-round-trip.md`
- Boundary Rules 1-2 (`CLAUDE.md`): sc-composer stays a pure library;
  sc-compose may depend on sc-composer only.

## Closeout Evidence

- Implementation commits: `26b6472` and `60c6df6`.
- Exported `sc_composer::strip_template_suffix` is the sole copy of the
  recognized template-suffix stripping logic; both renderer auto-escape and
  template-init JSON detection call it.
- Added unit coverage for all supported suffixes and the unchanged-name path.
- Validation passed: `cargo test --workspace`, `cargo fmt --all --check`, and
  `cargo clippy --all-targets --all-features -- -D warnings`.

## Priority

Minor, non-blocking cleanup — no release impact.
