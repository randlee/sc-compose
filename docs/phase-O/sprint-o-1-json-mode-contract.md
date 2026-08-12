---
id: O.1
title: JSON Escape Mode Contract and Safe Renderer Compatibility
phase: O
status: planned
branch: sprint/o-1-json-mode-contract
worktree: ../sc-compose-worktrees/sprint/o-1-json-mode-contract
target: integrate/phase-o
---

# Sprint O.1 — JSON escape mode contract and safe renderer compatibility

## Goal

Define and implement the two JSON interpolation modes without weakening the
FIX-272 injection fix. Existing unannotated JSON templates must have a safe
compatibility path; newly generated or migrated templates must be able to use
secure auto escaping explicitly.

## Dependencies and parallelism

No implementation sprint dependency. Start from the phase parent selected by
team-lead on `integrate/phase-o` after plan approval. O.1 blocks O.2 and O.3
because both consume its effective-mode and diagnostic contracts. O.1 may run
in parallel with unrelated work outside the JSON renderer, but no other Phase O
sprint may implement a second mode resolver or escape algorithm.

## Exact targets

- `crates/sc-composer/src/renderer.rs`
- `crates/sc-composer/src/frontmatter/model.rs`
- `crates/sc-composer/src/frontmatter/parser.rs`
- `crates/sc-composer/src/diagnostics/schema.rs`
- `crates/sc-compose/src/commands/template_init.rs`
- `crates/sc-compose/src/cli/schema.rs`
- `crates/sc-compose/src/render_request/request.rs`
- `crates/sc-composer/src/renderer.rs` tests
- `crates/sc-compose/tests/cli/templates.rs`
- `docs/requirements.md`
- `docs/adrs/0019-json-render-contract.md` (reserved; acceptance gate)

## Required work

1. Add the typed `JsonEscapeMode` defined by the phase plan's authoritative
   checked-render contract, with `Legacy` and `Auto` values.
2. Resolve mode using CLI override, root frontmatter, then 1.4.1 compatibility
   default `legacy`.
3. Restrict mode semantics to effective JSON templates using the existing
   suffix convention; diagnose format/mode mismatches instead of silently
   applying JSON rules to another format.
4. Implement legacy string-content escaping. It must escape JSON string
   contents but must not add surrounding quotes already present in the source.
5. Keep auto mode's complete-value minijinja JSON escaping and injection
   protection unchanged.
6. Make `template-init` generate JSON templates with
   `json_escape_mode: auto` and bare placeholders.
7. Add stable diagnostic codes for compatibility/deprecation and invalid mode
   usage; register them in the canonical schema.
8. Document the source contract and migration examples in requirements/help.

## Explicit contract

```text
legacy: "{{ value }}" -> JSON-escaped string contents, no added quotes
auto:   {{ value }}     -> complete JSON value, renderer owns string quotes
```

Legacy is not raw interpolation. A hostile value such as
`x", "injected": true` must remain one JSON string in both modes.

## Required tests

- auto string round-trip with quote, slash, newline, control character, and
  Unicode;
- auto injection cannot create a second object key;
- auto object, array, boolean, number, and null values retain type;
- legacy manually quoted string round-trip;
- legacy hostile string cannot inject syntax;
- legacy non-string in a quoted-string position has a stable diagnostic;
- mode precedence: CLI > frontmatter > compatibility default;
- warning is emitted once per legacy template;
- invalid mode and non-JSON mode use are diagnosed;
- template-init emits auto mode and bare JSON placeholders;
- template-init followed by render is a semantic JSON round-trip;
- existing HTML/XML/CDATA/Turtle renderer tests remain unchanged.

The canonical Rust signature is:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum JsonEscapeMode {
    Legacy,
    Auto,
}
```

## Deliverables

- typed mode and resolver;
- safe legacy and auto behavior;
- stable diagnostic schema entries;
- template-init alignment;
- requirements/help/migration documentation;
- unit and CLI regression tests.

## Acceptance criteria

- [ ] Existing six-template source shape can be rendered safely in legacy mode.
- [ ] Auto mode remains safe for the FIX-272 injection value.
- [ ] No raw/unescaped legacy path exists.
- [ ] New template-init JSON output selects auto mode and renders valid JSON.
- [ ] CLI/frontmatter/default precedence is tested.
- [ ] No non-JSON renderer behavior changes.
- [ ] All required tests and workspace quality checks pass.
- [ ] ADR-0019 is accepted before implementation handoff; this sprint does
      not dispatch source work while the Phase O design-acceptance gate is open.

## Sc-lint cleanup and QA handoff

Run the applicable sc-lint targets against the final O.1 commit. Fix minor
findings in this sprint. For remaining findings, create a `fix/` worktree from
this sprint's final commit, grouping by independent rule class (for example,
one renderer crate constant-string group, not one worktree per finding; keep
length refactors separate). Send team-lead the worktree, parent commit, finding
class, evidence, tests, and fix commit. Team-lead creates the PR and routes it
to quality-mgr. O.1 is complete only after required fix PRs are QA-approved,
merged, and O.1 is revalidated on the merged parent.

## Validation

```text
cargo test --workspace
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```
