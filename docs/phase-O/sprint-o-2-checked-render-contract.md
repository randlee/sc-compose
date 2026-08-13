---
id: O.2
title: Checked Render API, JSON Parser Gate, and ATM-core Contract
phase: O
status: complete
branch: sprint/o-2-checked-render-contract
worktree: ../sc-compose-worktrees/sprint/o-2-checked-render-contract
target: integrate/phase-o
merge: PR #425 at 23e8c0d
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

- `crates/sc-composer/src/render_check.rs` (new library module)
- `crates/sc-composer/src/lib.rs`
- `crates/sc-composer/src/diagnostics/schema.rs`
- `crates/sc-compose/src/commands/compose.rs`
- `crates/sc-compose/src/commands/compose_render.rs`
- `crates/sc-compose/src/commands/compose_output.rs`
- `crates/sc-compose/src/cli/schema.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli/render.rs`
- `crates/sc-compose/tests/cli/validate.rs`
- `crates/sc-compose/tests/json_cli/render.rs`
- `crates/sc-compose/tests/json_cli/validate.rs`
- `docs/requirements.md` FR-7, FR-8, and FR-8a
- `docs/atm-adapter-notes.md` (ATM-core handoff only; no ATM code)

## Required work

1. Add a library-owned `check_rendered_output(format, template, text)` path;
   JSON must parse the complete body and return line/column/offset diagnostics.
2. Add a structured render-check report with at least:

   ```json
   {
     "state": "render_checked",
     "template": "path/to/assignment.json.j2",
     "output_format": "json",
     "json_escape_mode": "auto",
     "checked_context": "caller-defined exact context summary",
     "diagnostics": []
   }
   ```

3. Add the canonical `--check-render` option to `render` and `validate`.
4. Keep plain `validate` static-only while making that fact explicit in its
   human and JSON output.
5. Make `validate --check-render` render in memory and emit no body/file.
6. Make `render --check-render` validate before writing/emitting.
7. Make ordinary JSON `render` fail closed by default in 1.4.1 before any
   stdout/file write; there is no opt-in transition that permits malformed
   output.
8. Ensure `render --json` wraps output-parser errors in the existing
   `DiagnosticEnvelope`, not plain stderr text.
9. Ensure ATM-core guidance says to inspect structured fields and diagnostics,
   not only process exit status.

## Authoritative contract reference

Implement the `Authoritative checked-render contract` in
`docs/phase-O/phase-O-plan.md`; that section is the only source for the enum,
checker signature, report fields, parser timing, and 1.4.1 fail-closed default.
This sprint may add implementation-specific error types, but must not rename
or weaken those fields. The report must distinguish static contract validity
from exact-context output validity; a successful static check without a context
is not a claim that all future conditional branches will parse.

## Required tests

- parser accepts valid JSON object, array, scalar, and whitespace;
- parser returns stable invalid-JSON code with line/column/offset;
- parser errors do not include full secret payloads;
- parser failure prevents stdout/file emission;
- `render --json` preserves the diagnostic envelope;
- `validate` reports the `static_only` state;
- `validate --check-render` proves the exact context;
- `validate --lint --check-render` combines diagnostics;
- multi-pass render checks identify the failing pass;
- checked and unchecked non-JSON formats preserve current behavior;
- six-template-style double quotes fail closed in auto mode and succeed safely
  in legacy mode.
- ordinary unflagged JSON `render` rejects malformed output before writing any
  stdout/file bytes;
- all checked-render paths use the authoritative checker and report fields;
- ADR-0019 is accepted before implementation handoff; this sprint does not
  dispatch source work while the Phase O design-acceptance gate is open.

## Deliverables

- reusable format-aware output checker;
- machine-readable render-check report;
- CLI integration across render and validate;
- fail-closed JSON emission;
- ATM-core adapter contract and examples;
- unit and CLI tests.

## Acceptance criteria

- [x] No checked render can emit malformed JSON with success status.
- [x] ATM-core can distinguish static-only, context-required, failed, and
      context-valid results without parsing human text.
- [x] `validate` does not silently become a rendering command.
- [x] `validate --check-render`, `render`, and `render --json` share the same
      parser implementation and diagnostic code.
- [x] Unflagged JSON `render` fails closed before emission; no release-review
      exception or opt-in transition remains in the implementation.
- [x] `RenderCheckReport` uses explicit states and only `CheckedOutput` has an
      emission method; parser failures are typed `Err` results.
- [x] Existing envelope and exit-code requirements remain satisfied.
- [x] ADR-0019 is accepted before implementation handoff.
- [x] All workspace and targeted quality checks pass.

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
git diff --check
```
