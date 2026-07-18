# Phase D — Multi-Pass Template Rendering

## Status

Planned follow-on work after the shipped Phase C Python bindings (v1.2.0).

## Objective

Implement multi-pass stacked-header template rendering in `sc-composer` and
`sc-compose`, closing 10 of the 11 gaps identified in
[prototype/multipass/docs/gaps.md](../../prototype/multipass/docs/gaps.md)
(GAP-4 was closed by Phase C).

Phase D delivers the production implementation of the multi-pass prototype
([prototype/multipass-v2](../../prototype/multipass/)) proven in the prototype
worktree. The prototype has been validated against the v1.2.0 maturin Python
bindings — 55/55 tests passing — and Phase D translates that prototype logic
into production Rust code.

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
| GAP-4 | Renderer::with_delimiters public | ✅ Closed (C.2) | sc-composer |
| GAP-5 | Pass config types | D.1 | sc-composer |
| GAP-6 | verify entry point (library) | D.4 | sc-composer |
| GAP-7 | --all flag and --pass N args | D.3 | sc-compose |
| GAP-8 | template-init multi-pass support | D.4 | sc-compose |
| GAP-9 | verify command | D.4 | sc-compose |
| GAP-10 | Custom delimiter flag | D.3 | sc-compose |
| GAP-11 | parse_template_document re-exports | D.3 | sc-composer |

## Sprint Plan

| Sprint | Title | Scope |
|--------|-------|-------|
| [D.1](sprint-d-1-library-foundation.md) | Multi-Pass Library Foundation | Stacked headers, brace-count validation, pass config types |
| [D.2](sprint-d-2-composition-pipeline.md) | Multi-Pass Composition Pipeline | Multi-pass compose loop, render_all, backward compat |
| [D.3](sprint-d-3-cli-surface.md) | Multi-Pass CLI Surface | --all, --pass N, re-exports, delimiter flag |
| [D.4](sprint-d-4-template-init-verify.md) | template-init + verify | template-init converter, verify library + CLI |

## Dependency Order

```
D.1 (library foundation)
 ├─► D.2 (composition pipeline)
 │    ├─► D.3 (CLI surface)
 │    │    └─► D.4 (template-init + verify)
 │    └─► D.4 (also depends on D.2 for render_all)
 └─► (D.3 and D.4 also depend on D.1 types)
```

D.1 must ship first — every other sprint depends on the stacked-header types
and brace-count-aware validation. D.2 ships second — the multi-pass compose
loop is required before any CLI surface can be built. D.3 ships third —
providing the `--pass N` CLI arg infrastructure that D.4 depends on. D.4
(template-init + verify) ships last, reusing D.3's per-pass variable
arguments and D.2's `render_all` entry point.

**Note:** D.4 depends on D.3 for `--pass N` CLI arg infrastructure; they
cannot proceed in parallel despite sharing D.2 as a common dependency.

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
  ATM-specific crates. Phase D Python wrapping is deferred to a follow-on
  phase.

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
- [sprint-d-2-composition-pipeline.md](./sprint-d-2-composition-pipeline.md)
- [sprint-d-3-cli-surface.md](./sprint-d-3-cli-surface.md)
- [sprint-d-4-template-init-verify.md](./sprint-d-4-template-init-verify.md)

## Architecture Documentation Gap

**Note:** `docs/architecture.md` has no coverage of multi-pass stacked-header
template rendering. The architecture doc should be updated with:

- Multi-pass rendering loop design and pass ordering (outer-to-inner)
- Brace-count-aware delimiter discrimination semantics
- `protect_higher_braces` mechanism and `{% raw %}` interaction constraints
- `verify` drift-check boundary between `sc-composer` (library) and `sc-compose` (CLI)
- `template-init` conversion algorithm and output contract
- New `types.rs` and `verify.rs` modules in `sc-composer` (Section 4 Module Architecture)
- `ComposePolicy.passes: Vec<PassConfig>` field (Section 8.2)
- Updated `ParsedTemplate` shape: `passes: Vec<Frontmatter>` (Section 8)

Additionally, the 7 Fixed Product Decisions below (DD-001 through DD-007) are
significant architectural choices that lack formal ADR records. The repo has
only ADR-0001 (observability health). Until architecture.md is updated to cover
multi-pass, the sprint plans serve as the interim architectural record.
Implementers should treat the sprint docs' code samples and decision rationale
as the build-time reference.

This is tracked as a pre-implementation requirement — D.1 implementers need
the architecture decisions documented before coding the `split_frontmatter()`
rewrite.

## Follow-On Sprints (not yet drafted)

- Python binding extension: wrap the multi-pass types (`ParsedTemplate` with
  `Vec<PassHeader>`, `Renderer.with_delimiters()`, `discover_tokens` with
  brace_count) in `bindings/python`
- Python `template-init` and `verify` bindings
- Multi-pass smoke/integration test suite
- `verify` builtin variable overrides (`RENDER_DATE`, `RENDER_TIMESTAMP`)
