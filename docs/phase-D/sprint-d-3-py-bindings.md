---
id: D.3-py
title: Python Bindings — Multi-Pass CLI Surface Parity Check
status: stub-pending-rust-sprint
branch: sprint/d-3-py-bindings
target: integrate/phase-d
---

# Sprint D.3-py — Python Bindings — Multi-Pass CLI Surface Parity Check (stub)

This is a placeholder, not a full sprint doc. It will be fleshed out once
[D.3 — Multi-Pass CLI Surface](sprint-d-3-cli-surface.md) has landed. See
[Phase D README — Python Binding Parity](./README.md#python-binding-parity)
for the tandem sequencing policy this sprint follows.

## Surface expected to be wrapped

D.3 is largely **CLI-only** (`sc-compose` crate): the `--all` flag, `--pass N`
/ `--var` / `--var-file` args, and `--variable-delimiters` / `--brace-count`
flags (D.3's `D1`, `D2` deliverables). Per repo boundary rules,
`bindings/python` may depend on `sc-composer` only and never on `sc-compose`
— CLI flags themselves are never exposed to Python.

The one item with Python relevance is D.3's `D3` deliverable:

- `sc_composer::lib.rs` re-exports of `PassConfig`, updated `ParsedTemplate`,
  `discover_tokens_with_brace_count`, `discover_all_pass_tokens`, `render_all`
  (GAP-11). These re-exports are what D.1-py and D.2-py already bind against
  via `sc_composer::...` paths, so this sprint is expected to be a **parity
  check**, not new binding surface: confirm nothing D.1-py/D.2-py wrapped
  needs its import path updated now that D.3's re-export consolidation has
  landed, and confirm no net-new public library symbol appeared as a side
  effect of D.3 that Python should also wrap.

## Open design questions

- Does D.3 land any new `sc-composer` (library) symbol beyond the GAP-11
  re-export consolidation, or is everything else CLI-only as the current
  D.3 sprint doc states? If library-only, this sprint may end up being a
  no-op / confirmation-only sprint rather than shipping new Python surface.
- If it is a no-op, should it still be a formal sprint (for tandem-policy
  consistency and an explicit "nothing to wrap" record) or folded into
  D.4-py's dependency check? Current lean: keep it as a real, if small,
  sprint — the tandem policy commits to a `-py` sprint after every Rust
  sprint regardless of surface size, so skipping it silently would break the
  "never more than one sprint behind" guarantee.

## Dependencies

- [Sprint D.3 — Multi-Pass CLI Surface](sprint-d-3-cli-surface.md) must land
  first.
- [Sprint D.1-py](sprint-d-1-py-bindings.md) and
  [Sprint D.2-py](sprint-d-2-py-bindings.md) ship the wrapper conventions and
  import paths this sprint verifies against.
