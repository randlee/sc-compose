# Phase K Boundary Contract

## Status and authority

Status: proposed with the Phase K plan. This document records the existing
contracts that the eight structural decomposition sprints must preserve; it
does not add a runtime feature or a new public interface. The machine-readable
inventory is [phase-k-boundaries.json](phase-k-boundaries.json).

## Affected crate documentation

- Product requirements for both crates: [`docs/requirements.md`](../requirements.md).
- Product architecture and dependency direction: [`docs/architecture.md`](../architecture.md).
- Stable diagnostic codes and JSON-facing error contract:
  [`docs/error-code-registry.md`](../error-code-registry.md).
- Python adapter packaging and smoke-test surface:
  [`bindings/python/README.md`](../../bindings/python/README.md).
- Cross-platform filesystem and subprocess rules:
  [`docs/cross-platform-guidelines.md`](../cross-platform-guidelines.md).

These shared documents are the crate-level requirements/architecture authority:
the repository has no separate `crates/sc-composer` or `crates/sc-compose`
requirements document.

## Stable interfaces

### `sc-composer` Rust and Python-facing library surface

K.1, K.4, K.5, K.6, and K.7 may move implementation behind existing private
modules, but must preserve existing crate-root/module exports and the
signatures shown in their sprint documents. The Python adapter imports
`DiagnosticCode`, extraction types, error families, include types, and
discovery functions directly from `sc-composer`; the K.1/K.4–K.7 Python smoke
gate is therefore mandatory before and after each move.

### CLI and diagnostic protocol

K.2, K.3, K.4, and K.8 must preserve CLI flags, exit codes, JSON envelopes,
diagnostic code/severity/message/location fields, artifact paths, and metadata
fields. `DiagnosticEnvelope<T>` and `docs/error-code-registry.md` are the
protocol/schema authority. No schema migration is part of Phase K.

### Include confinement and filesystem policy

K.6 must retain `canonicalize_within_roots` as the single containment
implementation, the existing include graph ordering, and the documented
cross-platform path/error behavior. K.1, K.3, K.6, and K.8 must follow the
temporary-directory, `PathBuf`/`.join()`, bounded subprocess, and no hardcoded
Unix-path rules in `docs/cross-platform-guidelines.md`.

### Reporting output

K.8 preserves latest/archive layout, metadata serialization, artifact ordering,
path separators, and `OutputError` mapping. `catalog.rs` and report producer
behavior are explicitly outside the K.8 boundary.

## Non-applicable required documents

Phase K does not change ATM workflow, QA routing, triage prompts, or any
protocol/schema. Therefore process-QA/triage documents and a protocol migration
plan are not additional deliverables. The unchanged boundary and validation
contract is recorded here and in the machine-readable inventory for review.
