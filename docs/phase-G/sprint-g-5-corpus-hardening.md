---
id: G.5
title: Corpus and Regression Closure
status: complete
branch: sprint/g-5-corpus-hardening
worktree: ../sc-compose-worktrees/sprint/g-5-corpus-hardening
target: develop
---

# Sprint G.5 — Corpus and Regression Closure

## Goal

Prove the supported Phase-G surface against realistic XML templates and
minimized edge cases. Promote confirmed deterministic regressions into the Rust,
Python, and CLI suites, keep intentional unsupported boundaries visible, and
update the user documentation so the feature is not mistaken for a general
inverse Jinja engine.

## Hard dependencies

- G.1 through G.4 are available in the campaign baseline.

## Exact targets

- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `crates/sc-composer/tests/fixtures/reverse-extract/**` (canonical corpus
  consumed by the Rust, Python, and CLI suites; binding-specific copies are
  intentionally not maintained)
- `bindings/python/tests/test_smoke.py`
- `docs/phase-G/evidence/g-5-corpus.json`
- `docs/requirements.md` and `docs/architecture.md` only for evidence-backed
  contract corrections discovered during this sprint

## Deliverables

- `G5-D1` — A deterministic corpus covers direct scalar attributes/text,
  repeated sibling paths, static prefix/suffix, entities, whitespace,
  empty values, XML declarations/comments, malformed XML, unsupported Jinja,
  missing occurrences, ambiguous structure, and the named
  `same-variable-conflicting-occurrences` case.
- `G5-D2` — Add Rust, Python, and CLI fixtures for supported values, repeated
  sibling paths, static prefix/suffix, entities, whitespace, empty values,
  malformed XML, unsupported Jinja, missing occurrences, ambiguity, and the
  `same-variable-conflicting-occurrences` case.

The canonical fixture directory is owned by `sc-composer` because it is the
shared semantic corpus. Python and CLI tests read those committed pairs
directly, which prevents byte-identical binding-specific copies from drifting
out of sync.
- `G5-D3` — Publish the deterministic corpus evidence and update requirements
  and architecture documentation to describe this as a from-scratch,
  research-informed known-template/XML-first feature. Remove stale claims that
  an uncommitted research harness is the product interface.

## This sprint does not close

- adversarial campaign execution or evidence publication;
- new extraction formats, unknown-template identification, or type inference;
- fixes for findings outside the documented XML subset unless a separate
  sprint is created.

## Acceptance criteria

- The committed corpus exercises every supported path and every documented
  intentional boundary.
- Rust, Python, and CLI fixtures agree on supported values and fail-closed
  boundary outcomes.
- The committed corpus evidence and requirements/architecture docs describe
  the supported product interface without relying on an uncommitted research
  harness.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
