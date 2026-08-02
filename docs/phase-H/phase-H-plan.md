---
id: phase-H
title: Reverse Extraction Format and Boundary Extensions
status: planned
branch: plan/phase-h-extraction-gaps
worktree: ../sc-compose-worktrees/plan/phase-h-extraction-gaps
target: develop
---

# Phase H — Reverse Extraction Format and Boundary Extensions

## Objective

Extend the completed Phase-G known-template extraction feature for the real
customer use cases documented in GitHub issue #193. Phase H retains the pure
library boundary, string-value report model, structural provenance, and
fail-closed diagnostics while adding explicitly approved format and input
policies one at a time.

This plan is documentation-only on the planning branch. No runtime behavior is
claimed until H.1 accepts the contract amendments and the implementation sprint
for that behavior passes its own gate.

## Source and scope

Issue #193 identifies five gaps against the prototype at commit `50de332` and
the Phase-G baseline at `5a4010b`:

- mixed-content/block-text XML extraction;
- TOML rendered-output extraction;
- YAML rendered-output extraction;
- JSON rendered-output extraction;
- narrowly specified tolerance for non-XML prefixes before a rendered XML
  document.

The gap review confirmed all five against live source. The prototype's block
text and prefix handling are reference behavior, not an authority for
semantics. The customer-use-case condition for every listed format is treated
as satisfied; product-level semantics still require the H.1 contract gate.

## Sprint sequence

1. [Sprint H.1 — Reverse Extraction Extension Contract](sprint-h-1-reverse-extraction-extension-contract.md)
   accepts the FR/architecture/ADR amendments and freezes the semantics for
   every gap before implementation begins.
2. [Sprint H.2 — JSON Extraction Core](sprint-h-2-json-extraction-core.md)
   implements the pure Rust known-template JSON adapter and its report/path
   semantics.
3. [Sprint H.3 — JSON Cross-Surface Parity](sprint-h-3-json-cross-surface-parity.md)
   exposes the approved JSON contract through Python and CLI surfaces and
   proves parity with the library.
4. [Sprint H.4 — XML Mixed-Content Extraction](sprint-h-4-xml-mixed-content.md)
   implements the approved block-text/mixed-content representation without
   weakening structural matching.
5. [Sprint H.5 — XML Dirty-Prefix Policy](sprint-h-5-xml-dirty-prefix-policy.md)
   implements the approved preamble policy while preserving malformed and
   multiple-root rejection.
6. [Sprint H.6 — YAML Extraction](sprint-h-6-yaml-extraction.md)
   implements the approved known-template YAML adapter across the product
   surfaces.
7. [Sprint H.7 — TOML Extraction](sprint-h-7-toml-extraction.md)
   implements the approved known-template TOML adapter across the product
   surfaces.
8. [Sprint H.8 — Cross-Format Corpus and Adversarial Closure](sprint-h-8-cross-format-closure.md)
   validates all approved extensions against shared Rust, Python, and CLI
   fixtures and publishes the evidence needed to close issue #193.

The numbering is contiguous and intentional. H.2 and H.4 through H.7 may be
staffed independently after H.1's contract is accepted; H.3 depends on H.2's
library API, and H.8 depends on all implementation sprints. Each
implementation sprint owns a complete boundary for its stated format or
policy. H.8 is the phase closure gate and cannot silently absorb missing
runtime work from an earlier sprint.

## Hard boundaries

- `sc-composer` remains pure and receives in-memory text only; it owns parsing,
  matching, reports, paths, and diagnostics.
- `sc-compose` owns file reads, format flags, output shaping, and exit codes;
  it must not duplicate extraction algorithms.
- Python remains a wrapper over the shared Rust semantics and must not grow a
  second extractor.
- Exact-Match Delimiter Scanning, Longest-Match-First Template-Init
  Replacement, and the Multi-Pass Brace-Count Delimiter Scheme are unchanged.
- The H.1 amendment must explicitly preserve the Phase-G scalar-only behavior
  for XML unless H.4's mixed-content contract is accepted as a narrowly
  defined extension.
- No format may infer original source types from rendered spelling alone.
- No prefix policy may turn malformed XML, multiple roots, or hostile content
  into a successful extraction without an explicit diagnostic and test.

## Phase exit gate

Phase H is complete only when:

- H.1's FR/architecture/ADR amendments are accepted and agree;
- JSON, YAML, and TOML each have a documented known-template contract,
  production Rust support, Python/CLI parity, and corpus coverage;
- the approved mixed-content XML behavior is deterministic and provenance is
  reviewable;
- the approved dirty-prefix policy is narrow, observable, and still fail-closed
  for malformed XML and multiple roots;
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
- automatic template rewriting or input-file generation;
- ATM runtime dependencies, network access, or a second extraction algorithm.
