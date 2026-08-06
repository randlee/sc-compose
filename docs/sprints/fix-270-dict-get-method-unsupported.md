---
id: FIX-270
title: "Jinja dict .get(key, default) method unsupported by render engine"
status: in-progress
branch: fix/270-dict-get-method-unsupported
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/270-dict-get-method-unsupported
target: develop
---

# FIX-270: Jinja `dict.get(key, default)` method unsupported by render engine

Issue: https://github.com/randlee/sc-compose/issues/270
Branch: `fix/270-dict-get-method-unsupported`
Base: `develop` @ `97c5a07`

## Problem

The render engine does not support the standard Jinja2/Python dict method
`.get(key, default)`. This crashes real production templates
(`templates/smoke-report/smoke.md.j2`, `smoke-thorough.md.j2` in `atm-core`)
precisely when a row's verdict is not `PASS` — exactly the "Deviations"
scenario those templates exist to report. Minimal repro:

```
{{ row.get("k", "n/a") }}
```

fails with `template rendering failed: unknown method: map has no method
named get (in inline:1)`, exit code 2.

## Root cause (confirmed via read-only review, team-lead, 2026-08-06)

`sc-composer` depends on `minijinja = "2.12"` (resolved to `2.18.0` in
`Cargo.lock`). Checked minijinja 2.18.0's source directly
(`~/.cargo/registry/src/.../minijinja-2.18.0/src`): there is no built-in
`.get()` method implementation for `ValueKind::Map` anywhere in the crate —
confirmed by grepping `src/value/mod.rs` and `src/value/object.rs` for a
`"get"` method-name match; none exists. This is a genuine minijinja engine
gap, not an sc-compose regression.

minijinja provides exactly the extension point needed:
`Environment::set_unknown_method_callback` (`src/environment.rs:322-350`),
documented as being invoked with `State`, the `Value`, method name, and args
whenever a method call would otherwise raise `ErrorKind::UnknownMethod` —
its own doc example implements a compatible `.items()` shim this same way.
minijinja's docs also point at a separate `minijinja-contrib` crate
(`pycompat` module) that implements a broader set of Python-dict/list
compatibility methods including `.get()`, but pulling in a new external
dependency is a larger-blast-radius change than this bug needs.

## Scope decision

Implement `.get(key, default=None)` directly via
`Environment::set_unknown_method_callback` in
`crates/sc-composer/src/renderer.rs`'s `configure_environment`, scoped
narrowly to `ValueKind::Map` values and the method name `"get"`:
- 1 argument (`key`): return the map's value for `key`, or minijinja's
  `Undefined` if absent (matches Python/Jinja2 `dict.get(key)` semantics).
- 2 arguments (`key`, `default`): return the map's value for `key`, or
  `default` if absent.
- Any other combination (wrong arity, non-Map value, unrecognized method
  name): fall through to `Err(Error::from(ErrorKind::UnknownMethod))`,
  preserving today's error behavior for everything else.

Do NOT add `minijinja-contrib` as a new dependency for this fix — it would
pull in a much broader Python-compatibility surface (multiple new methods,
new transitive dependency, new maintenance surface) to fix one specific,
narrowly-scoped method gap. This mirrors the FIX-268 precedent of preferring
a small project-owned callback over a broader library mechanism when only
one narrow behavior is actually needed.

## Required fixes

1. In `crates/sc-composer/src/renderer.rs`, add a `map_get_unknown_method_callback`
   (or similarly named) function implementing the semantics above, and wire
   it into `configure_environment` via `env.set_unknown_method_callback(...)`.
2. Follow the mandatory two-commit red→green regression-test process:
   commit 1 adds a failing test reproducing the exact issue #270 repro
   (`{{ row.get("k", "n/a") }}` against `{"row": {"k": "v"}}`, asserting
   output `"v"`) that fails before the fix. Commit 2 applies the fix and the
   test goes green.
3. Add regression coverage for: `.get(key)` with no default on a present key,
   `.get(key)` with no default on a missing key (must render as empty/undefined,
   not error), `.get(key, default)` on a missing key (must render `default`),
   and `.get()` called on a non-map value (must still raise the original
   `UnknownMethod` error, not silently succeed).
4. Re-run the exact production-template repro from the issue
   (`templates/smoke.md.j2` + a vars file with a non-PASS row) and confirm it
   now renders successfully instead of erroring.
5. Record the fix commit(s) and validation results in this doc's Closeout
   Evidence section before requesting QA.

## Out of scope (do not implement)

- Adding `minijinja-contrib` or its `pycompat` module as a new dependency.
- Implementing other Python dict/list compatibility methods (`.items()`,
  `.keys()`, `.values()`, `.pop()`, etc.) not required by this issue's repro
  — file a separate issue if those are independently needed.

## Acceptance criteria

- `cargo test --workspace` passes, including new regression tests listed
  above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #270's exact repro (`row.get("k", "n/a")` against `{"row": {"k":
  "v"}}`) renders `v`.
- The real production repro (`templates/smoke.md.j2` with a non-PASS row)
  renders successfully with exit code 0.
- Sprint doc Closeout Evidence section records exact fix commit(s) and
  validation results before requesting QA.

## References

- Issue #270: https://github.com/randlee/sc-compose/issues/270
- `crates/sc-composer/src/renderer.rs` (`configure_environment`,
  `legacy_auto_escape_callback`, `format_sc_compose_markup` — precedent for
  small project-owned Environment callbacks)
- minijinja 2.18.0 `src/environment.rs:322-350`
  (`set_unknown_method_callback`)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

_Pending — to be filled in by comp on completion._
