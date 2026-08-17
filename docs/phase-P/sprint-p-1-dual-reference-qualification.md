---
id: P.1
title: Dual-Reference Released Product Qualification
phase: P
status: planned
target: develop
---

# Sprint P.1 — Dual-Reference Released Product Qualification

## Goal

Prove the released sc-lint 0.5.0 consumer configuration product works against
the current sc-compose and atm-core layouts before either real repository is
changed. This is a product-qualification gate, not a mock/fixture-only sprint.

## Hard dependencies

- accepted public sc-lint 0.5.0 consumer configuration contract and release
  artifact with checksum verification;
- Phase P plan approval;
- clean disposable copies at the recorded current `develop` commits for both
  consumer repositories.

## Exact targets

- `docs/phase-P/qualification/sc-compose-request.json` (sanitized, new)
- `docs/phase-P/qualification/atm-core-request.json` (sanitized, new)
- `docs/phase-P/qualification/dual-reference-matrix.md` (new)
- `docs/phase-P/qualification/<release-version>/` transcripts and JSON evidence
- `docs/adrs/` ADR-0016/0017 amendment decision record (new or approved update)

## Deliverables

- A release-artifact record: version, checksum manifest, platform target,
  installer/archive command, installed path, and `sc-lint --json version`
  evidence for Linux, macOS, and Windows.
- Two schema-valid requests generated from actual current repository facts, not
  copied templates. They disclose every requested lint/test/Just/CI selection
  and contain no executable shell string or sibling source path.
- Complete preview JSON, apply transcript, filesystem operation inventory, and
  reapply/no-op result for both disposable copies.
- A dual-reference matrix proving `just setup`, `just lint`, `just test`, and
  `just upgrade`, plus the corresponding CI workflow path, from the same
  release/config authority on all supported platforms.
- An accepted ADR amendment that updates or deliberately retains ADR-0016 and
  ADR-0017 semantics based on the real released contract.

## Acceptance criteria

- Every preview identifies all write candidates, retained files, reserved-recipe
  decisions, conflicts, and recovery actions. A human reviewer can compare it
  directly with the disposable-copy diff.
- Apply changes only the reviewed paths, leaves each README untouched, and does
  not use `cargo run`, a source archive, copied `.just` utility, or an ambient
  sc-lint executable.
- Reapplying the same request to each applied copy produces an explicit no-op
  result and no content diff.
- Existing `lint`, `test`, and `ci` recipes receive an explicit supported
  composition/migration plan or produce a no-write blocking product gap. A
  heuristic whole-file or recipe-name rewrite fails this sprint.
- The exact same release version passes the full matrix for both consumers. A
  one-consumer pass is insufficient.

## Required validation

- release checksum and `sc-lint --json version` verification per platform
- schema validation for both requests and all returned JSON envelopes
- preview/apply/reapply diff evidence on both disposable copies
- `just setup`, `just lint`, `just test`, `just upgrade` on both copies
- adapted/generated CI matrix on Linux, macOS, and Windows
- `git diff --check` and documentation-link validation

## This sprint does not close

- It does not modify either production consumer checkout.
- It does not hide a product gap with a consumer-side script or manual edit.
- It does not start P.2 or P.3 until the dual-reference matrix and ADR decision
  are accepted.
