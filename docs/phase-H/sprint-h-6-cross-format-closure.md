---
id: H.6
title: Cross-Format Corpus and Adversarial Closure
status: complete
branch: sprint/h-6-cross-format-closure
worktree: ../sc-compose-worktrees/sprint/h-6-cross-format-closure
target: develop
---

# Sprint H.6 — Cross-Format Corpus and Adversarial Closure

## Goal

- Prove the complete Phase-H extension surface against realistic customer
  templates and minimized boundary cases.
- Publish reviewable evidence for issue #193 and close only findings that have
  implementation, cross-surface parity, and regression coverage.

## Hard Dependencies

- H.2 through H.5 are merged and their individual validation gates pass.
- H.1 is complete and its accepted contract plus all format-specific amendments
  are available.
- `.claude/skills/adversarial-fuzzing/` remains the authoritative campaign and
  report protocol.

## Exact Targets

- `crates/sc-composer/tests/fixtures/reverse-extract/**`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `bindings/python/tests/test_smoke.py`
- `docs/phase-H/evidence/h-6-cross-format-campaign.json`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H6-D1 — Add one canonical shared corpus covering JSON, YAML, and TOML
  success, ambiguity, malformed, filtering, and unsupported cases.
- H6-D2 — Run the bounded adversarial campaign across Rust, Python, and CLI;
  record concrete templates, rendered inputs, expected relations, observed
  results, replays, classifications, and owners.
- H6-D3 — Promote every confirmed regression into the smallest owning suite;
  record intentional boundaries and inconclusive cases without claiming a
  no-finding result for failed workers.
- H6-D4 — Update the in-scope issue #193 closure evidence and final Phase-H
  documentation;
  leave no format or boundary behavior undocumented or silently deferred.

## Required Work

- Use separate worker tasks for JSON paths, YAML/TOML parser boundaries, and
  cross-surface differential behavior.
- Keep the HTML report and machine-readable evidence aligned with the shared
  adversarial-fuzzing protocol.
- Verify that all three public surfaces return equivalent report semantics and
  that no prototype or ATM runtime dependency entered production.

## Explicit Code Samples

```json
{
  "phase": "H",
  "sprint": "H.6",
  "formats": ["json", "yaml", "toml"],
  "findings": [],
  "promoted_tests": [],
  "summary": {"confirmed_defects": 0, "intentional_boundaries": 0, "inconclusive": 0}
}
```

The evidence schema must retain the concrete template and rendered input for
each candidate so a reviewer can judge whether it is a realistic use case.

## This Sprint Does Not Close

- New formats or policies outside the accepted H.1 contract.
- XML mixed-content extraction, XML dirty-prefix tolerance, and
  customer-facing best-effort/degraded or raw-text modes.
- Unknown-template identification, Jinja execution, loop reconstruction, or
  typed-value inference.
- A confirmed bug that lacks a requirement/ADR trace or an explicit owner.

## Acceptance Criteria

- All approved H.1 contracts have cross-surface fixtures and adversarial
  evidence.
- Every worker returns valid structured results or an explicit failed-worker
  record; every candidate has a classification and replay evidence.
- Confirmed bugs are promoted, intentional boundaries remain documented, and
  inconclusive cases have owners and next actions.
- Full workspace, binding, CLI, repository-boundary, and report validation
  passes with a clean diff.
- The in-scope portion of issue #193 can be closed from the committed evidence
  and docs alone; deferred XML and parsing-mode findings remain future-phase
  work.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `.claude/skills/adversarial-fuzzing/` quick validation when available
- `git diff --check`

## Completion Evidence

- Canonical corpus: `crates/sc-composer/tests/fixtures/reverse-extract/cross-format-corpus.json`;
  one manifest covers JSON, YAML, and TOML success, ambiguity, malformed,
  filtering, and unsupported cases.
- Campaign evidence: `docs/phase-H/evidence/h-6-cross-format-campaign.json`;
  four bounded workers, seed `193`, 36/36 passing cases, zero confirmed bugs,
  six intentional boundaries, and zero inconclusive candidates.
- Report package: `site/reports/20260803-1-fuzz-report.html`, its JSON sidecar,
  and four XHTML worker panels. The panels were rendered from
  `fuzz-run-agent.xhtml.j2`; the shell was rendered from
  `fuzz-run-report.html.j2` using `target/debug/sc-compose render`.
- `xmllint --noout` passed on all four XHTML panels. `npm exec --yes
  html-validate@11.6.1 -- site/reports/20260803-1-fuzz-report.html` passed.
- Optional H.5 backlog coverage was added: TOML occurrence-limit regression and
  Python TOML input-limit end-to-end coverage.
- Issue #193 closure scope: JSON, YAML, and TOML are supported and evidenced;
  XML mixed-content extraction, dirty-prefix stripping, and template
  identification remain explicitly deferred under the existing Phase-H
  requirements and ADR-0012.
