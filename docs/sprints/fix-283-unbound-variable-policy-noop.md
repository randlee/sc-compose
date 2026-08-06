---
id: FIX-283
status: complete
branch: fix/283-unbound-variable-policy-noop
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/283-unbound-variable-policy-noop
target: develop
---

# Sprint FIX-283 — `UnknownVariablePolicy.ERROR` Has No Effect On Unbound `{{ var }}`

## Problem

Issue #283: `ComposePolicy(unknown_variable_policy=UnknownVariablePolicy.ERROR)`
has no observable effect on an unbound `{{ var }}`. The reporter's repro
(`probe.xml.j2` referencing `bound_var` and `unbound_var`, only `bound_var`
supplied in `vars_input`) renders `unbound_var` as an empty string and
`compose_file` succeeds — `rendered_text` and `warnings` are byte-identical
across `ERROR`, `IGNORE`, and `WARN`. Expected: `ERROR` should fail the
compose (or at minimum emit an error-severity diagnostic) when a referenced
variable has no binding at render time; `WARN` should at least emit a
distinct unbound-variable warning; `IGNORE`'s current behavior is correct.

## Investigation already done (this is real, not speculative)

Independently reproduced this cycle via direct read of
`crates/sc-composer/src/validation/diagnostics.rs` on `origin/develop`
(confirmed absent through commit `a05f3fd`, current tip). Two existing
policy axes were checked and neither covers this case — this is the root
cause, not a partial regression:

1. **`unknown_variable_policy`** (`push_extra_input_diagnostics`,
   `diagnostics.rs:63-107`, diagnostic code `ERR_VAL_EXTRA_INPUT`) only
   fires for variables **provided** (`vars_input`/`vars_env`) but never
   declared or referenced anywhere — the opposite direction from what the
   issue needs. It never inspects whether a *referenced* variable actually
   has a binding.
2. **`strict_undeclared_variables`** (`undeclared_referenced_variables`,
   `diagnostics.rs:40-56,223-`, diagnostic code `ERR_VAL_UNDECLARED_TOKEN`)
   fires for variables **referenced** in the template body but not
   **declared in frontmatter**. Per the issue's own control test, this
   fires for *both* `bound_var` and `unbound_var` regardless of whether
   `bound_var` actually has a runtime binding — it checks
   declared-in-frontmatter vs. referenced, not bound-at-render vs.
   referenced. It cannot serve as the missing-binding gate the issue wants,
   and firing on `bound_var` (which *is* correctly bound and renders fine)
   makes it unusable as a substitute.

No diagnostic axis exists anywhere in `sc-composer` for the actual case:
*"this variable is referenced in the template and has no value binding at
render time"* (not in `vars_input`, not in `vars_env`, not in
`vars_defaults`, not a loop-local or `{% set %}`-local per the existing
loop/set-scope tracking added for issue #242). That gap is why the renderer
falls through to minijinja's default `Undefined`-renders-empty behavior
with no diagnostic at all, independent of `unknown_variable_policy`.

## Charge to comp

