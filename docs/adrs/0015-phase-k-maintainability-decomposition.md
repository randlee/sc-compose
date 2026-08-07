# ADR-0015: Phase-K Maintainability Decomposition Boundaries

## Status

Proposed

## Context

GitHub issue [#311](https://github.com/randlee/sc-compose/issues/311)
identified ten Repowise hotspots. Phase K selects eight concrete files for
behavior-preserving structural decomposition. The selected modules sit across
the pure `sc-composer` library, the `sc-compose` CLI, the diagnostics/error
surface, include confinement, report output, and Python adapter consumers.

Without an explicit decision, a maintainability refactor could accidentally
change public Rust paths, Python imports, CLI JSON, error text/codes, include
security policy, or report artifacts while claiming that only ownership moved.

## Decision

Phase K uses eight independently reviewable implementation sprints, K.1–K.8,
with the following frozen boundaries:

| Sprint | Boundary that must remain stable |
| --- | --- |
| K.1 | XML extraction values, paths, sources, diagnostics, limits, and `extract_xml`/report paths |
| K.2 | CLI commands, flags, exit codes, JSON envelopes, observer events, output paths, and newline behavior |
| K.3 | JSON/YAML var-file shapes, duplicate/integer/merge-key policy, diagnostics, and command errors |
| K.4 | `DiagnosticCode`, severity and serde spellings, `Diagnostic`, `DiagnosticEnvelope<T>`, and filesystem classification |
| K.5 | Error-family types, constructors, accessors, codes, messages, source chains, backtraces, hints, and conversions |
| K.6 | Include expansion graph/order, confinement, canonicalization, cycles, symlinks, depth, and diagnostics |
| K.7 | Discovery token sets, scope filtering, delimiter behavior, loop built-ins, and per-pass maps |
| K.8 | Report paths, metadata fields, artifact ordering, archive/latest policy, separators, and output errors |

New implementation modules remain private behind the existing crate and
crate-public paths. `sc-composer` remains a pure library; `sc-compose` remains
the CLI wrapper; `bindings/python` remains a thin adapter. No new protocol,
serialized schema, CLI flag, Python API, extraction feature, include policy,
or report format is introduced.

K.4 → K.5/K.6 is a recommended merge order, not a hard source-level
dependency when characterization proves existing exports remain stable. For an
out-of-order K.5 or K.6 start, the sprint owner records the K.4 export-stability
check and the plan-gate reviewer accepts that evidence first. K.7 remains
non-closed as decomposition work if characterization cannot demonstrate a
safe seam; for Phase K exit, QA may approve an abandon-evidence record with
the baseline result, rationale, and confirmation that `discovery.rs` stayed
unchanged in place of a merged split. `catalog.rs` and `resolver.rs` remain
excluded follow-on candidates.

## Consequences

- Characterization tests run before and after each code move, including Python
  smoke tests for the public `sc-composer` surfaces consumed by bindings.
- Re-exports and the boundary inventory make structural changes reviewable
  without inventing a new API.
- Repowise is a post-integration diagnostic; sprint closure is based on stable
  observable behavior and concrete ownership evidence.
- Any public-surface, schema, policy, or feature change requires a separate
  scoped decision rather than being hidden in Phase K.

## References

- [Phase K plan](../phase-K/phase-K-plan.md)
- [Phase K boundary contract](../phase-K/phase-k-boundary-contract.md)
- [Phase K machine-readable boundaries](../phase-K/phase-k-boundaries.json)
- [Issue #311](https://github.com/randlee/sc-compose/issues/311)
- [Requirements](../requirements.md)
- [Architecture](../architecture.md)
