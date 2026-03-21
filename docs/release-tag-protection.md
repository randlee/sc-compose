# Release Tag Protection Policy

This policy prevents accidental or premature `v*` tags.

## Goal

Ensure release tags are created only after PRs merge to `main` and release
gates pass.

## Required GitHub Ruleset

Create a tag ruleset for pattern:
- `v*`

Recommended settings:
1. Restrict tag creation to trusted actors only.
2. Deny deletion of release tags by default.
3. Deny force-update of existing release tags.

## Operational Contract

- Human release flow uses `workflow_dispatch` in `.github/workflows/release.yml`.
- Workflow validates publish order and version alignment before creating tags.
- Workflow creates tags from `origin/main`.
