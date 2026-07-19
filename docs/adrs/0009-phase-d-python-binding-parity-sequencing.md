# ADR-0009: Phase D Python-Binding Parity Sequencing

## Status

Accepted

## Context

Phase D originally treated Python bindings as a single deferred follow-on
effort after the Rust multi-pass work landed. The Phase D README was then
restructured around tandem `-py` companion sprints, but that decision lived
only in prose and introduced several contradictions:

- the README dependency diagram rendered the `-py` sprints as hard serial
  gates for later Rust sprints,
- landing-status language implied some `-py` sprints were already assignable
  before their Rust counterparts had actually passed QA and merged, and
- the planned per-pass `contexts` parameter type diverged across D.2/D.4 docs
  (`VariableName`-keyed in one place, `String`-keyed in another).

Separately, included/supporting-template frontmatter handling in the current
Phase D codebase remains on the existing compatibility surface:
`ExpandedTemplate.frontmatters` stores one optional frontmatter block per file,
not a full stacked-pass vector, and D.1/D.1-py do not target expanding that
shape.

## Decision

- Phase D uses two tracks:
  - the Rust delivery track `D.1 -> D.2 -> D.3 -> D.4`
  - the Python companion track `D.1-py`, `D.2-py`, `D.3-py`, `D.4-py`
- A `D.#-py` sprint becomes dispatch-ready only after its corresponding Rust
  sprint has passed QA and merged to `integrate/phase-d`.
- A `D.#-py` sprint does not itself block the next Rust sprint. Python parity
  is contingent on the Rust sprint it wraps, but non-blocking for later Rust
  implementation work.
- The canonical Rust per-pass `contexts` type for both `render_all` and
  `verify` is `&[(u8, BTreeMap<VariableName, InputValue>)]`. Python bindings
  may expose that as `list[tuple[int, dict[str, Any]]]`, but that is an adapter
  representation over the same Rust contract rather than a second logical API.
- Included/supporting-template stacked-frontmatter exposure is not part of
  D.1/D.1-py. Until a later Rust sprint explicitly changes
  `ExpandedTemplate.frontmatters`, the docs must describe that surface as
  preserving the pre-existing single-frontmatter compatibility view.

## Consequences

- README and sprint docs must distinguish dispatch-readiness from mere draft
  completeness.
- D.2-py and D.4-py must reuse one shared `contexts` convention rather than
  documenting independent guesses.
- Reviews can treat `ExpandedTemplate.frontmatters` / `PyExpandedTemplate`
  stacked-pass exposure as intentionally deferred unless and until a later
  sprint explicitly targets `crates/sc-composer/src/include.rs` and the
  corresponding Python adapter surface.
