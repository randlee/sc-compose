---
id: S.11
title: sc-compose Go Native Module Adoption
status: planned
owner_repo: randlee/sc-compose
branch: sprint/s-11-sc-compose-go-native-module-adoption
depends_on:
  - S.9 remediation plan approval
  - S.10 merged to randlee/sc-publish develop
related_issue: https://github.com/randlee/sc-compose/issues/583
---

# Sprint S.11 — sc-compose Go Native Module Adoption

## Goal

Install the approved S.10 `go-native-module` peer package in `sc-compose` and
restore real `sc-sha-go` bundle verification for exactly the native targets
declared by `bindings/sc-sha-go/native/targets.toml`.

This sprint consumes the upstream package unchanged. All repository-specific
facts belong in the installer JSON/configuration; do not patch copied helper
logic or shared `sc-publish` scripts locally.

## Entry Gate

Before implementation, record in the sprint PR description:

- S.10 `sc-publish/develop` merge SHA and package version.
- The exact S.10 installer command and JSON input.
- The three expected `sc-sha-go` native targets from
  `bindings/sc-sha-go/native/targets.toml`: Linux x86_64, macOS arm64, and
  Windows GNU x86_64.

Do not start or merge S.11 against an unmerged S.10 branch, an unversioned
directory copy, or a locally modified package.

## Exact Targets in `randlee/sc-compose`

- Install/package-owned files:
  - `.github/scripts/go_native_module.py`
  - `.github/scripts/tests/test_go_native_module.py`
  - `release/go-native-module.toml`
- `.github/workflows/ci.yml`, only `sc-sha-go-plan` and `sc-sha-go` job steps
  that currently call removed `release_artifacts.py` commands.
- Existing workflow/source tests needed to prove the installed helper and
  matrix contract.

The following remain untouched unless an independent defect is found:

- `.github/scripts/release_artifacts.py`
- `.github/scripts/release_manifest.py`
- `release/publish-artifacts.toml`
- generic non-Go release matrix/workflow jobs

## Installation

Run the S.10 installer from the pinned upstream package with this consumer
payload:

```json
{
  "schema_version": 1,
  "source": "bindings/sc-sha-go",
  "cargo_package": "sc-sha-go",
  "artifact_prefix": "sc-sha-go"
}
```

Commit the installed assets exactly as generated. The package's copied helper
and test are vendored package assets: update them only through a newer S.10
package version/install, never by an independent local edit.

## Workflow Changes

1. In `sc-sha-go-plan`, replace the removed
   `release_artifacts.py go-native-target-matrix` call with
   `go_native_module.py target-matrix`, passing the core manifest and installed
   Go-native config. Export its JSON for the downstream matrix.
2. In `sc-sha-go`, retain target-specific values from the helper matrix:
   - use `matrix.goos` / `matrix.goarch` where the Go target identity is
     displayed or passed;
   - build `matrix.cargo_package`;
   - name/select the native archive from `matrix.library`;
   - use `matrix.artifact_prefix` for output paths and artifact names;
   - use `matrix.module` for the independent Go consumer and archive checks.
3. Replace the removed `release_artifacts.py stage-go-native-module` call with
   `go_native_module.py stage`.
4. Preserve existing bundled-module test, independent consumer test, archive
   round-trip test, and release-layout test. They are the consumer-level proof
   that staging is useful, not merely that the helper executed.

## Required Validation

- `python3 -m pytest .github/scripts/tests` runs the copied helper tests.
- A local fixture or workflow-equivalent invocation of `target-matrix` emits
  exactly the three declared target objects; it must not schedule macOS Intel
  or Windows MSVC.
- A local fixture stages each declared target with a dummy static archive and
  proves the expected module layout, then rejects a missing archive.
- `sc-sha-go-plan` and every `sc-sha-go` matrix leg are green on GitHub
  Actions.
- The existing Go bundle, independent-consumer, archive-round-trip, and
  release-layout workflow checks pass for every declared target.
- `just test`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` pass.

## Non-goals

- No generic release-matrix expansion.
- No addition of unsupported native targets.
- No local restoration of commands to `release_artifacts.py`.
- No change to publishing channels, release provenance, or public Go bindings
  beyond the already generated/bundled module layout.

## Closure Criteria

- [ ] S.10 merge SHA/package version is recorded and the install is
  byte-identical to the upstream package output.
- [ ] `sc-sha-go-plan` produces only the binding-declared native targets.
- [ ] Each declared target passes all existing downstream Go module consumer
  and archive verification.
- [ ] No core sc-publish asset or generic release behavior changed locally.
