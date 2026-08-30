---
id: S.11
title: sc-compose Go Native Module Adoption
status: planned
owner_repo: randlee/sc-compose
branch: sprint/s-11-sc-compose-go-native-module-adoption
worktree: ../sc-compose-worktrees/sprint/s-11-sc-compose-go-native-module-adoption
target: integrate/phase-s
depends_on:
  - S.9 remediation plan approval
  - S.10 merged to randlee/sc-publish develop
related_issue: https://github.com/randlee/sc-compose/issues/583
---

# Sprint S.11 — sc-compose Go Native Module Adoption

## Goal

- Install the approved S.10 `go-native-module` peer package in `sc-compose`.
- Restore real `sc-sha-go` bundle verification for exactly the native targets
  declared by `bindings/sc-sha-go/native/targets.toml`.
- Preserve Go-binding release version lockstep after its optional config moves
  out of the core publish-artifact manifest.

This sprint consumes the upstream package unchanged. All repository-specific
facts belong in the installer JSON/configuration; do not patch copied helper
logic or shared `sc-publish` scripts locally.

## Hard Dependencies

- S.9's remediation plan and [ADR-0022](../adrs/0022-go-native-module-peer-package.md)
  are accepted.
- S.10 is merged to `sc-publish/develop`, with its `manifest.toml` package
  version and immutable merge SHA available to the installer.
- The branch is based on `integrate/phase-s` only after the S.10 external
  gate is satisfied.

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
  - `release/go-native-module-install.json`
- `release/sc-publish-install.json` — remove the stale, ignored `go_native`
  object; it must not become a second config source.
- `.github/workflows/ci.yml`, only `sc-sha-go-plan` and `sc-sha-go` job steps
  plus the existing core lockstep step.
- `.github/workflows/release-preflight.yml`
- `.github/workflows/release.yml`
- `.github/scripts/release_gate.sh`
- `bindings/sc-sha-go/tests/test_release_layout.py`
- `.github/scripts/tests/test_release_artifacts.py`
- `.github/scripts/tests/test_publish_kit_scripts.py`

The following remain untouched unless an independent defect is found:

- `.github/scripts/release_artifacts.py`
- `.github/scripts/release_manifest.py`
- `release/publish-artifacts.toml`
- generic non-Go release matrix/workflow jobs

## Deliverables

- An exact, versioned S.10 installation recorded by package version, immutable
  merge SHA/release tag, input JSON, and a clean re-install parity check.
- A single consumer-owned Go-native config source, with the legacy ignored
  `go_native` install input removed.
- Restored three-target `sc-sha-go` CI execution and release/preflight
  lockstep checks through the installed peer helper.
- Updated source-layout tests that exercise the installed helper rather than
  removed core-script commands.

## Required Work

1. Create `release/go-native-module-install.json` containing the S.10 v1
   input and run the pinned package installer. Commit its generated assets
   byte-for-byte; a second identical installation must produce no diff.
2. Remove the legacy `go_native` object from `release/sc-publish-install.json`.
   It is ignored by the core installer after Phase Q and must not silently
   diverge from the peer config.
3. Replace the two missing helper calls in `sc-sha-go-plan` and `sc-sha-go`.
   The plan job consumes `target-matrix`; the bundle job consumes `stage`.
4. Add the installed helper's `verify-version-lockstep` command immediately
   after the core lockstep check in CI, release preflight, release, and
   `release_gate.sh`. All four are required: a CI-only check would leave the
   publish path unprotected.
5. Rewrite `bindings/sc-sha-go/tests/test_release_layout.py` to load the
   installed `release/go-native-module.toml`, invoke the installed helper, and
   retain the cgo-loader, Windows-GNU, successful staging, and failure-path
   assertions. It must not import `release_artifacts.py` or read a removed
   `[go_native]` table.
6. Update the existing release workflow/gate test fixtures in
   `test_release_artifacts.py` and `test_publish_kit_scripts.py` so the new
   peer call is executable in every fixture. Add source assertions that CI,
   preflight, release, and release-gate each retain core lockstep and invoke
   peer lockstep exactly once after it.
7. Add source assertions that the plan matrix is exactly the three declared
   targets, each target uses its declared library and Go OS/arch, and no macOS
   Intel or Windows MSVC job is scheduled.

## Explicit Code Samples

Run the S.10 installer from the pinned upstream package with this consumer
payload:

```json
{
  "schema_version": 1,
  "package_version": "0.1.0",
  "source": "bindings/sc-sha-go",
  "cargo_package": "sc-sha-go",
  "artifact_prefix": "sc-sha-go"
}
```

Commit the installed assets exactly as generated. The package's copied helper
and test are vendored package assets: update them only through a newer S.10
package version/install, never by an independent local edit.

Every existing release guard retains its core check and immediately invokes
the peer check:

```bash
python3 .github/scripts/release_artifacts.py verify-version-lockstep \
  --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
python3 .github/scripts/go_native_module.py verify-version-lockstep \
  --config release/go-native-module.toml --workspace-toml Cargo.toml
```

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
   round-trip test, and the updated release-layout test. They are the
   consumer-level proof that staging is useful, not merely that the helper
   executed.
5. Add the peer lockstep command to CI, release-preflight, release, and
   `release_gate.sh` as described above.

## This Sprint Does Not Close

- A local modification to the S.10 helper/template or sc-publish core asset.
- Generic release-matrix expansion or support for macOS Intel/Windows MSVC.
- Go module publication, release channel changes, or a public binding API
  change.

## Acceptance Criteria

- [ ] S.10 provenance, install input, and generated-file parity are recorded
  and reproducible from an immutable package source.
- [ ] There is exactly one Go-native consumer config; the legacy ignored core
  install-input object is gone.
- [ ] The three current release gates and CI all run peer version lockstep.
- [ ] The source-layout test calls the installed helper and has no dependency
  on removed core subcommands or `[go_native]`.
- [ ] `sc-sha-go-plan` emits exactly Linux x86_64, macOS arm64, and Windows
  GNU x86_64, and all three downstream matrix jobs pass.

## Required Validation

- `python3 -m pytest .github/scripts/tests` runs the copied helper tests.
- A local fixture or workflow-equivalent invocation of `target-matrix` emits
  exactly the three declared target objects; it must not schedule macOS Intel
  or Windows MSVC.
- A local fixture stages each declared target with a dummy static archive and
  proves the expected module layout, then rejects a missing archive.
- A binding-only version mismatch in `bindings/sc-sha-go/Cargo.toml` fails the
  peer lockstep command; an unchanged workspace passes. Exercise the command
  through each of the CI/release/preflight/release-gate source paths.
- `sc-sha-go-plan` and every `sc-sha-go` matrix leg are green on GitHub
  Actions.
- The existing Go bundle, independent-consumer, archive-round-trip, and
  release-layout workflow checks pass for every declared target.
- `just test`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` pass.

## Closure Criteria

- [ ] S.10 merge SHA/package version is recorded and the install is
  byte-identical to the upstream package output.
- [ ] `sc-sha-go-plan` produces only the binding-declared native targets.
- [ ] Each declared target passes all existing downstream Go module consumer
  and archive verification.
- [ ] No core sc-publish asset or generic release behavior changed locally;
  the only added release-gate behavior is invocation of the installed peer
  lockstep command.
