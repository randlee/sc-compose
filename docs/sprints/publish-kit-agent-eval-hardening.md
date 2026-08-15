---
id: PUBLISH-KIT-AGENT-EVAL-HARDENING
status: complete
branch: feature/publish-kit-preflight-hardening
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feature/publish-kit-preflight-hardening
target: develop
---

## Context

PR #507 landed the manifest-driven publish kit, non-disclosing credential
preflight, and per-channel independent retry (see
`docs/publish-kit-requirements.md`, requirements 1-7; QA PASS confirmed on
commit `ed8f879`). This follow-up closes three gaps identified after PR #507's
QA pass:

1. Preflight coverage stops at the `publisher` agent's own gate; it is not
   extended through every publishing subagent/channel worker.
2. `docs/publish-kit-requirements.md` is a standalone document — it is not
   linked from the repo's top-level requirements/ADR documentation, so the
   decision record is discoverable only by accident.
3. There is no written eval plan for exercising the publishing agents
   end-to-end (Rand verified agent behavior manually via ad hoc "preflight
   mode" runs; that verification procedure needs to be a durable, repeatable
   document, not tribal knowledge).

## Closure Checklist

- [x] **Extend preflight through all publishing agents/subagents.** Update
  `.claude/agents/publisher.md` and every per-channel publishing
  agent/subagent prompt (crates.io, GitHub Release, Homebrew, Scoop,
  `winget`, PyPI) so that each channel worker itself checks/consumes the
  non-disclosing preflight result for its own required credentials before
  attempting any publish action, not just the root `publisher` gate before
  the release workflow starts. Match the language and behavior Rand and comp
  already worked out directly (do not re-derive from scratch — if any detail
  is ambiguous, make the smallest reasonable extension consistent with
  `docs/publish-kit-requirements.md` requirement 4-5 and flag the assumption
  in the completion report).
- [x] **Link requirements/ADR into top-level documents.** Add
  `docs/publish-kit-requirements.md` into the repo's normal req/ADR
  structure: either promote it into (or cross-link it from)
  `docs/requirements.md`, and/or add a proper ADR under `docs/adrs/`
  recording the manifest-ownership boundary and non-disclosing-preflight
  design as an architectural decision, following the existing ADR numbering
  and format in `docs/adrs/`. The standalone file must no longer be an
  orphan — a reader starting from `docs/requirements.md` or `docs/adrs/`
  must be able to find it.
- [x] **Write an agent eval plan.** Add a durable, repeatable eval plan
  (e.g. `docs/eval/publishing/publish-kit-agent-eval-plan.md`) that lets team-lead or comp
  verify all publishing agents/subagents behave correctly without executing
  a real release: preflight-only dry runs per channel, expected
  pass/fail/skip outcomes, how to simulate a missing/revoked credential
  without using a real secret, and how to confirm no channel ever discloses
  a token value. Base this on the manual "preflight mode" verification Rand
  already ran directly against the agents.
- [x] Add or update tests covering the extended per-subagent preflight
  behavior where testable in code (not just agent-prompt text).
- [x] Re-read the checklist and changed files; run `cargo fmt --all --check`,
  `cargo test --workspace`, and any workflow YAML validation already used by
  this branch.

## Scope Guard

This work makes agent prompts, documentation linkage, and eval procedure
review-ready only. It must not dispatch, tag, publish, or otherwise execute
a release.

## Validation Evidence

Baseline validation was independently confirmed on commit
`fca2733ae741dc49776a22f55a043796768683fc`: `cargo fmt --all --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo test --workspace` all passed.

The QA-RECHECK fix commit `9f866768b0441e60c91563b5a235fddf69897df5` was
independently re-validated by comp (`cargo fmt --all --check`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`cargo test --workspace`, `scripts/tests/test_release_artifacts.py` all
passed, worktree clean) and confirmed clean again by quality-mgr's
`rust-qa-agent` during the QA-RECHECK pass (see
`/Users/randlee/.atm/.config/atm/share/sc-compose/pk-eval-qa-recheck-atm.txt`).
The no-release scope guard held throughout: no tag, dispatch, or publish
action was taken.
