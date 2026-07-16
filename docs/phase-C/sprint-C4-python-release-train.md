---
id: C4
title: Python Release Train And Packaging Hardening
status: planned
branch: plan/maturin-bindings-implementation-plan
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/maturin-bindings-implementation-plan
---

# Sprint C4 — Python Release Train And Packaging Hardening

## Goal

Extend the main `sc-compose` release train so the Python adapter ships as a
first-class release channel after the adapter surface and cross-platform wheel
builds already exist.

This sprint is intentionally separate from C1 because it depends on live
release credentials and release-pipeline ownership that are not required to
prove the binding scaffold itself.

## Hard Dependencies

- [docs/phase-C/sprint-C1-maturin-bindings.md](./sprint-C1-maturin-bindings.md)
- [docs/architecture.md](../architecture.md)
- [docs/publishing.md](../publishing.md)
- [docs/publishing-agent.md](../publishing-agent.md)

## Exact Targets

- `.github/workflows/release.yml`
- `release/publish-artifacts.toml`
- `docs/publishing.md`
- `docs/publishing-agent.md`
- `docs/phase-C/sprint-C4-python-release-train.md`

## Deliverables

- add Python wheel and sdist build steps to the main release workflow
- add PyPI publish wiring to the main release workflow
- add Python artifact metadata to `release/publish-artifacts.toml`
- document the new `PYPI_API_TOKEN` requirement
- update release operator docs so PyPI is a required verification channel
- attach built wheels and sdist artifacts to GitHub Releases

## Acceptance Criteria

- `.github/workflows/release.yml` builds wheel artifacts on macOS, Linux, and
  Windows for tagged releases
- `.github/workflows/release.yml` builds one Python source distribution
- PyPI publication uses `MATURIN_PYPI_TOKEN` sourced from
  `PYPI_API_TOKEN`
- `release/publish-artifacts.toml` documents Python release artifacts
- `docs/publishing.md` and `docs/publishing-agent.md` include PyPI verification
  and secret requirements
- GitHub Releases attach Python wheels and the sdist beside existing release
  archives

## Required Validation

- release workflow YAML parses cleanly
- Python artifact metadata and operator docs are internally consistent
- a release dry-run or equivalent workflow validation passes before merge
