---
id: O.2
title: Checked Render API, JSON Parser Gate, and ATM-core Contract
phase: N
status: planned
branch: sprint/o-2-checked-render-contract
worktree: ../sc-compose-worktrees/sprint/o-2-checked-render-contract
target: integrate/phase-o
---

# Sprint O.2 — checked render API, JSON parser gate, and ATM-core contract

## Goal

Ensure a JSON template is parsed after rendering and before emission, and expose
a machine-readable result that ATM-core can trust for the exact context it is
about to send or cache.

## Dependencies and parallelism

Requires O.1's merged mode resolver, safe escaping, and diagnostic codes. After
that merge, O.2 may run in parallel with O.3. O.4 depends on both O.2 and O.3.
O.2 must not duplicate mode selection or JSON escaping from O.1.

## Exact targets

- `crates/sc-composer/src/` checked-render/output-validation module
- `crates/sc-composer/src/diagnostics/`
- `crates/sc-compose/src/commands/compose.rs`
- `crates/sc-compose/src/commands/compose_render.rs`
- `crates/sc-compose/src/commands/compose_output.rs`
- `crates/sc-compose/src/cli/`
- `crates/sc-compose/tests/`
- `docs/requirements.md` FR-7, FR-8, and FR-8a
- ATM-core integration documentation or a handoff note, without ATM code

## Required work

1. Add a library-owned `check_rendered_output(format, template, text)` path;
   JSON must parse the complete body and return line/column/offset diagnostics.
2. Add a structured render-check report with at least:

   ```json
   {
     "template_contract_valid": true,
     "render_checked": true,
     "render_valid_for_context": true,
     "output_format": "json",
     "json_escape_mode": "auto",
     "diagnostics": []
   }
   ```

3. Add the canonical `--check-render` option to `render` and `validate`.
4. Keep plain `validate` static-only while making that fact explicit in its
   human and JSON output.
5. Make `validate --check-render` render in memory and emit no body/file.
6. Make `render --check-render` validate before writing/emitting.
7. Make ordinary JSON `render` fail closed by default in 1.4.1, unless the
   release review explicitly approves a short opt-in transition; it must never
   silently accept malformed output.
8. Ensure `render --json` wraps output-parser errors in the existing
   `DiagnosticEnvelope`, not plain stderr text.
9. Ensure ATM-core guidance says to inspect structured fields and diagnostics,
   not only process exit status.

## Guarantee boundaries

The report must distinguish static contract validity from exact-context output
validity. A successful static check without a context is not a claim that all
future conditional branches will parse. Auto mode may claim safe arbitrary
string interpolation only when the template does not use an untyped raw JSON
bypass.

## Required tests

- parser accepts valid JSON object, array, scalar, and whitespace;
- parser returns stable invalid-JSON code with line/column/offset;
- parser errors do not include full secret payloads;
- parser failure prevents stdout/file emission;
- `render --json` preserves the diagnostic envelope;
- `validate` reports `render_checked: false`;
- `validate --check-render` proves the exact context;
- `validate --lint --check-render` combines diagnostics;
- multi-pass render checks identify the failing pass;
- checked and unchecked non-JSON formats preserve current behavior;
- six-template-style double quotes fail closed in auto mode and succeed safely
  in legacy mode.

## Deliverables

- reusable format-aware output checker;
- machine-readable render-check report;
- CLI integration across render and validate;
- fail-closed JSON emission;
- ATM-core adapter contract and examples;
- unit and CLI tests.

## Acceptance criteria

- [ ] No checked render can emit malformed JSON with success status.
- [ ] ATM-core can distinguish static-only, context-required, failed, and
      context-valid results without parsing human text.
- [ ] `validate` does not silently become a rendering command.
- [ ] `validate --check-render`, `render`, and `render --json` share the same
      parser implementation and diagnostic code.
- [ ] Existing envelope and exit-code requirements remain satisfied.
- [ ] All workspace and targeted quality checks pass.

## Sc-lint cleanup and QA handoff

Run sc-lint against the final O.2 commit. Fix minor findings locally. Any
remaining finding gets a dedicated `fix/` worktree branched from this sprint's
final commit and grouped by rule class/owner; do not make one worktree per
warning or mix parser behavior with unrelated refactors. Send parent commit,
worktree, evidence, tests, and fix commit to team-lead. Team-lead opens the PR
and routes it to quality-mgr. O.2 closes only after QA approval, merge, and
post-merge revalidation.

## Validation

```text
cargo test --workspace
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
just lint target=template-contracts   # after O.3 target registration
git diff --check
```
