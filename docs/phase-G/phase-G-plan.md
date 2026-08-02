---
id: phase-G
title: Reverse Template Extraction
status: planned
branch: integrate/phase-g
worktree: ../sc-compose-worktrees/integrate/phase-g
target: develop
---

# Phase G — Reverse Template Extraction

## Objective

Deliver a supported `sc-compose extract` feature, informed by prior
reverse-extraction research, for the most defensible use case: recovering
string bindings from a known `.xml.j2` template and a rendered XML document.
The feature must be deterministic, fail closed on ambiguous or unsupported
Jinja constructs, preserve the standalone crate boundaries, and expose enough
evidence for a caller to decide whether the result is trustworthy.

This phase does not claim that rendered output can uniquely reconstruct every
input to an arbitrary Jinja program. A rendered document loses information
about source types, loops, conditionals, filters, and omitted branches. The
supported contract therefore names a reversible template subset and reports
when an input falls outside it.

## Prior research findings carried into the contract

Prior reverse-extraction research is contract input, not a production
implementation. Its known limitations are phase requirements:

- extraction by the first matching tag returns the wrong value for repeated
  sibling tags;
- regex discovery recognizes only simple variable expressions and exact
  single-line shapes;
- the bulk harness skips all but one registered root type and validates only
  that `task_id` is non-empty;
- rendered scalar text cannot prove whether the original value was a string,
  number, boolean, or another type;
- malformed XML and unsupported control flow are not distinguished clearly.

## Supported Phase-G contract

The production feature supports a known template and rendered XML pair. The
initial reversible subset is:

- well-formed XML output, including XML declaration, comments, entities, and
  ordinary whitespace;
- scalar variable expressions in XML attribute values and text nodes;
- static prefix/suffix text around a scalar expression when the occurrence is
  structurally unambiguous;
- repeated sibling elements addressed by structural occurrence path rather
  than by the first matching tag;
- explicit include/exclude variable selection;
- a machine-readable report containing recovered rendered strings,
  occurrence provenance, confidence, warnings, and stable diagnostics.

The feature must reject or mark as unsupported, rather than fabricate values
for:

- loops whose iteration count or element-to-input mapping cannot be proven;
- conditionals and branches that omit a variable occurrence;
- filters, function calls, arithmetic, concatenation, indexing, and other
  non-scalar expressions outside the declared subset;
- namespace or structural ambiguity that prevents a unique occurrence path;
- attempts to infer original non-string types from rendered text alone.

## Public boundary

`sc-composer` owns pure template/output analysis and accepts in-memory text.
`sc-compose` owns CLI argument parsing, file reads, diagnostics formatting,
exit codes, and stdout/stderr behavior. No ATM integration, network access,
automatic edits, or output-file writes are introduced.

The implementation is expected to converge on an API equivalent to:

```rust
pub struct ExtractRequest<'a> {
    pub template: &'a str,
    pub rendered: &'a str,
    pub format: ExtractFormat,
    pub include: &'a [VariableName],
    pub exclude: &'a [VariableName],
}

pub struct ExtractionReport<P = OccurrencePathSegment, S = OccurrenceSource> {
    pub values: BTreeMap<VariableName, String>,
    pub occurrences: Vec<ExtractionOccurrence<P, S>>,
    pub confidence: f64,
    pub diagnostics: Vec<ExtractionDiagnostic>,
}

pub enum ExtractError {
    InvalidRequest { message: String },
    MalformedXml { diagnostic: ExtractionDiagnostic },
    UnsupportedSyntax { diagnostic: ExtractionDiagnostic },
    AmbiguousStructure { diagnostic: ExtractionDiagnostic },
}

pub fn extract(request: ExtractRequest<'_>)
    -> Result<ExtractionReport, ExtractError>;
```

The exact names may change during G.1, but the pure-library boundary,
string-value limitation, occurrence provenance, and fail-closed error model
are normative.

## Sprint sequence

1. [Sprint G.1 — Extraction Contract and Analysis Model](sprint-g-1-extraction-contract.md)
   establishes the requirement/ADR, public report types, supported subset, and
   unsupported/ambiguous diagnostic contract.
