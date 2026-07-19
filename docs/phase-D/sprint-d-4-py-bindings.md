---
id: D.4-py
title: Python Bindings — template-init + verify
status: stub-pending-rust-sprint
branch: sprint/d-4-py-bindings
target: integrate/phase-d
---

# Sprint D.4-py — Python Bindings — template-init + verify (stub)

This is a placeholder, not a full sprint doc. It will be fleshed out to the
same rigor as [D.1-py](sprint-d-1-py-bindings.md) once
[D.4 — template-init + verify](sprint-d-4-template-init-verify.md) has passed
QA, merged to `integrate/phase-d`, and its exact library surface is final.
This is the last sprint in the tandem
Python-binding sequence for Phase D — see
[Phase D README — Python Binding Parity](./README.md#python-binding-parity).

## Surface expected to be wrapped

Based on D.4's own deliverables (`D1`, `D3` in that sprint's Deliverables
section):

- `sc_composer::verify(template_path, deployed_path, contexts) -> Result<VerifyResult, ComposeError>`
  (D.4's `D1` deliverable, library-hosted — bindable as-is). Expected Python
  shape: a new `PyVerifyResult` wrapper class (`clean -> bool`,
  `diff -> str | None`, `exit_code -> int`) and
  `verify(template_path, deployed_path, contexts, overrides=None) -> VerifyResult`,
  with `overrides` mapping to the builtin-variable override dict
  (`RENDER_DATE`, `RENDER_TIMESTAMP`) for deterministic output.
- `template-init` multi-pass support (D.4's `D3` deliverable) is **CLI-hosted
  in D.4's current plan** (`sc-compose/src/commands/template_init.rs`), and
  `bindings/python` may depend on `sc-composer` only. Whether this is
  bindable at all depends on whether the multi-pass template-init conversion
  core gets hosted in `sc-composer` (mirroring how the existing single-pass
  `frontmatter_init` is bindable because it lives in the library). This is
  the single biggest open question for this sprint — see below.

## Open design questions

- **template-init CLI/library boundary.** If D.4 (or a follow-up) hosts the
  multi-pass template-init conversion core in `sc-composer`, this sprint
  would expose `template_init(path, passes, force=False, dry_run=False) -> FrontmatterInitResult`
  reusing the existing `PyFrontmatterInitResult` wrapper. If template-init
  stays CLI-only, this deliverable drops from D.4-py entirely and must be
  recorded explicitly under that sprint's "This Sprint Does Not Close"
  section rather than silently deferred. This needs a decision before D.4-py
  is fleshed out — ideally settled during or immediately after D.4 itself,
  since it affects D.4's own crate placement.
- **`verify` builtin-override semantics in Python.** D.4 scopes builtin
  overrides as non-persistent (per-call only). Confirm no expectation of a
  Python-side persistent override store when this sprint is drafted.
- **Contexts representation reuse.** `verify`'s `contexts` parameter should
  reuse whatever `list[tuple[int, dict]]` (or equivalent) shape D.2-py
  settles on for `render_all`, rather than inventing a second convention. The
  corresponding Rust API should stay aligned with D.2's
  `&[(u8, BTreeMap<VariableName, InputValue>)]` contract.

## Dependencies

- [Sprint D.4 — template-init + verify](sprint-d-4-template-init-verify.md)
  must pass QA and merge first.
- [Sprint D.2-py — Multi-Pass Composition Pipeline (Python)](sprint-d-2-py-bindings.md)
  settles the per-pass `contexts` convention this sprint reuses for `verify`.
