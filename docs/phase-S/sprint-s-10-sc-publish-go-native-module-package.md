---
id: S.10
title: sc-publish Go Native Module Package
status: planned
branch: sprint/s-10-go-native-module-package
worktree: ../sc-publish-worktrees/sprint/s-10-go-native-module-package
target: randlee/sc-publish:develop
owner_repo: randlee/sc-publish
depends_on:
  - S.9 plan approval
  - ADR-0022 accepted
blocks:
  - S.11 sc-compose Go Native Module Adoption
related_issue: https://github.com/randlee/sc-compose/issues/583
---

# Sprint S.10 — sc-publish Go Native Module Package

## Goal

- Create a reusable, optional `go-native-module` peer package in `sc-publish`.
- Restore target-aware Go native-module matrix selection, staging, and release
  version-lockstep verification without extending the closed core
  `sc-publish` release manifest or modifying vendored core release scripts.
- Deliver an immutable, fully tested package contract that S.11 can install
  unchanged into `sc-compose`.

This is an **upstream `sc-publish` sprint**. Its code and tests live in
`randlee/sc-publish`; this document records the required contract and gate for
the Phase S stack. S.10 is not a `gh stack` branch in `sc-compose`.

## Hard Dependencies

- S.9's approved remediation decision.
- [ADR-0022: Go Native Module Peer Package Ownership and Release
  Validation](../adrs/0022-go-native-module-peer-package.md) is Accepted
  before production source is authored.
- `sc-publish/develop` and its existing peer-package convention, represented
  by `plugins/uniffi-bindgen-go`.

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
- `plugins/go-native-module/release/go-native-module.toml.j2`
- `plugins/go-native-module/tests/test_install.py`
- `plugins/go-native-module/tests/test_go_native_module.py`
- `randlee/sc-publish/.github/workflows/ci.yml` — add a test step that runs
  this peer package's unit tests and a temporary installed-consumer fixture.

The installer copies these consumer assets:

- `.github/scripts/go_native_module.py`
- `.github/scripts/tests/test_go_native_module.py`
- `release/go-native-module.toml`, rendered from the explicit installer input

## Deliverables

Every deliverable below is production-ready for the peer-package boundary; no
consumer-local helper, template, or validation fork is permitted.

- An Accepted ADR-0022 defining ownership, source provenance, schema
  compatibility, and the core-versus-peer validation boundary.
- A versioned S.10 package with a strict JSON install contract, a rendered
  consumer config, and byte-identical installed helper/test assets.
- Three peer-helper commands: `target-matrix`, `stage`, and
  `verify-version-lockstep`.
- sc-publish CI proof for both package-source and installed-consumer layouts.

## Required Work

1. Create the peer package parallel to `plugins/uniffi-bindgen-go`, never
   nested in `plugins/sc-publish`.
2. Create the Jinja config template and make `install.py --input <json>
   <consumer-root>` validate the entire v1 schema, render the config, copy the
   helper and tests through the same pinned `sc-compose` rendering contract
   used by the existing `plugins/sc-publish` installer, and support a
   non-mutating `--dry-run`. Do not create a second template renderer.
3. Use `schema_version`, `package_version`, `source`, `cargo_package`, and
   `artifact_prefix` in the v1 installer input. `package_version` is an
   expected-package guard: it must equal `manifest.toml`'s version. The final
   rendered config contains only the three consumer facts. Derive and validate
   the Go module from `<source>/go.mod`, the generated package path from
   `<source>/native/targets.toml`'s `[contract].generated_package`, and target
   identity from that file's `[[targets]]` entries. Do not duplicate these
   binding-owned facts in JSON.
4. Make `target-matrix` read `[[release_targets]]` only for runner and archive
   data, join it to the binding-owned targets by Rust target, and emit exactly
   `{"include":[...]}`. It must fail without stdout on any invalid entry or
   incomplete join.
5. Make `stage` copy exactly `go.mod`, `README.md`, `go/`, `testdata/`,
   `native/targets.toml`, one selected native archive, and `VERSION`. Reject
   pre-existing output, unsafe source/output paths, missing required source
   files, unknown target, or an archive whose filename differs from the target
   contract; no failed invocation may leave a partial output directory.