2. [Sprint G.2 — Deterministic XML Extraction Engine](sprint-g-2-xml-extraction-engine.md)
   implements structural occurrence matching and the XML subset without
   relying on first-tag heuristics.
3. [Sprint G.3 — Python Extraction Bindings](sprint-g-3-python-extraction-bindings.md)
   exposes the core extraction API to the first customer through the existing
   PyO3 adapter without duplicating extraction semantics.
4. [Sprint G.4 — CLI Extract Surface](sprint-g-4-cli-extract-surface.md)
   exposes the known-template workflow with text and JSON output, stable exit
   behavior, and file-boundary tests.
5. [Sprint G.5 — Corpus and Regression Closure](sprint-g-5-corpus-hardening.md)
   proves supported and intentionally rejected cases with deterministic Rust,
   Python, and CLI fixtures and updates stale research claims.
6. [Sprint G.6 — Adversarial Evidence and Hardening](sprint-g-6-adversarial-evidence.md)
   runs the bounded multi-agent campaign and records every candidate,
   classification, promotion, and unresolved owner for QA review.
7. [Sprint G.7 — Reject Dotted Extraction Expressions](sprint-g-7-dotted-expression-rejection.md)
   closes G.6's `FUZZ-template-probe-dotted-expression` finding by rejecting
   dotted expressions during XML extraction as unsupported syntax.

The sequence reflects API dependencies, not a prohibition on parallel plan
review. Each sprint has its own acceptance and validation gate; Phase G cannot
close until every gate passes and the evidence artifact accounts for the
supported, rejected, and inconclusive cases.

## Hard boundaries and dependencies

- Phase F's `integrate/phase-f` result is the baseline for implementation.
- `sc-composer` remains pure and must not read CLI paths, invoke subprocesses,
  depend on ATM, or own output formatting.
- `sc-compose` remains a thin CLI over the library; it must not duplicate the
  extraction algorithm.
- G.2 depends on the report/error contract from G.1; G.3 depends on the
  library API from G.2; G.4 depends on G.1 and G.2 only; G.3 is independent
  and does not gate G.4. G.5
  exercises the complete Rust/Python/CLI surface; G.6 depends on the corpus
  and regression gates from G.5.
- Prior research artifacts remain non-production input and must not be
  imported by production Rust or Python bindings.
- G.7 depends on G.6's evidence artifact; it has no other Phase-G dependency
  and does not gate or reopen G.1 through G.6.

## Phase exit gate

Phase G is complete only when:

- `sc-compose extract TEMPLATE.xml.j2 RENDERED.xml` works for the documented
  XML subset and returns provenance for every recovered value;
- repeated sibling occurrences do not alias to the first matching tag;
- unsupported Jinja constructs, malformed XML, and ambiguous structure are
  reported deterministically and never produce fabricated values;
- text and JSON CLI output preserve stdout cleanliness, stable diagnostics,
  and exit-code conventions;
- the Python adapter exposes the same extraction values, occurrence
  provenance, diagnostics, and unsupported-boundary semantics as Rust;
- corpus and adversarial evidence includes successful cases, intentional
  boundaries, unsupported cases, and inconclusive cases;
- dotted expressions (e.g. `user.name`) are rejected during extraction with
  `ERR_EXTRACT_UNSUPPORTED`, and G.6's
  `FUZZ-template-probe-dotted-expression` finding is closed;
- `cargo fmt --all --check`, `cargo test --workspace`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` pass;
- quality-mgr, plan-scope-reviewer, and critical-plan-reviewer can review the
  sprint docs and evidence without reconstructing scope from chat.

## Explicit non-goals and follow-on work

The following are not Phase-G closure requirements:

- identifying an unknown template from a directory of candidates;
- JSON or Markdown rendered-output adapters;
- reconstructing loop-produced arrays or conditional input branches;
- recovering original JSON/YAML scalar types without caller-provided hints;
- automatic template rewriting or input-file generation;
- ATM-specific wrappers or runtime dependencies.

These remain follow-on design candidates and must receive their own
requirements and sprint plans if product demand justifies them.
