---
id: phase-J
title: Repowise Hot-Spot Maintainability Cleanup
status: planned
branch: integrate/phase-j
worktree: ../sc-compose-worktrees/integrate/phase-j
target: develop
---

# Phase J — Repowise Hot-Spot Maintainability Cleanup

## Objective

Reduce the maintainability risk flagged by GitHub issue
[#212](https://github.com/randlee/sc-compose/issues/212)'s Repowise hot-spot
snapshot (`develop`, 443 files analyzed, hotspot health 5.61/10) for the three
lowest-scoring live files: `crates/sc-compose/src/cli.rs` (3.31),
`crates/sc-composer/src/validation.rs` (2.37), and
`crates/sc-composer/src/frontmatter.rs` (3.45). This is not a correctness
emergency — Phase I is complete and the current code is fully tested — but
these files have grown since the snapshot (`cli.rs` 726→821 NLOC,
`validation.rs` 1655→1816 NLOC) and concentrate unrelated change, raising the
cost and risk of every future touch. Phase J follows the same
snapshot-review→decomposition pattern already used successfully in Phase F
(PR #149/#172 hotspot scans → PR #150-156 decomposition → PR #173 sprint
plans → PR #181 merge).

Phase J performs behavior-preserving structural decomposition only; it is not a
planning/design sprint under the sprint-planning guidelines. Each J.1-J.4
sprint is a full implementation sprint and must meet the complete validation
checklist below. Observable behavior must remain unchanged: diagnostic codes,
ordering, severity, CLI flags, exit codes, and public Rust/Python APIs must be
identical before and after each sprint.

## Refactor target ledger

The four sprints cover the six concrete seams identified in the issue review;
this ledger is the scope authority for Phase J:

| Target | Owning sprint | Closure boundary |
| --- | --- | --- |
| CLI command/argument schema | J.1 | `crate::cli::*` re-exports preserve all consumers |
| CLI pass-scoped input normalization | J.1 | `--pass`, `--var`, and `--var-file` parsing/filtering remain byte-for-byte compatible |
| CLI format and JSON-capability mapping | J.1 | every existing command/subcommand retains its JSON-vs-text decision |
| Validation state/context assembly | J.2 | `ValidationState` shape, precedence, pass maps, and built-in injection are unchanged |
| Validation policy and required-path diagnostics | J.3 | diagnostic code, severity, order, location, and include-chain output are unchanged |
| Frontmatter parser and normalizer | J.4 | `Frontmatter`, `ParsedTemplate`, and `parse_template_document` remain stable public entry points |

## Current baseline and authority

- Phase I (merged `develop` @ `57c4f71`, backlog cleanup merged @ `46e079d`)
  is the current baseline. Its extraction and validation behavior (I.1-I.6)
  is a compatibility boundary, not a decomposition target.
- `crates/sc-composer/src/discovery.rs` was isolated and hardened by Phase
  I.5 (loop-context built-ins) and is explicitly **out of scope** for Phase
  J — comp's review scores it 5/5 leave-alone-readiness and 2/5
  refactor-readiness precisely because it was just stabilized.
- `crates/sc-composer/src/extract/*` (all format adapters) is likewise
  **out of scope**. Phase I's extraction behavior is a shipped contract, and
  the backlog-cleanup sprint already deduplicated its one piece of internal
  debt (`line_column()`).
- `crates/sc-composer/src/diagnostics.rs` and `crates/sc-composer/src/types.rs`
  are out of scope for the first Phase J pass; they are shared by every
  module Phase J touches and must not move underneath sprints that depend on
  their current shape.
- This baseline and the sprint scope below come from a read-only review
  performed by `comp` against issue #212 (ATM task
  `issue-212-hotspot-review`, 2026-08-04). Findings, severities, and file/line
  locations in each sprint document are transcribed from that review, not
  independently re-derived by this plan.

## Sprint sequence and concurrency

1. [Sprint J.1 — CLI Argument and Pass-Input Seams](sprint-j-1-cli-argument-seams.md)
   splits `cli.rs`'s schema, pass-input normalization, and JSON-capability
   mapping into internal modules behind `crate::cli` re-exports. Independent
   first sprint; no dependency on the others.
2. [Sprint J.2 — Validation State and Context Assembly](sprint-j-2-validation-state-assembly.md)
   extracts `ValidationState` construction, frontmatter/default merging,
   per-pass discovery maps, and built-in injection from `validation.rs` into
   a state module, freezing a state-shape contract first. The
   highest-risk sprint because I.5 recently changed this seam's
   state/discovery boundary.
3. [Sprint J.3 — Validation Policy and Required-Path Diagnostics](sprint-j-3-validation-policy-diagnostics.md)
   separates the remaining `validation.rs` diagnostic-policy and
   required-path/location collectors from `validate_expanded`'s
   orchestration. Depends on J.2's frozen state contract.
4. [Sprint J.4 — Frontmatter Parser and Normalizer Split](sprint-j-4-frontmatter-parser-split.md)
   splits `frontmatter.rs`'s model, delimiter/stacked-header scanning, and
   YAML normalization into internal modules behind the existing
   `Frontmatter`/`ParsedTemplate`/`parse_template_document` public API. Depends
   on J.2 and J.3's characterization work because `Frontmatter`/
   `ParsedTemplate` are validation's primary inputs; it is the highest
   fan-out sprint (consumed by `include.rs`, `composer.rs`, `discovery.rs`,
   `validation.rs`, `frontmatter_init.rs`, and every extraction adapter).

J.1 has no dependency on J.2-J.4 and may be developed, reviewed, QA-tested,
and merged independently. J.2 must land (and its characterization tests must
pass) before J.3 begins. J.4 must not start until J.2 and J.3's
characterization coverage exists, because it is the sprint most likely to
destabilize validation and extraction indirectly through the shared
`Frontmatter`/`ParsedTemplate` types.

## Plan review and handoff

The plan is not implementation-ready until it passes `/plan-hardening`
(traceability, architecture review per `qa-template.xml.j2`
`review_type: plan_gate` — `req-qa` and `arch-qa`) and is approved by
quality-mgr. Only after that gate does Phase J move to `/codex-orchestration`
dev dispatch.

## Hard boundaries

- Every sprint is behavior-preserving: no diagnostic code, severity, message,
  ordering, include-chain attribution, CLI flag, exit code, or public
  Rust/Python API may change as an observable side effect of a Phase J
  sprint. Any such change belongs in a separate, explicitly-scoped sprint.
- `crates/sc-composer/src/discovery.rs`'s contents, public surface, and logic
  are excluded from Phase J. Call sites elsewhere may be relocated only when
  they invoke an existing exported discovery function, `discovery.rs` itself
  remains untouched, and discovery semantics remain unchanged. The
  `crates/sc-composer/src/extract/*` adapters remain entirely excluded: do not
  move, rename, or restructure them even incidentally.
- `crates/sc-composer/src/diagnostics.rs` and `crates/sc-composer/src/types.rs`
  are not decomposition targets in this phase; sprints may depend on them but
  must not restructure them.
- Each sprint must add characterization tests for the exact contract it
  extracts (see each sprint's Required validation) **before** moving code,
  not after — this is a decomposition safety requirement, not a QA
  afterthought.
- Module splits preserve existing public paths via re-exports
  (`crate::cli::*`, `sc_composer::{Frontmatter, ParsedTemplate,
  parse_template_document}`, etc.). No downstream crate (`sc-compose`,
  `bindings/python`) may need a call-site change as a result of a Phase J
  sprint.
- Any new implementation files remain private submodules behind the existing
  `cli`, `validation`, or `frontmatter` ownership boundaries. Phase J does not
  create public `context`, `tokens`, or alternate validation/frontmatter APIs;
  the architecture's pure-library ownership remains unchanged.

## Authoritative validation checklist

Every implementation sprint owns tests with the behavior it preserves. The
minimum validation for each implementation sprint is:

- a characterization-test pass added *before* the code move, proven to pass
  against the pre-move code, then proven to still pass after the move;
- `cargo test --workspace`;
- `cargo clippy --all-targets --all-features -- -D warnings`;
- `cargo fmt --all --check` and `git diff --check`;
- Python binding tests for any type/module Phase J's split touches
  indirectly (J.4 in particular, given `Frontmatter`/`ParsedTemplate` fan-out
  into `bindings/python`): run
  `maturin develop --release --manifest-path bindings/python/Cargo.toml`, then
  run `python3 -m pytest bindings/python/tests/test_smoke.py`;
- a targeted diff review confirming no diagnostic code, severity, order, or
  location output changed for any existing fixture.

J.2 must additionally re-run and pass the full I.5 loop-context regression
suite (`validation::tests::strict_mode_accepts_approved_loop_context_builtins`
and siblings) unchanged, since it is the sprint touching that seam most
directly. J.4 must additionally re-run the full extraction adapter test
suite (`crates/sc-composer/tests/extract_integration.rs` and per-format CLI
tests) unchanged, since `Frontmatter`/`ParsedTemplate` are extraction inputs.

Sprint documents reference this checklist rather than restating these common
commands. Each sprint's document adds only its own evidence requirements.

## Exit gate

Phase J is complete only when:

- `cli.rs`, `validation.rs`, and `frontmatter.rs` are decomposed per their
  sprint's scope, with material NLOC/complexity reduction for each owning
  module demonstrated by the sprint diff and decomposition evidence;
- every characterization test added for J.1-J.4 passes both before and after
  its corresponding move, and remains in the suite afterward;
- no diagnostic code, severity, ordering, CLI flag, exit code, or public
  Rust/Python API changed as an observable side effect of any sprint;
- `discovery.rs`, `extract/*`, `diagnostics.rs`, and `types.rs` remain
  untouched by Phase J's changes;
- the full workspace, Python, CLI, and formatting checks pass at the
  integration tip; and
- team-lead, quality-mgr, req-qa, and arch-qa can review each sprint from its
  authoritative document and evidence; and
- after the Phase J integration tip is available and before phase closeout,
  `quality-mgr` requests a fresh Repowise scan and records it in the plan-gate
  report as a non-blocking diagnostic. A score that does not improve cannot by
  itself fail closure because scan timing is outside sprint control; a concrete
  regression found by that scan must be assigned before closeout.

## Traceability matrix

| Sprint | Target | Status | PR | Notes |
|--------|--------|--------|----|-------|
| J.1 | `cli.rs` | planned | — | independent |
| J.2 | `validation.rs` (state/context) | planned | — | sequence per Sprint sequence and concurrency |
| J.3 | `validation.rs` (policy/diagnostics) | planned | — | sequence per Sprint sequence and concurrency |
| J.4 | `frontmatter.rs` | planned | — | sequence per Sprint sequence and concurrency |

## References

- GitHub issue #212 (Repowise hot-spot snapshot, `develop`):
  https://github.com/randlee/sc-compose/issues/212
- GitHub issue #208 (duplicate snapshot, closed as superseded by #212)
- Phase F precedent: PR #149, #172 (hotspot scans), PR #150-156
  (decomposition), PR #173 (Phase F sprint plans), PR #181 (Phase F merge)
- Phase I exit-gate PASS and merge-forward: PR #224 (`57c4f71`)
- Phase I backlog cleanup: PR #225 (`46e079d`)
- [ADR-0014: Phase-J Maintainability Decomposition Boundaries](../adrs/0014-phase-j-maintainability-decomposition.md)
- comp's issue #212 review: ATM task `issue-212-hotspot-review`,
  2026-08-04
