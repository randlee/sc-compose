# Phase D — Nested Templates as a First-Class Feature

## Status

Complete. All eight Phase D sprints (`D.1`-`D.4`, `D.1-py`-`D.4-py`) have
passed QA and merged into `integrate/phase-d`, then promoted to `develop` via
PR #140 (merge commit `0d09652`). Follow-up fixes landed through PRs #141–#144.

## Objective

Implement first-class nested-template support in `sc-composer` and
`sc-compose` through multi-pass stacked-header rendering, closing 10 of the 11
gaps identified in
[prototype/multipass/docs/gaps.md](../../prototype/multipass/docs/gaps.md)
(GAP-4 was closed by Phase C).

Phase D delivers the production implementation of the committed
[prototype/multipass](../../prototype/multipass/) reference implementation.
That prototype now runs against the maturin-backed `sc_compose` bindings where
those bindings already exist, and its passing test suite is the canonical
behavior contract for parser, discovery, rendering, verify, and template-init
semantics. The Rust implementation should follow that behavior closely rather
than re-deriving nested-template semantics from sprint prose alone.

## User Stories

Covered by the 6 user stories in
[prototype/multipass/docs/user-stories.md](../../prototype/multipass/docs/user-stories.md):

| Story | Description | Closed By |
|-------|-------------|-----------|
| US-1 | Deploy → Install → Invoke variable resolution (3-pass template) | D.1, D.2, D.3 |
| US-2 | template-init converter | D.4 |
| US-3 | render --all across all passes | D.3 |
| US-4 | verify drift check | D.4 |
| US-5 | Single-pass backward compatibility | D.2 (verified D.1–D.4) |
| US-6 | Per-pass validation | D.1 |

## Gap → Sprint Mapping

| Gap | Description | Sprint | Crate |
|-----|-------------|--------|-------|
| GAP-1 | Stacked header parsing (single → Vec\<Frontmatter\>) | D.1 | sc-composer |
| GAP-2 | Brace-count aware validation/discovery | D.1 | sc-composer |
| GAP-3 | Multi-pass compose loop | D.2 | sc-composer |
| GAP-4 | Renderer::with_delimiters public (now fallible on invalid delimiters) | ✅ Closed (C.2) | sc-composer |
| GAP-5 | Pass config types | D.1 | sc-composer |
| GAP-6 | verify entry point (library) | D.4 | sc-composer |
| GAP-7 | --all flag and --pass N args | D.3 | sc-compose |
| GAP-8 | template-init multi-pass support | D.4 | sc-compose |
| GAP-9 | verify command | D.4 | sc-compose |
| GAP-10 | Custom delimiter flag | D.3 | sc-compose |
| GAP-11 | parse_template_document re-exports | D.3 | sc-composer |

## Sprint Plan

