---
id: S.10
title: sc-publish Go Native Module Package
status: planned
owner_repo: randlee/sc-publish
target_branch: develop
depends_on:
  - S.9 plan approval
blocks:
  - S.11 sc-compose Go Native Module Adoption
related_issue: https://github.com/randlee/sc-compose/issues/583
---

# Sprint S.10 — sc-publish Go Native Module Package

## Goal

Create a reusable, optional `go-native-module` peer package in `sc-publish`.
It restores target-aware Go native-module matrix selection and deterministic
module staging without extending the closed core `sc-publish` release manifest
or modifying vendored core release scripts.

This is an **upstream `sc-publish` sprint**. Its code and tests live in
`randlee/sc-publish`; this document records the required contract and gate for
the Phase S stack. S.10 is not a `gh stack` branch in `sc-compose`.

## Rationale

The Phase Q cutover removed sc-compose's local `go-native-target-matrix` and
`stage-go-native-module` commands. The generic `release-target-matrix` is not
an equivalent replacement: it schedules five release targets while a Go
native module may deliberately support only a subset. Restoring the commands
inside the vendored `release_artifacts.py` would fork canonical package logic.

`sc-publish/plugins/uniffi-bindgen-go` establishes the appropriate extension
seam: optional tool families are independent peer packages with their own
installer, release asset/configuration, and hermetic tests. `go-native-module`
must use the same seam.

## Exact Targets in `randlee/sc-publish`

- `plugins/go-native-module/README.md`
- `plugins/go-native-module/manifest.toml`
- `plugins/go-native-module/install.py`
- `plugins/go-native-module/go_native_module.py`
- `plugins/go-native-module/tests/test_install.py`
- `plugins/go-native-module/tests/test_go_native_module.py`

The installer copies these consumer assets:

- `.github/scripts/go_native_module.py`
- `.github/scripts/tests/test_go_native_module.py`
- `release/go-native-module.toml`, rendered from the explicit installer input

## Installer Contract

The installer accepts one JSON object. The first version contains only facts
that vary by consumer:

```json
{
  "schema_version": 1,
  "source": "bindings/sc-sha-go",
  "cargo_package": "sc-sha-go",
  "artifact_prefix": "sc-sha-go"
}
```

- Reject a non-object payload, missing fields, unknown schema version, empty
  strings, and paths escaping the consumer repository.
- `--dry-run` reports every destination and rendered file without writing.
- An actual install must be idempotent: a second install with the same input
  produces byte-identical outputs.
- Do not add a `[go_native]` table to core `release/publish-artifacts.toml`.
  The consumer-owned `release/go-native-module.toml` is the extension config.

## Helper Contract

The copied helper must use only the Python standard library and expose two
commands:

```text
python3 .github/scripts/go_native_module.py target-matrix \
  --manifest release/publish-artifacts.toml \
  --config release/go-native-module.toml

python3 .github/scripts/go_native_module.py stage \
  --config release/go-native-module.toml \
  --target <target> \
  --native-library <path> \
  --output <directory> \
  --version <semver>
```

`target-matrix` reads the binding-owned `<source>/native/targets.toml`, checks
each entry against `release/publish-artifacts.toml`, and emits JSON suitable
for GitHub Actions `fromJSON`. Every entry includes exactly:

- `target`, `os`, `archive`
- `goos`, `goarch`
- `library`, `cargo_package`, `module`, `artifact_prefix`

It must fail before emitting a partial matrix if a native target lacks a
matching release target, repeats a target, or has invalid/missing Go metadata.

`stage` creates a self-contained Go module under its output directory,
including `go.mod`, generated Go source, and the target-specific static native
archive at the Go package location. It rejects an absent input archive,
unknown target, unsafe output, and a module/version mismatch.

## Required Tests and Evidence

- Installer tests: valid install, dry-run, malformed JSON, invalid schema,
  unsafe path, and idempotent reinstall.
- Helper tests: supported three-target matrix, unsupported generic target,
  duplicate/missing mapping, malformed target entry, complete staged module,
  absent archive, unsafe output, and deterministic JSON output.
- The package test suite must run in a temporary consumer fixture, not only in
  the package source tree.
- Run the copied helper test from a temporary installed consumer layout.
- Open and merge a reviewed PR to `sc-publish/develop`; record its merge SHA
  and package version in S.11 before beginning S.11 implementation.

## Non-goals

- No sc-compose CI/workflow edit.
- No Go-module publication.
- No new native support for macOS Intel or Windows MSVC; support follows the
  consumer binding's explicit `native/targets.toml`.
- No changes to sc-publish core manifest schema or shared release scripts.

## Closure Criteria

- [ ] `plugins/go-native-module` is a peer package, not nested in
  `plugins/sc-publish` and not coupled to `uniffi-bindgen-go`.
- [ ] The installer and helper contracts above are implemented and fully
  covered by hermetic tests.
- [ ] S.10 is merged to `sc-publish/develop` with its final package version
  and merge SHA recorded for S.11.
- [ ] The package can be installed in a clean sc-compose fixture without
  modifying core publish-kit files.

