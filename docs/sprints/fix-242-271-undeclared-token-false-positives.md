---
id: FIX-242-271
title: "ERR_VAL_UNDECLARED_TOKEN false positives on subscripts, loop builtins, operators, filter names, and {% set %} locals"
status: complete
branch: fix/242-undeclared-token-false-positives
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/242-undeclared-token-false-positives
target: develop
---

# FIX-242-271: Undeclared-token scanner false positives

Issues: https://github.com/randlee/sc-compose/issues/242,
https://github.com/randlee/sc-compose/issues/271
Branch: `fix/242-undeclared-token-false-positives`
Base: `develop` @ `97c5a07`

## Problem

Two fuzz-round-2 findings are both root-caused in the exact same scanner
function, `collect_identifiers` in `crates/sc-composer/src/discovery.rs`, and
are combined into one fix per team-lead's standing "same worktree for
intersecting logic" instruction:

- **#242**: `ERR_VAL_UNDECLARED_TOKEN` false positives on numeric
  subscript/slice literals (`items[0]`, `items[1:2]`), the standard Jinja
  loop-builtin namespace (`loop.index0`, etc, when a real loop scope *is*
  active — the "outside a loop" case was already handled correctly), and bare
  operator fragments (`-`, `..`) split out as their own candidate tokens.
- **#271**: `--strict` misclassifies Jinja filter names applied via `|` (`e`,
  `safe`, `lower`, ...) and `{% set %}`-bound local variables as undeclared
  external references, making `--strict` unusable on nearly any real
  production template.

## Root cause (confirmed via read-only review + working prototype, team-lead,
2026-08-06)

All four sub-bugs trace to `collect_identifiers` in
`crates/sc-composer/src/discovery.rs` (current develop, ~line 213 onward):

```rust
let masked_expression = mask_quoted_literals(expression);
for candidate in masked_expression.split(|character: char| {
    !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
}) {
    if candidate.is_empty() || KEYWORDS.contains(&candidate) {
        continue;
    }
    let root = candidate.split('.').next().unwrap_or(candidate);
    if bound_names.contains(root) || is_loop_context_name(candidate, &masked_expression, scopes) {
        continue;
    }
    if let Ok(variable) = VariableName::new(candidate) {
        tokens.insert(variable);
    }
}
```