6. Make `verify-version-lockstep` compare the workspace version with the
   binding Cargo manifest's inherited workspace version and validate the
   installed config/source relationship. This replaces the coverage lost when
   the optional module leaves core `publish-artifacts.toml`.
7. Add the package test commands to sc-publish CI. The installed-consumer test
   must execute copied assets from a temporary consumer root rather than
   importing the package source tree.

## Explicit Code Samples

The exact S.10 input contract is:

The installer accepts one JSON object. The first version contains only facts
that vary by consumer:

```json
{
  "schema_version": 1,
  "package_version": "0.1.0",
  "source": "bindings/sc-sha-go",
  "cargo_package": "sc-sha-go",
  "artifact_prefix": "sc-sha-go"
}
```

The rendered config contains only the install facts, not a second target list:

```toml
schema_version = 1
source = "bindings/sc-sha-go"
cargo_package = "sc-sha-go"
artifact_prefix = "sc-sha-go"
```

The S.11 installer invocation must be reproducible from an immutable S.10
release tag or merge SHA:

```bash
python3 <pinned-sc-publish>/plugins/go-native-module/install.py \
  --input release/go-native-module-install.json .
```

The peer helper CLI is:

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

python3 .github/scripts/go_native_module.py verify-version-lockstep \
  --config release/go-native-module.toml \
  --workspace-toml Cargo.toml
```

## Installer Contract

- Reject a non-object payload, missing fields, unknown schema version, empty
  strings, and paths escaping the consumer repository.
- Reject an input `package_version` that differs from the peer package's
  `manifest.toml`; print the observed version in successful install output.
- `--dry-run` reports every destination and rendered file without writing.
- An actual install must be idempotent: a second install with the same input
  produces byte-identical outputs.
- Do not add a `[go_native]` table to core `release/publish-artifacts.toml`.
  The consumer-owned `release/go-native-module.toml` is the extension config.
- The package manifest version plus the immutable source commit/release tag is
  the install provenance. S.11 records both and verifies a clean re-install
  produces no diff.

## Helper Contract

The copied helper must use only the Python standard library and expose the
three commands in the explicit contract above.

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
unknown target, unsafe output, and an empty or malformed release version.

## This Sprint Does Not Close

- sc-compose workflow adoption, which is exclusively S.11 work.
- Publishing a Go module or adding new native targets.
- Any change to sc-publish core manifest schema or shared release scripts.

## Acceptance Criteria

- [ ] ADR-0022 is Accepted before package implementation begins.
- [ ] The v1 installer schema, rendered TOML schema, CLI output schema, and
  immutable install provenance are documented and enforced.
- [ ] The package emits an exact `include` matrix and fails closed without
  partial stdout/output on malformed input.
- [ ] The package is versioned, tested by sc-publish CI, and merged to
  `sc-publish/develop`.

## Required Validation

- Installer tests: valid install, dry-run, malformed JSON, invalid schema,
  wrong package version, unsafe path, and idempotent reinstall.
- Helper tests: supported three-target matrix, unsupported generic target,
  duplicate/missing mapping, malformed target entry, runner/target mismatch,
  exact `{"include": [...]}` JSON, no stdout on failure, complete staged
  module, absent/wrong-name archive, existing/unsafe output, missing source
  asset, malformed `go.mod`, and deterministic JSON output.
- Lockstep tests: a binding-only Cargo version drift fails while the unchanged
  workspace passes; invalid source/package/config relationships fail.
- The package test suite must run in a temporary consumer fixture, not only in
  the package source tree.
- Run the copied helper test from a temporary installed consumer layout.
- Open and merge a reviewed PR to `sc-publish/develop`; record its merge SHA
  and package version in S.11 before beginning S.11 implementation.

## Closure Criteria

- [ ] `plugins/go-native-module` is a peer package, not nested in
  `plugins/sc-publish` and not coupled to `uniffi-bindgen-go`.
- [ ] The installer and helper contracts above are implemented and fully
  covered by hermetic tests.
- [ ] S.10 is merged to `sc-publish/develop` with its final package version
  and merge SHA recorded for S.11.
- [ ] The package can be installed in a clean sc-compose fixture without
  modifying core publish-kit files.
