# ADR-0025: Complete Platform Delivery and Python Stable ABI

## Status

Accepted user decisions, recorded 2026-11-04; implementation is pending.
This records existing Phase U rulings, not a new implementation approval.

## Context

Interpreter-specific fixed-version wheels prevent consumers from using newer
Python runtimes. The user selected CPython 3.11 as the stable ABI floor for
all three packages and requires complete platform/artifact delivery. The
lead relayed the floor in ATM message 01M6JAJWE373EJC3EV62AZHE14; the live
Phase U root records the platform/channel and integration-only matrix rulings.

## Decision

FR-25 governs the six OS/architecture targets, five artifact families,
channels, all-three abi3-py311 build/metadata contract and runtime matrix.
Do not impose a Python upper bound or silently drop a Go target. macOS CLT
Python 3.9 users need a newer interpreter. Binding adapters stay thin under
ADR-0018/0020/0021; Go-native ownership/installation stays ADR-0022.

Build a candidate's intended release-version files once, test those bytes,
retain them with their successful workflow identity and verify all SHA-256
values at preflight and immediately before publication. The canonical kit
must support selecting/staging the tested files; no post-gate rebuild may
inherit old evidence. Route missing capability to the sc-publish owner and
adopt its supported immutable release, never patch consumer copies. U5 owns
that dependency coordination/adoption, U8 the producer, U9 the real rehearsal.
A full workflow_dispatch matrix for the final publication candidate is
mandatory after later code changes, even if U8's upgrade run passed.

## Consequences

The 30 artifact/platform cells and declared runtime/install cells require
actual outcomes. Unsupported targets and missing upstream capabilities are
visible blockers. Normal CI remains small. Phase U performs private/dry-run
publication-path rehearsal and does not publish. Planning/QA approval does
not turn pending runtime or packaging work into delivered capability.
