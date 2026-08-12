---
id: O.3
title: JSON Template Lint, sc-compose lint Target, and just lint Integration
phase: N
status: planned
branch: sprint/o-3-template-lint-and-repo-target
worktree: ../sc-compose-worktrees/sprint/o-3-template-lint-and-repo-target
target: integrate/phase-o
---

# Sprint O.3 — JSON template lint and repository lint integration

## Goal

Detect the quoted-scalar-placeholder anti-pattern before a production render,
and make repository-wide lint report the same findings without duplicating the
Rust scanner or creating another Python implementation.

## Dependencies and parallelism

Requires O.1's mode and diagnostic contract. O.3 may run in parallel with O.2
after O.1 merges. O.4 consumes the target and its report contract. O.3 must
not implement the post-render parser; it consumes O.2's shared checker when
fixture-backed render checks are enabled.

## Exact targets

- `crates/sc-compose/src/commands/template_lint.rs`
- `crates/sc-compose/src/commands/sc_lint.rs`
- `.sc/sc-lint/targets/template-contracts.toml`
- `justfile`
- `crates/sc-compose/tests/`
- `tests/fixtures/` template-contract fixtures
- `docs/requirements.md`, `docs/architecture.md`, and lint documentation

## Required work

1. Extend `lint_request` to identify effective JSON templates and source
   locations for literal-quoted scalar placeholders.
2. Detect mode mismatch conservatively across loops, arrays, conditionals,
   includes, comments, and Jinja literals; avoid false positives for explicit
   raw structured JSON paths.
3. Emit stable warning/error codes from the canonical diagnostic schema.
4. Keep warning-only `validate --lint` exit behavior compatible, while making
   auto-mode contract errors and checked-render failures non-zero.
5. Add an allowlisted `template-contracts` target to the sc-compose lint target
   registry. It must enumerate templates and invoke the shared Rust scanner/
   checker rather than duplicate logic in Python or shell.
6. Add JSON and HTML report fields for template, mode, location, diagnostic,
   migration recommendation, and whether a context-backed render was run.
7. Add the target to `just lint`/full-profile aggregation with stable behavior
   when fixture contexts are absent or invalid; capability failure must not be
   reported as a green pass.
8. Preserve the existing external sc-lint boundary and report materialization
   contracts.

## Diagnostic policy

| Situation | Interactive result | Strict/repository result |
| --- | --- | --- |
| unannotated/legacy template | deprecation warning | finding, optionally promotable |
| quoted placeholder in auto | error-level contract finding | failure |
| bare placeholder in auto | clean | clean |
| raw/ambiguous expression | conservative finding or deferred note | never silently claimed safe |
| missing fixture context | explicit capability/configuration result | not a pass |

## Required fixtures and tests

- valid auto scalar/object/array;
- valid legacy quoted scalar with warning;
- auto-mode quoted scalar failure;
- injection payload;
- nested arrays and loops;
- conditional branches;
- explicit raw JSON field;
- include with source location/include chain;
- Jinja comments and literal strings that must not be flagged;
- missing context and invalid fixture configuration;
- same fixtures through `validate --lint` and
  `sc-compose lint --target template-contracts --json`;
- `just lint target=template-contracts` report materialization.

## Deliverables

- source lint rules and diagnostics;
- allowlisted repository target;
- no-duplicate-implementation `just lint` integration;
- JSON/HTML report evidence;
- fixture and CLI tests;
- updated command/help documentation.

## Acceptance criteria

- [ ] `validate --lint` finds the six-template anti-pattern with locations and
      migration guidance.
- [ ] `sc-compose lint --target template-contracts` reports the same rule code
      and does not reimplement the scanner.
- [ ] `just lint` includes the target in the appropriate profile.
- [ ] Missing tools/fixtures are explicit config/capability failures.
- [ ] Existing sc-lint targets and report paths remain unchanged.
- [ ] All workspace and targeted quality checks pass.

## Sc-lint cleanup and QA handoff

Run the complete applicable lint profile on O.3's final commit. Fix minor
findings in place. For remaining findings, create a `fix/` worktree from this
sprint's final commit, grouping same-rule mechanical findings by owner/crate,
keeping length refactors separate, and avoiding one worktree per warning. Send
team-lead the parent commit, fix worktree, class/evidence, tests, and fix
commit. Team-lead creates the PR and routes it to quality-mgr. O.3 is not
complete until QA approval, merge, and revalidation are recorded.

## Validation

```text
cargo test --workspace
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
sc-compose lint --target template-contracts --root . --json
just lint target=template-contracts
git diff --check
```
