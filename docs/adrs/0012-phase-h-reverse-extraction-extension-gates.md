# ADR-0012: Phase-H Reverse-Extraction Extension Gates

## Status

Proposed

## Context

GitHub issue #193 records real customer use cases that exceed Phase G's
known-template, XML-first scalar contract. The gap review confirmed that the
current Rust library, Python binding, and CLI intentionally support only XML;
the prototype additionally identifies JSON, YAML, and TOML rendered-output
formats as missing adapters. The issue also records XML mixed-content and
leading-prefix behavior, but those two boundaries are deferred to a future
phase rather than included in Phase H.

Phase G's FR-16 and ADR-0011 remain correct and must not be weakened by
implementing an attractive prototype behavior without a contract. Each Phase-H
extension therefore needs an explicit decision about value representation,
provenance, ambiguity, malformed-input handling, and cross-surface exposure.

## Proposed decision

H.1 shall amend FR-16, the extraction architecture, and this ADR before any
runtime implementation sprint starts. The amendment shall resolve these gates:

1. JSON: whether placeholders may occur only in string values or also in object
   keys and complete non-string values; the canonical object/array path model;
   null, duplicate-key, ambiguity, and string-versus-typed-value behavior.
2. YAML: duplicate-key, alias/anchor, scalar, document-stream, and type
   preservation policy.
3. TOML: table/array-of-table paths, duplicate-key/parser errors, scalar
   rendering policy, and format selection in each public surface.
4. Shared raw-text matching: the exact seam for moving format-neutral value
   matching out of the current XML implementation, including delimiter
   scanning, template-segment parsing, static-prefix/suffix matching, capture
   boundaries, and adjacent-variable ambiguity handling. XML structural
   traversal and format-specific provenance remain in the XML adapter; JSON,
   YAML, and TOML must delegate candidate-value matching to the shared core.
   The core must be reusable by a future customer-facing raw-text or
   best-effort mode without redesign.

The accepted amendment shall retain these invariants:

- all extraction values remain rendered strings unless a future requirement
  explicitly adds typed recovery;
- ambiguous structural occurrences never silently overwrite a value;
- `sc-composer` remains in-memory and format-semantic, while CLI/Python remain
  adapters over it;
- malformed input remains distinguishable from unsupported syntax;
- the shared raw-text core is an internal foundation, not a Phase-H customer
  feature; and
- the three established delimiter decisions and the shared `VariableName`
  grammar are unchanged.

## Consequences

- H.2 through H.5 are blocked from implementation until H.1 changes this ADR
  from Proposed to Accepted and updates the linked normative documents.
- Format-specific adapters may share the generic `ExtractionReport`, but they
  must not introduce incompatible report models or format-specific Python
  algorithms.
- A format is not considered delivered merely because a parser can read it;
  Rust, Python, CLI, diagnostics, and corpus evidence must agree.
- H.1 must leave XML's current behavior covered while identifying the first
  implementation seam that extracts its format-neutral matcher into shared
  code. The future customer-facing raw-text/best-effort modes—including
  arbitrary text such as Markdown—are named requirements for a later phase,
  not Phase-H features or numbered sprints.

## Explicit future-phase non-goals

Phase H does not design or expose:

- a best-effort/degraded parser that recovers values from structurally modified
  or partially corrupt documents; or
- a customer-facing cross-format raw-text mode for arbitrary rendered text,
  including Markdown, XML, YAML, JSON, or TOML.

Both future modes must reuse the shared raw-text matching core defined by H.1.

## References

- [FR-16](../requirements.md#fr-16-known-template-reverse-extraction-v11-phase-g1)
- [ADR-0011](0011-reverse-extract-known-template-contract.md)
- [Phase-H plan](../phase-H/phase-H-plan.md)
- [GitHub issue #193](https://github.com/randlee/sc-compose/issues/193)