1. **Subscripts/slices/operators (#242)**: `[`, `]`, `:` are treated as plain
   separators, so `items[0]` splits into candidates `items` and `0`; `0` is
   not a keyword, passes through, and gets flagged. `-` and `.` are kept as
   valid "identifier" characters (not split points), so a lone `-` or `..`
   surrounded by whitespace survives as its own candidate and also gets
   flagged, even though it is an operator, not a reference.
2. **Loop builtins outside a loop, already correct**: `is_loop_context_name`
   correctly returns `false` when `scopes.is_empty()` (no active `for`), so
   `loop.last` used outside a loop is correctly flagged today — this behavior
   must be preserved exactly (see Required fixes item 4).
3. **Filter names (#271)**: `|` is a plain separator with no special
   handling. `{{ x | e | lower }}` splits into `x`, `e`, `lower` — the filter
   names `e`/`lower` are checked only against `KEYWORDS` (which has no filter
   names) and get flagged as undeclared references.
4. **`{% set %}` locals (#271)**: only `{% for %}` gets scope tracking
   (`LoopScope`, pushed/popped around `endfor`). `{% set greeting = ... %}`
   falls through to plain `collect_identifiers` on the whole statement, which
   flags `greeting` itself (the assignment target) as an undeclared
   reference, and later uses of `{{ greeting }}` are also flagged since
   `greeting` is never registered as a bound name anywhere.

Verified with a working prototype in this worktree (reverted before dispatch
— implementation belongs to comp per standing process) against all four
repros from both issues, plus the existing regression suite. One existing
test, `render::validate_strict_rejects_loop_context_outside_for_and_lookalikes`
(`crates/sc-compose/tests/cli/render.rs`), depends on `is_loop_context_name`
seeing an empty `scopes` slice when no loop is active — see Required fixes
item 4 for the exact interaction to preserve.

## Scope decision

Fix all four sub-bugs in `collect_identifiers` and its call site in
`discover_tokens_with_delimiters`, without changing the scanner's overall
architecture (still a single forward pass over `{{ }}`/`{% %}` delimiters, no
new dependencies, no move to a real Jinja AST parser — that would be a much
larger change than four narrowly-scoped false-positive fixes need).

`{% set %}` scoping is intentionally loose: bind the target name at a single
persistent base scope for "the rest of the scan" rather than modeling Jinja's
real block-scoping rules (e.g. a `set` inside a `for` loop going out of scope
at `endfor`). This mirrors the existing scanner's own looseness (it does no
scope-exit tracking for `if`/`block` either) and is a token-discovery
heuristic, not a runtime evaluator — false negatives here (failing to flag a
token that's technically out of scope) are an acceptable tradeoff against the
false positives this fix removes. Only the simple assignment form
(`{% set name = value %}`, bare identifier target) is handled; namespace
targets (`{% set ns.attr = value %}`) and the block form
(`{% set name %}...{% endset %}`) are out of scope — they fall through to
today's (already-buggy-for-namespaces, pre-existing) behavior unchanged.

## Required fixes

1. In `discover_tokens_with_delimiters`, initialize `scopes` with one
   always-present base `LoopScope` (`vec![LoopScope::default()]`) instead of
   an empty `Vec`. This base scope holds `{% set %}`-bound names for the rest
   of the scan and is never popped.
2. Add a `parse_set_scope(expression, scopes, tokens) -> Option<String>`
   function (mirroring `parse_for_loop_scope`'s shape): matches
   `set NAME = VALUE` where `NAME` is a bare identifier (ASCII
   alphanumeric/`_` only — reject and fall through to `collect_identifiers`
   unchanged for anything else, e.g. `ns.attr`), calls
   `collect_identifiers(VALUE, scopes, tokens)` on the right-hand side so
   real references inside the assigned expression are still discovered, and
   returns `Some(NAME)`. Wire it into the statement-handling `match` in
   `discover_tokens_with_delimiters`: check it after the existing
   `parse_for_loop_scope`/`endfor` arms and before the fallback
   `collect_identifiers` call; on `Some(name)`, insert `name` into
   `scopes[0].bound_names` (do not call `collect_identifiers` on the
   statement itself, since `parse_set_scope` already handled the RHS).
3. Guard the existing `endfor` arm (`scopes.pop()`) to only pop when
   `scopes.len() > 1`, so an unbalanced/malformed `endfor` can never pop the
   persistent base scope out from under `scopes[0]` usage.
4. Update `is_loop_context_name`'s "not inside a loop" check from
   `scopes.is_empty()` to `scopes.len() <= 1`, since `scopes` is no longer
   ever empty after fix 1 — `scopes.len() <= 1` means only the persistent
   base scope is present, i.e. no active `for` loop. This is required to keep
   `render::validate_strict_rejects_loop_context_outside_for_and_lookalikes`
   passing (it asserts `loop.last` outside any `for` loop is still flagged as
   undeclared).
5. In `collect_identifiers`: mask filter names before the candidate split.
   Add `mask_filter_names(expression: &str) -> String` that scans for `|`,
   skips any following whitespace, then blanks (replaces with spaces) the
   following identifier run (`is_ascii_alphanumeric() || '_'`) — leaving
   everything else (including filter call arguments) untouched so real
   variable references inside filter arguments, e.g. `x | default(other)`,
   are still discovered. Apply as
   `mask_filter_names(&mask_quoted_literals(expression))` before the existing
   split loop.
6. In `collect_identifiers`'s candidate loop, after the existing
   empty/keyword check, add: skip any candidate with no ASCII alphabetic
   character at all (`!candidate.chars().any(|c| c.is_ascii_alphabetic())`).
   This removes bare numeric subscript/slice fragments (`0`, `1`, `2`) and
   bare operator fragments (`-`, `..`) as candidates, without affecting real
   identifiers (which always contain at least one letter, including
   kebab-case names like `task-id`).
7. Mandatory two-commit red→green process: commit 1 adds failing tests
   reproducing all four sub-bugs exactly (see Required tests below) that fail
   against current `develop`; commit 2 applies the fix and all tests go
   green.
8. Required tests (add to `crates/sc-composer/src/discovery.rs`'s existing
   `#[cfg(test)] mod tests`, plus a CLI-level regression if appropriate given
   existing coverage in `crates/sc-compose/tests/cli/`):
   - `{{ items[0] }} {{ items[1:2] }}` inside a template with `items`
     declared: only `items` is discovered, not `0`/`1`/`2`.
   - `{% for it in items %}{{ it }}{{ loop.index0 }}{% endfor %}`: only
     `items` is discovered (matches issue #242's exact repro).
   - `{{ a - b }}`: `a` and `b` are discovered (real references), `-` is not.
   - `{{ x | e }}` and `{{ x | e | lower }}`: only `x` is discovered, not
     `e`/`lower`.
   - `{% set greeting = 'Hi ' + name %}{{ greeting | e | lower }}`: only
     `name` is discovered — `greeting` (the set-local) and `e`/`lower`
     (filters) are not.
   - Regression: `loop.last` used outside any `for` loop is still discovered
     as an undeclared token (must NOT regress
     `render::validate_strict_rejects_loop_context_outside_for_and_lookalikes`).
   - Regression: `x | default(other_var)` still discovers `other_var` (filter
     argument references are not masked away).
9. Re-run all repro commands from both issue #242 and issue #271 verbatim and
   confirm they no longer report the false-positive tokens.
10. Record the fix commit(s) and validation results in this doc's Closeout
    Evidence section before requesting QA.

## Out of scope (do not implement)

- Replacing the regex/split-based scanner with a real Jinja AST parser.
- Namespace `{% set ns.attr = value %}` target scoping.
- Block-form `{% set name %}...{% endset %}` scoping.
- Precise block-exit scoping for `{% set %}` locals (e.g. going out of scope
  at `endfor`/`endif`/`endblock`) — the persistent-base-scope approach is
  intentionally loose, per Scope decision above.
- Renaming `ERR_VAL_UNDECLARED_TOKEN`/`ERR_VAL_MISSING_FRONTMATTER` to a
  `WARN_` prefix family (issue #242 mentions this as a secondary, independent
  suggestion) — file a separate issue if still wanted after this fix lands,
  since it's an unrelated cosmetic/API-shape change.

## Acceptance criteria

- `cargo test --workspace` passes with 0 failures, including all new tests
  listed above and the existing
  `render::validate_strict_rejects_loop_context_outside_for_and_lookalikes`
  test (unregressed).
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #242's exact repro
  (`{% for it in items %}{{ it }}{{ loop.index0 }}{% endfor %}` +
  `{{ items[0] }} {{ items[1:2] }}`) produces zero `ERR_VAL_UNDECLARED_TOKEN`
  diagnostics for `0`, `1`, `2`, or `loop.index0`.
- Issue #271's exact repros (`{{ x | e }}` and
  `{% set greeting = 'Hi ' + name %}{{ greeting | e | lower }}`) produce zero
  `ERR_VAL_UNDECLARED_TOKEN` diagnostics for `e`, `lower`, or `greeting`
  (`name` is still correctly flagged — it's a genuine external reference).
- Sprint doc Closeout Evidence section records exact fix commit(s) and
  validation results before requesting QA.

## References

- Issue #242: https://github.com/randlee/sc-compose/issues/242
- Issue #271: https://github.com/randlee/sc-compose/issues/271
- `crates/sc-composer/src/discovery.rs` (`collect_identifiers`,
  `is_loop_context_name`, `parse_for_loop_scope`, `discover_tokens_with_delimiters`)
- `crates/sc-compose/tests/cli/render.rs`
  (`validate_strict_rejects_loop_context_outside_for_and_lookalikes` — must
  not regress)
- Fuzz round 2 report, 2026-08-06 (adversarial fuzzing of `sc-compose`
  against production templates in `atm-core`)

## Closeout Evidence

- Red regression commit: `97a7cef` (`test: reproduce undeclared token false
  positives`). It adds all seven required discovery regressions; the pre-fix
  run fails on numeric subscript fragments, the binary operator fragment, and
  filter/set-local false positives.
- Green implementation commit: `d3e2c04` (`fix: ignore false undeclared
  template tokens`). It adds persistent set-local scope, safe loop-scope
  handling, filter-name masking that preserves filter arguments, and
  alphabetic-candidate filtering for numeric/operator fragments.
- Exact CLI repros pass with exit code `0`, `valid: true`, and empty
  diagnostics under `--strict --json`:
  `issue-242.md.j2` covers the loop/subscript/slice case,
  `issue-271.md.j2` covers `{% set %}` locals and filters, and
  `issue-271-filter-arg.md.j2` confirms `default(other_var)` retains the
  genuine argument reference. The committed unit tests in
  `crates/sc-composer/src/discovery.rs` provide the durable equivalent cases;
  those three named manual CLI repro fixtures are not committed to the
  repository.
- Known limitation: dotted or namespaced filter names such as `ns.custom`
  remain outside the scope of this sprint and are not handled by
  `mask_filter_names`; this is retained as a documented follow-up gap rather
  than a functional scope expansion.
- `cargo fmt --all --check` — PASS.
- `cargo test --workspace` — PASS (all workspace tests passed).
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS.
- `git diff --check` — PASS.
