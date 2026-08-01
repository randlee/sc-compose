---
id: G.4
title: Corpus Hardening and Evidence
status: planned
branch: sprint/g-4-corpus-hardening
worktree: ../sc-compose-worktrees/sprint/g-4-corpus-hardening
target: develop
---

# Sprint G.4 — Corpus Hardening and Evidence

## Goal

Prove the complete Phase-G surface against realistic XML templates, minimized
edge cases, and adversarial inputs. Preserve confirmed defects as deterministic
tests, keep intentional unsupported boundaries visible, and update the user
documentation so the feature is not mistaken for a general inverse Jinja
engine.

## Hard dependencies

- G.1, G.2, and G.3 are available in the campaign baseline.
- The existing `.claude/skills/adversarial-fuzzing` workflow and its bounded
  report contract are available for the adversarial pass.
- Quality-mgr can inspect the committed evidence file without terminal
  scrollback or private corpus paths.

## Exact targets

- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `crates/sc-compose/tests/fixtures/reverse-extract/**`
- `docs/phase-G/evidence/g-4-reverse-extract-campaign.json`
- `prototype/reverse_extract/README.md`
- `docs/requirements.md` and `docs/architecture.md` only for evidence-backed
  contract corrections discovered during this sprint

## Deliverables

- `G4-D1` — A deterministic corpus covers direct scalar attributes/text,
  repeated sibling paths, static prefix/suffix, entities, whitespace,
  empty values, XML declarations/comments, malformed XML, unsupported Jinja,
  missing occurrences, and ambiguous structure.
- `G4-D2` — Run a bounded adversarial campaign using separate focuses for XML
  structure, template syntax, CLI/error boundaries, and differential/round
  trip behavior. Record seed, baseline, worker IDs, case limits, timeout,
  commands, and every worker result.
- `G4-D3` — Minimize each candidate, reproduce it three times, and classify it
  as `confirmed_bug`, `intentional_boundary`, or `inconclusive`. Confirmed
  in-scope bugs become deterministic tests; unsupported behavior remains
  visible rather than being relabeled as a pass.
- `G4-D4` — Replace stale prototype claims with documentation that points to
  the production CLI and states the known-template/XML-first limitation.
- `G4-D5` — Produce a committed evidence envelope that lets quality-mgr
  distinguish successful execution, no findings, intentional boundaries,
  inconclusive cases, and worker failures.

## Evidence contract

```json
{
  "phase": "G",
  "sprint": "G.4",
  "baseline": "<git-sha>",
  "seed": 7001,
  "target": "known-template-xml",
  "limits": {"workers": 4, "cases_per_worker": 50, "timeout_s": 120},
  "workers": [],
  "findings": [],
  "promoted_tests": [],
  "summary": {
    "confirmed_bugs": 0,
    "intentional_boundaries": 0,
    "inconclusive": 0,
    "failed_workers": 0
  }
}
```

The concrete report may add fields, but it must retain enough information to
reproduce every candidate and audit every worker outcome.

## This sprint does not close

- new extraction formats, unknown-template identification, or type inference;
- fixes for findings outside the documented XML subset unless a separate
  sprint is created;
- a no-finding claim when a worker fails, times out, or the corpus is
  unavailable;
- automatic production edits by adversarial workers.

## Acceptance criteria

- The committed corpus exercises every supported path and every documented
  intentional boundary.
- Every adversarial worker has a bounded, deterministic result; failures and
  timeouts remain visible in the evidence file.
- Every candidate is minimized and reproduced three times before promotion or
  being marked inconclusive.
- Confirmed in-scope bugs have deterministic tests in the owning crate suite.
- The prototype README no longer presents the research harness as the
  supported product interface.
- Quality-mgr can review the evidence and identify the next owner of every
  unresolved confirmed finding.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `python3 <skill-creator-root>/scripts/quick_validate.py .claude/skills/adversarial-fuzzing` when available
- `git diff --check`
