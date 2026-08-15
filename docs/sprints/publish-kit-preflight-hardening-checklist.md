---
id: PUBLISH-KIT-PREFLIGHT-HARDENING
status: complete
branch: feature/publish-kit-preflight-hardening
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feature/publish-kit-preflight-hardening
target: develop
---

## Closure Checklist

- [x] Make `release/publish-artifacts.toml` the source of truth for each
  post-release channel's dispatch workflow and fixed dispatch inputs.
- [x] Add manifest-helper commands for a tag-specific parallel channel
  dispatch plan and the non-disclosing preflight credential plan; validate
  malformed or unsupported channel configuration.
- [x] Make release preflight consume that plan, check only required credentials,
  authenticate crates.io and GitHub-destination tokens without outputting
  values, and inspect PyPI/TestPyPI environment-secret names without binding
  an approval-gated environment.
- [x] Replace the hard-coded published `sc-compose` renderer composite action
  with a generic manifest-configured renderer extractor and update Homebrew and
  Scoop workflows to use it.
- [x] Make the publisher agent, operator guide, and release checklist
  repository-neutral: no project/package/destination literals outside the
  manifest; require parallel, fungible channel teammates with structured
  results and retry-only-failed behavior.
- [x] Add regression tests for manifest plans, workflow contract, sanitized
  diagnostics, generic renderer extraction, and documentation/publisher
  contract.
- [x] Re-read the checklist and changed workflows; run focused tests, workflow
  YAML validation, `cargo fmt --all --check`, and `git diff --check`.

## Scope Guard

This work makes workflows and publishing guidance review-ready only. It must
not dispatch, tag, publish, or otherwise execute a release.
