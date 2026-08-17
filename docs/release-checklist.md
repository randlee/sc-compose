# Release Checklist

Use this checklist with the repository's manifest-driven publish kit.

## Before Release

- [ ] Confirm the requested version is represented consistently by the
  manifest-declared workspace, crates, and Python packages.
- [ ] Validate the manifest:
  ```bash
  python3 scripts/release_artifacts.py validate-manifest \
    --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
  ```
- [ ] Run `Release Preflight`; do not inspect, request, or locally substitute a
  credential.
- [ ] Confirm preflight passed before any release dispatch.
- [ ] Preserve the complete per-channel sanitized preflight result. A denied
  overall result is fail-closed, but it must include every independent check
  rather than stopping at the first failure.

## Root Release

- [ ] Dispatch the root release workflow only with explicit authorization.
- [ ] Verify it created the immutable GitHub Release from the workflow-owned
  tag.

## Parallel Channels

- [ ] Produce the channel plan for the immutable tag:
  ```bash
  python3 scripts/release_artifacts.py channel-dispatch-plan \
    --manifest release/publish-artifacts.toml --tag v<VERSION>
  ```
- [ ] Fan out the named channel publisher from each returned dispatch plan concurrently.
- [ ] Give every worker its matching preflight contract and completed
  non-disclosing preflight result; deny that channel if either is missing,
  failed, stale, or mismatched.
- [ ] For every channel with `credential_rehearsal` in the plan, complete its
  manifest-declared safe rehearsal before that channel's production dispatch.
- [ ] Collect one structured result for each channel.
- [ ] Retry only failed channels with the same tag and manifest inputs.
- [ ] Record the final per-channel verification facts and sanitized failures,
  if any, for `team-lead`.
