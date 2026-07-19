---
id: D.2-py
title: Python Bindings — Multi-Pass Composition Pipeline
status: stub-pending-rust-sprint
branch: sprint/d-2-py-bindings
target: integrate/phase-d
---

# Sprint D.2-py — Python Bindings — Multi-Pass Composition Pipeline (stub)

This is a placeholder, not a full sprint doc. It will be fleshed out to the
same rigor as [D.1-py](sprint-d-1-py-bindings.md) once
[D.2 — Multi-Pass Composition Pipeline](sprint-d-2-composition-pipeline.md)
has landed and its exact library surface is final. See
[Phase D README — Python Binding Parity](./README.md#python-binding-parity)
for the tandem sequencing policy this sprint follows.

## Surface expected to be wrapped

Based on D.2's own deliverables (`D1`–`D3` in that sprint's Deliverables
section):

- `sc_composer::render_all(parsed: &ParsedTemplate, contexts: &[(u8, HashMap<VariableName, InputValue>)]) -> Result<String>`
  — programmatic multi-pass rendering entry point (D.2's `D2` deliverable).
  Python signature is expected to be
  `render_all(parsed: ParsedTemplate, contexts: list[tuple[int, dict[str, Any]]]) -> str`.
- Multi-pass `compose()` auto-detection (D.2's `D1` deliverable) likely
  requires **no new Python function** — the existing `compose(request)`
  binding should auto-detect stacked headers once the Rust-side loop lands.
  This sprint would then add a smoke test proving a stacked-header template
  composes end-to-end through the existing `compose()` binding, not a new
  binding.
- `PyComposePolicy.passes` (constructor keyword + getter, accepting
  `list[PassConfig]`) — deferred out of D.1-py specifically because it has no
  consumer until this sprint's compose loop exists. `PassConfig` itself
  already ships in D.1-py.
- A `list[tuple[int, dict]]` → `Vec<(u8, HashMap<VariableName, InputValue>)>`
  conversion helper (`extract_pass_contexts` or similar) in `convert.rs`,
  reused by `render_all` here and by `verify` in D.4-py.

## Open design questions

- **Contexts representation.** `render_all` takes ordered per-pass contexts.
  Proposed: `list[tuple[int, dict]]` (preserves outer-to-inner order and
  matches the Rust `&[(u8, HashMap<...>)]` slice) rather than `dict[int, dict]`
  (loses guaranteed ordering pre-3.7 semantics and hides duplicate-pass
  errors). Needs confirmation once D.2 lands and the exact `render_all`
  signature is final.
- **`PyComposePolicy.passes` scope.** Should per-pass config be settable on
  the Python `ComposePolicy` constructor, or is it considered CLI-only
  (built from `--pass N` args in D.3) and therefore intentionally excluded
  from the Python surface? This widens the policy constructor if included —
  worth an explicit ruling before implementation.
- Whether `render_all` error surfaces cleanly through the existing
  `ScComposeError` / `ScValidationError` hierarchy or needs anything new
  (expectation: no new error types needed, per D.1-py's precedent).

## Dependencies

- [Sprint D.2 — Multi-Pass Composition Pipeline](sprint-d-2-composition-pipeline.md)
  must land first.
- [Sprint D.1-py — Multi-Pass Library Foundation (Python)](sprint-d-1-py-bindings.md)
  ships the `PassConfig` class and general wrapper conventions this sprint
  builds on.
