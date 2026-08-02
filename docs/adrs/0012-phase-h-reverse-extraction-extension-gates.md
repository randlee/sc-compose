# ADR-0012: Phase-H Reverse-Extraction Extension Gates

## Status

Proposed

## Context

GitHub issue #193 records real customer use cases that exceed Phase G's
known-template, XML-first scalar contract. The gap review confirmed that the
current Rust library, Python binding, and CLI intentionally support only XML;
the prototype additionally demonstrates mixed-content block extraction and a
narrow leading-prefix cleanup policy, while listing other formats as planned.

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
2. XML mixed content: whether a block value is plain descendant text, canonical
   serialized inner XML, or another representation; how child markup is
   preserved and how one occurrence is identified.
3. Dirty XML prefixes: the exact accepted preamble grammar, root selection,
   comments/declarations, multiple-root handling, security limits, and the
   diagnostic/evidence representation of discarded bytes.
4. YAML: duplicate-key, alias/anchor, scalar, document-stream, and type
   preservation policy.
5. TOML: table/array-of-table paths, duplicate-key/parser errors, scalar
   rendering policy, and format selection in each public surface.

The accepted amendment shall retain these invariants:

- all extraction values remain rendered strings unless a future requirement
  explicitly adds typed recovery;
- ambiguous structural occurrences never silently overwrite a value;
- `sc-composer` remains in-memory and format-semantic, while CLI/Python remain
  adapters over it;
- malformed input remains distinguishable from unsupported syntax;
- the three established delimiter decisions and the shared `VariableName`
  grammar are unchanged.

## Consequences

- H.2 through H.7 are blocked from implementation until H.1 changes this ADR
  from Proposed to Accepted and updates the linked normative documents.
- Format-specific adapters may share the generic `ExtractionReport`, but they
  must not introduce incompatible report models or format-specific Python
  algorithms.
- A format is not considered delivered merely because a parser can read it;
  Rust, Python, CLI, diagnostics, and corpus evidence must agree.

## References

- [FR-16](../requirements.md#fr-16-known-template-reverse-extraction-v11-phase-g1)
- [ADR-0011](0011-reverse-extract-known-template-contract.md)
- [Phase-H plan](../phase-H/phase-H-plan.md)
- [GitHub issue #193](https://github.com/randlee/sc-compose/issues/193)
