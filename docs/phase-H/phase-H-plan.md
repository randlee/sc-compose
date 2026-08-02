---
id: phase-H
title: Reverse Extraction Format Extensions
status: planned
branch: plan/phase-h-rescope-formats-only
worktree: ../sc-compose-worktrees/plan/phase-h-rescope-formats-only
target: develop
---

# Phase H — Reverse Extraction Format and Boundary Extensions

## Objective

Extend the completed Phase-G known-template extraction feature for the three
missing file formats identified as customer use cases in GitHub issue #193:
JSON, YAML, and TOML. Phase H retains the pure library boundary, string-value
report model, structural provenance, and fail-closed diagnostics while adding
the approved format contracts one at a time.

This plan is documentation-only on the planning branch. H.1's contract
amendments are accepted; no runtime behavior is claimed until each
implementation sprint for that behavior passes its own gate.

## Source and scope

Issue #193 identifies three missing rendered-output format adapters against the
prototype at commit `50de332` and the Phase-G baseline at `5a4010b`:

- JSON rendered-output extraction;
- YAML rendered-output extraction;
- TOML rendered-output extraction.

The gap review confirmed all three against live source. Product-level
semantics still require the H.1 contract gate. The issue's XML mixed-content
and non-XML-prefix observations remain valid findings, but are explicitly
deferred to a future phase rather than included in Phase H.

## Sprint sequence

1. [Sprint H.1 — Reverse Extraction Format Contract](sprint-h-1-reverse-extraction-extension-contract.md)
   accepts the FR/architecture/ADR amendments, freezes the JSON/YAML/TOML
   semantics, and defines the shared raw-text matching core before
   implementation begins.
2. [Sprint H.2 — JSON Extraction Core](sprint-h-2-json-extraction-core.md)
   implements the pure Rust known-template JSON adapter and its report/path
   semantics.
3. [Sprint H.3 — JSON Cross-Surface Parity](sprint-h-3-json-cross-surface-parity.md)
   exposes the approved JSON contract through Python and CLI surfaces and
   proves parity with the library.
4. [Sprint H.4 — YAML Extraction](sprint-h-4-yaml-extraction.md)
   implements the approved known-template YAML adapter across the product
   surfaces.
5. [Sprint H.5 — TOML Extraction](sprint-h-5-toml-extraction.md)
   implements the approved known-template TOML adapter across the product
   surfaces.
6. [Sprint H.6 — Cross-Format Corpus and Adversarial Closure](sprint-h-6-cross-format-closure.md)
   validates JSON, YAML, and TOML against shared Rust, Python, and CLI
   fixtures and publishes the evidence needed to close the in-scope portion
   of issue #193.

The numbering is contiguous and intentional. H.2 follows H.1; H.3 depends on
H.2's library API; H.4 and H.5 may proceed in parallel after H.2/H.3 establish
the shared format/report patterns; and H.6 depends on all implementation
sprints. Each implementation sprint owns a complete boundary for its stated
format, except H.2, which additionally owns the one-time extraction of
format-neutral matching logic out of `xml.rs` into the shared core; XML's
structural and provenance ownership remains with `xml.rs` throughout. H.6 is
the phase closure gate and cannot silently absorb missing runtime work from an
earlier sprint. Confirmed findings from the H.6 fuzz
campaign route through the normal fix-assignment loop; Phase H reserves no
numbered sprint for follow-on fixes.

## Hard boundaries

- `sc-composer` remains pure and receives in-memory text only; it owns parsing,
  matching, reports, paths, and diagnostics.
- `sc-compose` owns file reads, format flags, output shaping, and exit codes;
  it must not duplicate extraction algorithms.
- Python remains a wrapper over the shared Rust semantics and must not grow a
  second extractor.
- Exact-Match Delimiter Scanning, Longest-Match-First Template-Init
  Replacement, and the Multi-Pass Brace-Count Delimiter Scheme remain the
  shared raw-text matching foundation. H.1 must define/confirm that JSON,
  YAML, and TOML value matching delegate to this core rather than implement
  independent format-specific text matchers.
- No format may infer original source types from rendered spelling alone.
- YAML extraction must skip the template's own frontmatter and operate on the
  rendered YAML body; it must remain distinct from YAML template-frontmatter
  and var-file decoding semantics.
- Best-effort/degraded parsing and a customer-facing cross-format raw-text mode
  are future-phase features. Phase H must not expose either mode, but its
  shared matching core must be reusable by them without redesign.

## Phase exit gate

Phase H is complete only when:

- H.1's accepted FR/architecture/ADR amendments remain mutually consistent;
- JSON, YAML, and TOML each have a documented known-template contract,
  production Rust support, Python/CLI parity, and corpus coverage;
- H.1 documents the single shared raw-text matching core and each format sprint
  demonstrably delegates value matching to it;
- all supported and rejected cases are represented in the cross-surface corpus
  and adversarial evidence;
- `cargo fmt --all --check`, `cargo test --workspace`,
  `cargo clippy --all-targets --all-features -- -D warnings`, the Python smoke
  suite, the repository boundary test, and `git diff --check` pass;
- quality-mgr, req-qa, and arch-qa can review each sprint directly from its
  authoritative document.

## Explicit non-goals

- unknown-template identification;
- loop or conditional reconstruction;
- typed-value recovery or arbitrary Jinja evaluation;
- XML mixed-content extraction and XML dirty-prefix tolerance;
- customer-facing best-effort/degraded parsing and cross-format raw-text mode;
- automatic template rewriting or input-file generation;
- ATM runtime dependencies, network access, or a second extraction algorithm.
