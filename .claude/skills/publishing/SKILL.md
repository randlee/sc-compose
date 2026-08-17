---
name: publishing
description: Coordinate a manifest-driven software release through a named ATM publisher teammate. Use when preparing release preflight, publishing a release, retrying a failed publish channel, or diagnosing release workflow readiness in the current repository.
---

# Publishing

Use the named `publisher` ATM teammate for production release work. Do not use
an unnamed background agent and do not create version-specific production
publisher identities. The shared release-state policy is
[`ref/release-state-strategy.md`](ref/release-state-strategy.md); read it
before selecting a branch, preflight location, or publish action.

## Start the publisher

1. Verify the required tools before delegation:

   ```bash
   command -v atm && atm --help
   command -v rmux && rmux --help
   ```

2. Confirm the roster has a named `publisher` teammate. Start one when needed;
   its production identity is exactly `publisher` for either runtime:

   ```bash
   rmux claude publisher --team <team-name> --model <claude-model>
   rmux codex publisher --team <team-name> --model <codex-model>
   ```

   The launch must establish `ATM_TEAM=<team-name>` and
   `ATM_IDENTITY=publisher`. Evaluation runs may use a distinct, clearly
   non-production identity.

3. Send a rendered [`preflight.xml.j2`](preflight.xml.j2) or
   [`publish.xml.j2`](publish.xml.j2) assignment through ATM. Require the
   immediate ACK, milestone status, and fenced JSON completion report from
   `publisher`.

## Durable evaluations

Run the applicable fresh-context evaluation after changing this skill,
`publisher.md`, the manifest helper, or release workflows. The durable cases
are [`evals/publisher-preflight.md`](evals/publisher-preflight.md) and
[`evals/publisher-recovery.md`](evals/publisher-recovery.md). They use
evaluation-only identities and must never create a production tag or publish.

## Operating rules

- Use the assignment's publishing manifest (normally
  `release/publish-artifacts.toml`) and `scripts/release_artifacts.py` as the
  only repository-specific publish surface.
- Complete readiness preflight before a `main` merge and final preflight on
  the exact `main` commit before publishing, as the shared policy requires.
- Treat all publish tokens as already-provisioned GitHub Actions secrets. Do
  not ask whether they exist, request them, inspect them, or substitute local
  credentials.
- Permit retry only for failed structured results. For a partial crates.io
  run, preserve the same tag and release ref; the idempotent manifest job skips
  live crates and retries only the missing crate set.
- Keep `publisher` accountable for the release and let it fan out only
  manifest-declared channel work to fungible one-channel teammates.