This sprint is scoped as **root-cause + design + implement**, not a
pre-specified fix — the two existing policy axes above are close cousins of
what's needed but neither is it, and bolting the check onto either one
without care risks the same confusion the issue reports (a variable-related
policy field that doesn't do what the reporter expects).

1. **Root-cause and confirm** the investigation above against the current
   codebase (don't just trust this doc — re-verify directly). Look
   specifically for:
   - Where/how minijinja's `Undefined` handling is configured for this
     renderer (is it `Undefined`/`ChainableUndefined`/a custom
     `UndefinedBehavior`?) — this is likely *why* an unbound reference
     silently renders empty instead of erroring at the template-engine
     layer before diagnostics even get a chance to run.
   - Whether `ValidationState` already has enough information (declared
     variables, referenced variables, provided variables, loop/set-scope
     locals from the #242 fix) to compute "referenced but unbound" without
     new tracking, or whether new tracking is required.
2. **Design a general solution**, not a one-off patch for this exact repro.
   Consider explicitly:
   - Is this a new, distinctly-named diagnostic axis (e.g. a new
     `DiagnosticCode` and a new or repurposed policy field), or should
     `unknown_variable_policy` be documented/renamed to make its actual
     scope (extra-provided, not referenced-but-unbound) unambiguous, with
     a *new* field added for the referenced-but-unbound case? Recommend:
     do not silently overload `unknown_variable_policy`'s existing meaning
     — that would just move the confusion, not fix it. A new, clearly-named
     field/diagnostic pair is strongly preferred; document the rationale
     in the Closeout Evidence either way.
   - How "bound" is determined for a referenced variable: presence in
     `vars_input` ∪ `vars_env` ∪ `vars_defaults` ∪ loop-locals ∪
     `{% set %}`-locals ∪ any other value-binding mechanism this repo
     supports. Reuse `ValidationState`'s existing tracking rather than
     re-deriving it if at all possible.
   - Interaction with `strict_undeclared_variables` and
     `unknown_variable_policy`: all three axes must be able to disagree
     independently (e.g. a variable can be declared-in-frontmatter,
     referenced, and still unbound; or referenced-and-bound but
     undeclared) — do not conflate them.
   - Generalizing "similar issues": are there other silent-success paths in
     the render pipeline where a template construct fails soft (renders
     empty/wrong) instead of surfacing a diagnostic? A brief audit note in
     Closeout Evidence (even if the answer is "none found") is expected,
     not a full separate sprint.
3. **Implement** per the design, with the standard two-commit red→green
   regression-test process (red commit landed first and confirmed failing,
   then the fix in a second commit) — this is now standing process for
   this fuzz/backlog queue.

## Required tests

1. The exact repro from the issue: `ERROR` policy + unbound `{{ var }}` +
   `vars_input` missing that key → compose fails or emits an
   error-severity diagnostic distinguishable from the bound-variable case.
2. `WARN`-equivalent policy setting emits a distinct unbound-variable
   warning (not silently identical to `IGNORE`).
3. `IGNORE`-equivalent policy setting preserves current (correct) silent
   behavior — explicit regression guard that this sprint does not change
   `IGNORE`'s output.
4. A bound variable (present in `vars_input`) under the new
   error/warn/ignore settings is never flagged as unbound — regression
   guard against the `strict_undeclared_variables` false-positive-on-bound
   behavior the issue's control test exposed.
5. Interaction test: a variable declared in frontmatter but unbound at
   render time vs. a variable not declared in frontmatter but bound at
   render time — confirm the new check and `strict_undeclared_variables`
   fire independently and correctly on each.
6. Loop-local and `{% set %}`-local variables (per #242) are correctly
   treated as bound, not flagged as unbound.

## Acceptance criteria

- `cargo test --workspace` passes, including all new/updated tests above.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #283's exact repro now fails closed (or emits a clear error-severity
  diagnostic) under the error-equivalent policy setting, and emits a
  distinct warning under the warn-equivalent setting.
- No new dependency added to `Cargo.toml`.
- Sprint doc Closeout Evidence records: the confirmed root cause (including
  the minijinja `Undefined`-handling finding), the design decision (new
  field vs. reused field, and why), exact fix commit(s), validation
  results, and the brief similar-issues audit note.

## References

- Issue #283: https://github.com/randlee/sc-compose/issues/283
- `crates/sc-composer/src/validation/diagnostics.rs` (`push_extra_input_diagnostics`,
  `undeclared_referenced_variables`, lines ~40-107, ~223-)
- `crates/sc-composer/src/validation/mod.rs` (`ValidationState`, declared/referenced
  variable tracking, loop/set-scope locals from issue #242)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`) — issue #283 predates fuzz
  round 2 but is part of the same backlog sweep

## Closeout Evidence

### Confirmed root cause

- `Renderer` configures Minijinja's escaping and filters but does not install
  an undefined-value handler. Minijinja therefore renders an unresolved
  `Undefined` value as an empty string in the normal composition path.
- Validation previously had two nearby but different axes: `ERR_VAL_EXTRA_INPUT`
  applies to supplied names that are neither declared nor referenced, while
  `ERR_VAL_UNDECLARED_TOKEN` applies to referenced names missing from
  frontmatter. Neither compares referenced paths with the merged runtime
  context, so an unbound reference reached Minijinja and disappeared without
  a diagnostic.
- `compose_with_observer` already fails before rendering when validation has
  errors, so the missing piece was the validation diagnostic, not a renderer
  workaround.

### Design and implementation

- Added `ERR_VAL_UNBOUND_VARIABLE` as a distinct validation code. It checks
  referenced paths against the merged defaults/environment/input context,
  including nested object paths, while preserving loop locals, `{% set %}`
  locals, and built-in render-context names.
- Added `ComposePolicy.unbound_variable_policy`. It uses the same
  `UnknownVariablePolicy` severity values but is a separate axis; `None`
  inherits `unknown_variable_policy` so existing Python callers using
  `unknown_variable_policy=ERROR` get the issue's expected fail-closed result.
  An explicit value allows extra-input and unbound-reference behavior to
  disagree. The CLI maps its existing variable mode to both axes, and the
  Python binding exposes the new override.
- Red regression commit: `d04118d`.
- Green implementation commit: `8cc017d`.

### Similar silent-success audit

- Optional variables inside `{% if ... %}` can still intentionally suppress a
  section when the policy is `ignore`; warn/error modes now surface the
  missing reference before rendering rather than silently accepting it.
- `dict.get("missing")` without a default remains an intentional Minijinja
  undefined-to-empty behavior. The existing renderer contract supports this
  form; callers that need a value must use `dict.get("key", default)`, and a
  future runtime-undefined audit would need expression-level tracking to
  distinguish it from a legitimate optional lookup.
- Loop-body field paths are scope-correct for the iterable and local names,
  but this validation pass cannot prove every field exists on every runtime
  element. Required nested paths or format-specific parsing remain the guard
  for that case. JSON/XML/YAML/TOML output paths already parse/shape-check
  their rendered results; raw text and Markdown remain intentionally
  syntax-agnostic.
- No new dependency was added.

### Validation

- `cargo test --workspace` — PASS
- `cargo fmt --all --check` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS
- `git diff --check` — PASS
