# Go Native Module Remediation

## Decision

Restore the `sc-sha-go` CI bundle verification through a new peer package,
`plugins/go-native-module`, in `sc-publish`. The package owns the Go-native
helper, its installation contract, and its tests; `sc-compose` installs and
invokes that package. Do **not** add the removed commands back to the vendored
`.github/scripts/release_artifacts.py`, and do **not** add a `[go_native]`
table to the generated `release/publish-artifacts.toml` manifest.

This follows the existing peer-package seam used by
`plugins/uniffi-bindgen-go`. The core sc-publish installer deliberately has a
strict, closed manifest schema, so a peer package is the appropriate extension
point for an optional artifact family that only some consumers need.

## Confirmed Current State

Issue [#583](https://github.com/randlee/sc-compose/issues/583) is correct
about the immediate failure, with one additional constraint that determines
the correct fix.

1. Phase P.2 commit `fe2614d` provided both `go-native-target-matrix` and
   `stage-go-native-module` in `scripts/release_artifacts.py`. The former
   selected only native Go targets; the latter copied a self-contained Go
   module plus one target-specific static archive.
2. Phase Q's sc-publish cutover (`71b9d7f`) replaced that script with the
   vendored publish-kit implementation. The current
   `.github/scripts/release_artifacts.py` has neither subcommand. Both fail
   locally with argparse exit status 2.
3. `release-target-matrix` is not a replacement for
   `go-native-target-matrix`: it produces five general release targets, while
   `bindings/sc-sha-go/native/targets.toml` declares exactly three supported
   native module targets: Linux x86_64, macOS arm64, and Windows GNU x86_64.
   Scheduling the generic five-target matrix would run unsupported macOS Intel
   and Windows MSVC legs.
4. `sc-publish` `develop` contains no Go-native staging command and no
   `go_native` manifest schema. Extending the vendored script locally would
   create a permanent sync conflict with its canonical package.

## Scope and Non-goals

This is the implementation plan for the release-verification gap. It is two
ordered changes: first publish the reusable peer package; then consume that
approved package from `sc-compose`.

In scope:

- Provide a target-aware matrix and deterministic staging utility as an
  optional sc-publish peer package.
- Consume that utility for `sc-sha-go` without changing sc-publish core.
- Make every supported native target execute the existing bundled module,
  independent consumer, archive round-trip, and release-layout checks.
- Add hermetic Python coverage for the matrix and staging contracts.

Out of scope:

- Altering sc-publish's generic manifest schema or vendored scripts.
- Publishing a Go module, changing release-channel workflows, or changing the
  generated UniFFI Go bindings.
- Adding native support for macOS Intel or Windows MSVC. Those need explicit
  entries in `bindings/sc-sha-go/native/targets.toml` first.
- Merging PR #582 unchanged. Its `release-target-matrix` substitution must be
  superseded by the target-aware implementation below.

## Package and Helper Contract

Create `sc-publish/plugins/go-native-module/`, modelled on the existing
`plugins/uniffi-bindgen-go` peer package. Its installer accepts one explicit
JSON object and copies the helper and its hermetic test module to a consumer as
`.github/scripts/go_native_module.py` and
`.github/scripts/tests/test_go_native_module.py`; it renders the
consumer-owned config at `release/go-native-module.toml`.

The installer input is intentionally small and independent of sc-publish's
core release-artifact manifest:

```json
{
  "schema_version": 1,
  "source": "bindings/sc-sha-go",
  "cargo_package": "sc-sha-go",
  "artifact_prefix": "sc-sha-go"
}
```

The helper uses only Python standard-library modules (`argparse`, `json`,
`pathlib`, `shutil`, and `tomllib`). It reads the installed config, the
binding-owned target contract at `<source>/native/targets.toml`, and the shared
release-target inventory from `release/publish-artifacts.toml`. It has two
subcommands.

### `target-matrix`

```text
python3 .github/scripts/go_native_module.py target-matrix \
  --manifest release/publish-artifacts.toml \
  --config release/go-native-module.toml
```

Inputs:

- `--manifest`: the generated shared release manifest; only
  `[[release_targets]]` is read.
- `--config`: installed Go-native config defining `source`, `cargo_package`,
  and `artifact_prefix`; the binding root contains `native/targets.toml`.

Output is compact JSON suitable for `GITHUB_OUTPUT`:

```json
{
  "include": [
    {
      "target": "x86_64-unknown-linux-gnu",
      "os": "ubuntu-latest",
      "archive": "tar.gz",
      "goos": "linux",
      "goarch": "amd64",
      "library": "libsc_sha_go.a",
      "cargo_package": "sc-sha-go",
      "module": "github.com/randlee/sc-compose/bindings/sc-sha-go",
      "artifact_prefix": "sc-sha-go"
    }
  ]
}
```

The command fails closed when the contract is malformed, a native Rust target
is absent from the release manifest, a target appears twice, or a required
target field is empty. It emits only the intersection defined by the native
target contract; it must never silently expand to all release targets.

### `stage`

```text
python3 .github/scripts/go_native_module.py stage \
  --config release/go-native-module.toml \
  --target "${{ matrix.target }}" \
  --native-library "$library" \
  --output "$bundle" \
  --version "$version"
```

The command verifies that `--target` exists in `native/targets.toml`, that the
archive exists and has the contract's expected filename, and that the output
does not already exist. It then writes exactly:

```text
<output>/go.mod
<output>/README.md
<output>/go/...
<output>/testdata/...
<output>/native/targets.toml
<output>/native/<rust-target>/<static-library>
<output>/VERSION
```

It must reject missing source files, missing or wrong-name static archives,
unknown targets, malformed native contracts, and pre-existing output paths.
The source tree is never modified.

## Implementation Steps

### 1. Upstream: add the optional peer package to sc-publish

Add the following package-owned files:

| File | Change |
| --- | --- |
| `plugins/go-native-module/README.md` | Consumer contract, JSON input schema, install command, and CI invocation examples. |
| `plugins/go-native-module/install.py` | Validate JSON input, render `release/go-native-module.toml`, copy the helper and its test module, and support `--dry-run`. |
| `plugins/go-native-module/go_native_module.py` | Parsed target/config model plus the `target-matrix` and `stage` subcommands. |
| `plugins/go-native-module/tests/test_install.py` | Installer validation, dry-run, output layout, and overwrite protection tests. |
| `plugins/go-native-module/tests/test_go_native_module.py` | Hermetic helper success and failure-mode tests. |

Keep the helper's shared release-manifest parser local: it needs only
`[[release_targets]]`, so importing private functions from the vendored
`release_artifacts.py` would make this extension fragile.

The upstream package PR must pass its own tests and a consumer-install fixture
before the downstream integration begins.

### 2. Downstream: install and use the approved package in sc-compose

Install the package with the `sc-sha-go` JSON above. Its generated artifacts
are `.github/scripts/go_native_module.py` and `release/go-native-module.toml`.
Then update `sc-compose` as follows:

1. Accept the package-installed
   `.github/scripts/tests/test_go_native_module.py`; it is the same hermetic
   behavior suite tested upstream and is executed by the existing consumer
   script-test job.

   - Matrix success test: fixture manifest plus fixture native contract emits
     the exact three supported entries in native-contract order.
   - Matrix failures: duplicate target, unknown release target, and malformed
     target field fail with nonzero status and actionable stderr.
   - Staging success test: synthetic source tree and static archive produce
     exactly the documented tree and preserve the version text.
   - Staging failures: absent archive, wrong archive filename, unknown target,
     malformed contract, and existing output directory all fail without a
     partial output tree.
2. Update `.github/workflows/ci.yml` at current lines 315-329 so
   `sc-sha-go-plan` calls `go_native_module.py target-matrix`, passing
   `release/publish-artifacts.toml` and `release/go-native-module.toml`.
3. Keep the job label at current line 363 as
   `${{ matrix.goos }}/${{ matrix.goarch }}`. It becomes correct again because
   the replacement matrix deliberately emits those fields. Do not replace it
   with the generic matrix's `os/archive` fields.
4. Update current line 390 to build `-p ${{ matrix.cargo_package }}`, line 398
   to use `matrix.library`, lines 399 and 432 to use
   `matrix.artifact_prefix`, and consumer-module lines 422 and 452 to use
   `matrix.module`. This preserves one generic workflow shape without
   hard-coded `sc-sha-go` identifiers.
5. Update current lines 392-405 so the staging step calls
   `go_native_module.py stage` with the installed config and built archive.
   The downstream bundled-test, independent-consumer, archive round-trip, and
   release-layout steps stay unchanged.
6. Rebase or replace PR #582 with this implementation. Do not merge its
   generic-matrix-only change; it would hide the target contract and schedule
   unsupported jobs.

## Files Expected to Change

| File | Change |
| --- | --- |
| `sc-publish/plugins/go-native-module/**` | New optional package, installer, helper, README, and complete package tests. |
| `.github/scripts/go_native_module.py` | Installed copy of the approved peer-package helper. |
| `.github/scripts/tests/test_go_native_module.py` | Installed hermetic helper behavior tests. |
| `release/go-native-module.toml` | Installed, repository-specific Go-native config. |
| `.github/workflows/ci.yml` | Lines 315-329 and 389-452 call the helper and consume matrix metadata; line 363 retains the Go OS/arch label. |

These files must **not** change in the follow-on implementation:

- `.github/scripts/release_artifacts.py`
- `.github/scripts/release_manifest.py`
- `release/publish-artifacts.toml`
- `.github/workflows/release.yml`

## Verification Plan

Before opening the implementation PR:

```bash
python3 -m pytest .github/scripts/tests/test_go_native_module.py -q
python3 -m pytest .github/scripts/tests -q
git diff --check
```

The existing `manifest-validation` job already runs the complete
`.github/scripts/tests` suite. The package installs the same hermetic helper
tests into the consumer, so both upstream package CI and downstream consumer
CI prove the behavior without a new workflow job.

The implementation PR must also demonstrate all of the following in GitHub
Actions:

1. `sc-sha-go-plan` emits three, not zero or five, matrix entries.
2. The three supported bundle jobs run and pass: Linux x86_64, macOS arm64,
   and Windows GNU x86_64.
3. Each job builds the static archive, stages a bundle, runs `go test ./...`,
   runs the independent consumer module, verifies the zip round trip, and
   passes `bindings/sc-sha-go/tests/test_release_layout.py`.
4. No matrix job is scheduled for macOS Intel or Windows MSVC until those are
   explicitly added to `native/targets.toml` with a compatible library.

## Follow-on Acceptance Criteria

- [ ] No CI command calls a missing release-artifact subcommand.
- [ ] The Go matrix is derived from the binding's native contract and shares
  only runner/archival fields with the general release manifest.
- [ ] The current three supported native module targets complete all bundle
  verification steps in CI.
- [ ] Python unit tests cover both success and fail-closed behavior without
  requiring Rust, Go, a network connection, or a CI runner.
- [ ] sc-compose installs the approved helper, config, and hermetic tests from
  `plugins/go-native-module` without local modifications.
- [ ] `release_artifacts.py` remains byte-identical to the vendored
  sc-publish copy after the implementation.