Python bindings ship in **tandem** with each Rust sprint rather than as one
bolt-on phase at the end — see
[Python Binding Parity](#python-binding-parity) below.

| Sprint | Title | Scope |
|--------|-------|-------|
| [D.1](sprint-d-1-library-foundation.md) | Multi-Pass Library Foundation | Stacked headers, brace-count validation, pass config types |
| [D.1-py](sprint-d-1-py-bindings.md) | Python Bindings — Multi-Pass Library Foundation | PyO3 exposure of D.1's library surface |
| [D.2](sprint-d-2-composition-pipeline.md) | Multi-Pass Composition Pipeline | Multi-pass compose loop, render_all, backward compat |
| [D.2-py](sprint-d-2-py-bindings.md) | Python Bindings — Multi-Pass Composition Pipeline | PyO3 exposure of D.2's library surface |
| [D.3](sprint-d-3-cli-surface.md) | Multi-Pass CLI Surface | --all, --pass N, re-exports, delimiter flag |
| [D.3-py](sprint-d-3-py-bindings.md) | Python Bindings — Multi-Pass CLI Surface Parity Check | Library-surface parity and boundary audit for D.3 |
| [D.4](sprint-d-4-template-init-verify.md) | template-init + verify | template-init converter, verify library + CLI |
| [D.4-py](sprint-d-4-py-bindings.md) | Python Bindings — template-init + verify | PyO3 exposure of D.4's library-owned surface |

## Dependency Order

```text
Rust track:     D.1 ─► D.2 ─► D.3 ─► D.4
                  │      │      │      │
Python track:   D.1-py D.2-py D.3-py D.4-py

Dispatch rule: each D.#-py becomes dispatch-ready after the corresponding
Rust sprint has passed QA and merged to integrate/phase-d.
```

D.1 must ship first because every later Rust sprint depends on the
stacked-header types and brace-count-aware validation. D.2 ships next — the
multi-pass compose loop is required before any CLI surface can be built. D.3
ships next, providing the `--pass N` CLI arg infrastructure that D.4 depends
on. D.4 (template-init + verify) ships last among the Rust sprints, reusing
D.3's per-pass variable arguments and D.2's `render_all` entry point.

Each `-py` sprint depends on its corresponding Rust sprint having passed QA and
merged to `integrate/phase-d`, plus any explicitly documented wrapper
conventions established by earlier `-py` sprints. A `-py` sprint does **not**
serially gate the next Rust sprint. The Python track is therefore contingent on
the Rust track, but non-blocking with respect to subsequent Rust implementation
work.

**Landing checkpoint:** as of Monday, July 20, 2026, all eight Phase D
sprints had passed QA and merged to `integrate/phase-d`: Rust track `D.1`–
`D.4` via PRs #129, #130, #131, and #133, plus Python track `D.1-py`–
`D.4-py` via PRs #134, #135, #136, and #137. The phase-complete branch was
subsequently promoted to `develop` by PR #140; the current status is recorded
at the top of this document.

## Python Binding Parity

**Process change from the original Phase D plan.** Python bindings for the
multi-pass surface are no longer planned as a single follow-on phase
(the original "D.5") after all four Rust sprints land. Team-lead and the
project owner have decided Python is a first-class customer of every Phase D
feature, not a bolt-on: each Rust sprint gets its own `-py` companion sprint
that wraps exactly that sprint's new library surface once the Rust sprint has
passed QA and merged to `integrate/phase-d`.

- Sequencing policy: each Rust sprint `D.#` owns a corresponding `D.#-py`
  companion sprint once the Rust sprint is merge-complete.
- Each `-py` sprint becomes assignable to comp only after its corresponding
  Rust sprint has passed QA and merged to `integrate/phase-d` — not batched,
  not deferred, and not assumed from branch/task wording alone.
- Python is never more than one sprint behind the Rust surface it wraps.
- All four `-py` sprint docs must be maintained at the same full rigor as the
  Rust sprint docs once their corresponding Rust sprint has landed.
- D.1-py sets the structure and wrapper-style bar for the later `-py` docs.
- D.2-py wraps D.2's library-owned `render_all()` surface and
  `ComposePolicy.passes`.
- D.3-py is intentionally a small parity/boundary sprint: D.3 is largely
  CLI-owned, so D.3-py exists to lock the library-vs-CLI boundary and protect
  Python import-surface stability rather than to mirror CLI flag grammar.
- D.4-py wraps library-owned `verify()` and `VerifyResult` only.
  Multi-pass `template-init` remains CLI-owned and therefore out of scope for
  a `bindings/python` library adapter.
- All `bindings/python` boundary rules from `CLAUDE.md` (rules 3–5) continue
  to apply unchanged to every `-py` sprint: `bindings/python` may depend on
  `sc-composer` only, never on `sc-compose` or ATM-specific crates.

## Fixed Product Decisions

These decisions are closed and must not be revisited during implementation:

- Pass N uses `{N+1}` braces for variable delimiters. Block delimiters
  `{% %}` are unchanged. (DD-001)
- Stacked YAML frontmatter: each pass has its own `---...---` header block,
  appearing in outer-to-inner order. (DD-002)
- `pass: N` in the header declares which pass the header belongs to.
  Absent → `pass: 1`. Brace count = pass number + 1. (DD-003)
- YAML header's `pass` field is authoritative; file extension is a human
  signal only. (DD-004)
- Exact-match delimiter scanning: `discover_tokens(brace_count=N)` must NOT
  match `{N}` as a prefix inside `{N+1}`. (DD-005)
- Longest-match-first for template-init when replacing concrete values with
  variables. (DD-006)
- No breaking changes to current single-header API. (DD-007)
- `bindings/python` must not depend on `sc-compose`, `sc-observability`, or
  ATM-specific crates. Phase D Python wrapping ships in tandem with each Rust
  sprint (see [Python Binding Parity](#python-binding-parity)), not as a
  deferred follow-on phase.

## Hard Dependencies

- [docs/architecture.md](../architecture.md)
- [docs/requirements.md](../requirements.md)
- [docs/project-plan.md](../project-plan.md)
- [prototype/multipass/docs/gaps.md](../../prototype/multipass/docs/gaps.md)
- [prototype/multipass/docs/user-stories.md](../../prototype/multipass/docs/user-stories.md)
- [Phase C deliverables](../phase-C/README.md) (Renderer::with_delimiters is public)
- [CLAUDE.md](../../CLAUDE.md)

## Authoritative Review Baseline

Future Phase D reviews should treat this document set as the authoritative
baseline:

- [README.md](./README.md) (this file)
- [sprint-d-1-library-foundation.md](./sprint-d-1-library-foundation.md)
- [sprint-d-1-py-bindings.md](./sprint-d-1-py-bindings.md)
- [sprint-d-2-composition-pipeline.md](./sprint-d-2-composition-pipeline.md)
- [sprint-d-2-py-bindings.md](./sprint-d-2-py-bindings.md)
- [sprint-d-3-cli-surface.md](./sprint-d-3-cli-surface.md)
- [sprint-d-3-py-bindings.md](./sprint-d-3-py-bindings.md)
- [sprint-d-4-template-init-verify.md](./sprint-d-4-template-init-verify.md)
- [sprint-d-4-py-bindings.md](./sprint-d-4-py-bindings.md)
- [prototype/multipass/](../../prototype/multipass/) (reference implementation
  and executable behavior oracle)

## Architecture Coverage

This gap is now closed. `docs/architecture.md` has been updated to cover:

- the multi-pass rendering loop and outer-to-inner pass ordering,
- brace-count-aware delimiter discrimination and higher-brace protection,
- the leading-header-only parsing rule so `---` in the body remains literal,
- the `verify` library/CLI ownership boundary,
- the `template-init` conversion algorithm and single-pass normalization rule,
- the `types.rs` and `verify.rs` modules in `sc-composer`,
- `ComposePolicy.passes: Vec<PassConfig>`, and
- the updated `ParsedTemplate { passes, body }` shape.

The ADR set in `docs/adrs/0002` through `0008` closes the prior decision-record
gap for DD-001 through DD-007. `docs/architecture.md`, those ADRs, and the
committed `prototype/multipass/` directory are the authoritative design
baseline for Phase D.

## Follow-On Sprints

Python binding sprints are no longer a single follow-on phase — see
[Python Binding Parity](#python-binding-parity). Status per tandem sprint:

All four tandem `-py` sprint docs are now expected to exist at full sprint-doc
rigor:

- [D.1-py — Python Bindings — Multi-Pass Library Foundation](sprint-d-1-py-bindings.md)
  wraps D.1's parsing, pass-config, and discovery library surface.
- [D.2-py — Python Bindings — Multi-Pass Composition Pipeline](sprint-d-2-py-bindings.md)
  wraps `render_all()` and Python exposure of `ComposePolicy.passes`.
- [D.3-py — Python Bindings — Multi-Pass CLI Surface Parity Check](sprint-d-3-py-bindings.md)
  locks the library-vs-CLI boundary and verifies that D.1-py / D.2-py remain
  aligned with D.3's consolidated `sc_composer` public surface.
- [D.4-py — Python Bindings — template-init + verify](sprint-d-4-py-bindings.md)
  wraps library-owned `verify()` and `VerifyResult`, and explicitly records
  that multi-pass `template-init` remains CLI-owned and out of scope for the
  Python adapter.

Not yet drafted:

- Multi-pass smoke/integration test suite beyond each `-py` sprint's own
  binding smoke tests
- `verify` builtin variable overrides (`RENDER_DATE`, `RENDER_TIMESTAMP`)
  persistence/config surface
