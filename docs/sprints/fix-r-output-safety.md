---
id: FIX-R-OUTPUT-SAFETY
title: Phase R post-close output safety hardening
status: complete
branch: fix/r-output-safety
target: develop
---

# Sprint FIX-R-OUTPUT-SAFETY — Phase R Post-close Output Safety Hardening

## Root Cause

Phase R merged as [PR #562](https://github.com/randlee/sc-compose/pull/562)
at `8920f62`. Its dedicated production-readiness review subsequently found a
Blocking output-write vulnerability: a final output-path symlink could escape
the requested destination, including through a check-to-write race. The same
review found unbounded `bd` stdout/stderr capture, lossy non-UTF-8 path
conversion, and missing real-`bd` proof for the required-variable failure
path.

## Fix Design

- Reject a symlinked final output component and write through a sibling
  temporary file followed by replacement rename, so a symlink cannot redirect
  the write between validation and replacement.
- Bound each captured `bd` stream to 64 KiB and terminate a child that exceeds
  the limit.
- Reject non-UTF-8 paths before constructing `bd` argv.
- Prove the missing-required-variable path against the checksum-verified,
  pinned Beads binary with an isolated, explicitly authorized pour that is
  expected to fail before any bead persists.

## Scope

- [PR #563](https://github.com/randlee/sc-compose/pull/563) contains the
  implementation at `74956cf` and this documentation follow-up.
- The real-pour test exception is limited to an isolated negative-path test
  that deterministically fails validation before persistence. Ordinary
  non-dry-run pours remain outside test scope.

## Out of Scope

- Redesigning the process-runner trait solely to replace its bounded-output
  overflow marker with a typed runner result.
- Adding an elapsed-time timeout to the V1 runner; callers retain ownership of
  cancellation and deadlines.

## Acceptance Criteria

1. Final-component symlink and TOCTOU output-write regressions are rejected
   without touching the symlink target.
2. `bd` output is bounded and an overflowing child is terminated.
3. Non-UTF-8 paths fail before `bd` argv construction.
4. The pinned real-`bd` suite proves a required-variable failure without
   persistent Beads state.
5. Phase R's governing documents record the narrow negative-path exception
   and post-close review evidence.

## Closeout Evidence

- Phase R merge: [PR #562](https://github.com/randlee/sc-compose/pull/562),
  `8920f62`.
- Implementation: [PR #563](https://github.com/randlee/sc-compose/pull/563),
  `74956cf`.
- QA independently confirmed the output-safety fixes and watched all 17 PR
  checks pass, including pinned-Beads integration and installed Python-wheel
  jobs on Linux, macOS, and Windows.
- Documentation follow-up records the post-close finding and preserves the
  safety boundary for the one real-`bd` negative-path test.
