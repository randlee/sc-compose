---
id: ARM64.1
title: ARM64 Linux CLI and Python Wheel Support Plan
status: complete
branch: feature/arm64-linux-support
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feature/arm64-linux-support
target: develop
---

# ARM64.1 — ARM64 Linux Support Plan

## Goal

Add first-class `aarch64` Linux release coverage for the `sc-compose` CLI
archive and the `bindings/python` PyPI wheel in a later implementation sprint.
This document is planning-only. It records the repository state inspected at
`f7747f7` and defines one implementation path; it makes no production, CI, or
release change itself.

## Current-State Evidence and Gap

The release manifest is the source of the two release matrices:

- `release/publish-artifacts.toml:12-31` declares exactly five CLI targets:
  `x86_64-unknown-linux-gnu`, both macOS architectures, and two Windows x86_64
  targets. There is no `aarch64-unknown-linux-gnu` or musl ARM64 target.
- `release/publish-artifacts.toml:85-107` gives each of the three Python
  distributions only `ubuntu-latest`, `macos-latest`, and `windows-latest`
  wheel runners. `bindings/python` therefore has no ARM64 Linux wheel leg.
- `.github/scripts/release_artifacts.py:436-446` expands each literal `wheels`
  value into a release matrix entry, and `:485-488` emits the CLI release
  target matrix directly from the manifest. The current commands emit neither
  ARM64 Linux entry.
- `.github/workflows/release.yml:136-170` builds the manifest-derived CLI
  matrix and `:337-367` builds the manifest-derived Python-wheel matrix. There
  is no handwritten target list in that workflow to keep in sync.
- `bindings/python/pyproject.toml:25-32` configures module packaging only; it
  has no target, Linux compatibility, or manylinux setting. The release job
  currently invokes `maturin build` with no target-specific arguments at
  `.github/workflows/release.yml:356-358`.

Thus `aarch64-unknown-linux-gnu` CLI archives and
`manylinux2014_aarch64`/`manylinux_2_17_aarch64` Python wheels are genuinely
absent today. An ordinary host-built `linux_aarch64` wheel would not satisfy
the PyPI portability goal.

## Recommended Implementation Path

Use GitHub's native ARM64 Linux runner, **`ubuntu-24.04-arm`**, for both the
CLI archive and the ARM64 Python wheel. The correct public runner label is
`ubuntu-24.04-arm`, not `ubuntu-24.04-arm64`: GitHub lists the former as its
ARM64 Ubuntu 24.04 runner label. It executes the same architecture users will
run, so the CLI archive and smoke tests do not depend on QEMU emulation or a
cross C toolchain.

For the Python wheel, retain that native runner but use maturin's Zig path to
produce the explicit `manylinux2014` (glibc 2.17) compatible wheel. Maturin's
own guidance requires a manylinux container or Zig for broadly portable Linux
wheels; Zig avoids relying on Docker availability on the ARM runner. The
future command is:

```bash
maturin build --release --manifest-path bindings/python/Cargo.toml \
  --out dist --compatibility manylinux2014 --zig
```

On the native runner, maturin derives `aarch64-unknown-linux-gnu` from the
host; no cross target flag is needed. The generated filename must carry an
`aarch64` manylinux-compatible platform tag. Keep the existing `maturin`
version pin and install its Zig extra in the release build action.

This is a single native-execution strategy. Cross compilation through `cross`,
an x86_64 manylinux cross container, or QEMU is not the chosen path: each adds
a second architecture/toolchain boundary while providing no benefit for the
CLI, and QEMU makes the tests slower and less representative.

## Exact Future Changes

### Release artifact and Python distribution manifest

In `release/publish-artifacts.toml`:

1. Add one `[[release_targets]]` entry after the existing Linux GNU entry:

   ```toml
   [[release_targets]]
   target = "aarch64-unknown-linux-gnu"
   os = "ubuntu-24.04-arm"
   archive = "tar.gz"
   ```

   The existing `release-target-matrix` command will then add that target to
   the `build` job automatically, and its normal archive step will produce
   `sc-compose_<version>_aarch64-unknown-linux-gnu.tar.gz`.

2. Append `"ubuntu-24.04-arm"` only to the `wheels` array for the
   `[[python_distributions]]` entry named `sc-compose` at current lines
   `91-98`. Do not add it to `sc-sha` or `sc-composer-beads` in this sprint;
   this feature promises the `bindings/python` wheel only.

No change is needed in `.github/scripts/release_artifacts.py` or
`.github/scripts/release_manifest.py`: their generic string-list validation
and matrix expansion already preserve an additional runner label. Add/update a
focused assertion in `.github/scripts/tests/test_release_artifacts.py` that
the real manifest's Python matrix includes the new `sc-compose` ARM64 entry
and that the release matrix includes the new CLI target.

### Release workflow and maturin invocation

In `.github/workflows/release.yml`:

1. Leave `release-plan` and `build` matrix expressions intact. Their current
   data-driven entries at lines `324-326` and `136-141` will consume the new
   manifest target.
2. In `build-python-wheels`, make the existing generic **Build wheels
   (maturin)** step apply to every non-ARM64 leg, and add a named **Build ARM64
   Linux wheel (maturin + Zig)** step conditioned on
   `matrix.os == 'ubuntu-24.04-arm'`. That step runs the command in the
   recommendation above. The existing `python-wheel-${{ matrix.artifact }}-${{
   matrix.os }}` upload name at lines `364-367` will remain unique and needs no
   special collector logic.
3. In `.github/actions/setup-python-release-build/action.yml`, change the
   maturin installation at current lines `45-48` from `maturin==1.9.4` to the
   same pinned version with its Zig extra, so the ARM64 release step can use
   `--zig`. Do not put an ARM target in `bindings/python/pyproject.toml`: the
   target is correctly a release-matrix property, and that project file's
   existing packaging configuration applies on every supported host.
4. Keep the existing release asset collection and checksum paths unchanged.
   They already collect every `*.tar.gz` and `*.whl`
   (`release.yml:515-534`), while the manifest-derived expected patterns will
   gain the CLI archive automatically.

### Pull-request CI

In `.github/workflows/ci.yml`, do **not** add `ubuntu-24.04-arm` to the
existing `test` matrix at lines `106-112`. That job invokes
`setup-lint-toolchain`, whose `setup-sc-lint` action explicitly accepts only
`Linux:X64` at `.github/actions/setup-sc-lint/action.yml:28-35`; adding the
runner there would create a deterministic, unrelated failure.

Instead add a dedicated job named **`linux-arm64-release-validation`** after
`test`, with `runs-on: ubuntu-24.04-arm` and `needs: [clippy,
manifest-validation]`. It must:

1. install Rust `1.94.1` and the target-native Beads binary via
   `setup-beads` (which already maps `Linux-ARM64` at
   `.github/actions/setup-beads/action.yml:26-34`);
2. run `cargo test --workspace` with `BD_EXECUTABLE` as the normal test job
   does;
3. run `cargo build --release --target aarch64-unknown-linux-gnu -p
   sc-compose` and execute the resulting `--help` binary; and
4. build, install, and smoke-test `bindings/python` using the same native
   maturin command as the release job, then assert that the wheel filename is
   manylinux-compatible and contains `aarch64`.

Do not run `just lint-ci-consumer` in this new job until sc-lint publishes its
ARM64 Linux archive and `setup-sc-lint` gains a `Linux:ARM64` target mapping.
That is a separate tool-release dependency, not a reason to represent the
ARM64 runner as tested when setup is known to reject it.

## Distribution Channel Impact

The GitHub Release and PyPI assets are in scope. Homebrew, Winget, and Scoop
manifest support is not part of the first implementation sprint:

- Homebrew currently declares one Linux asset (`linux` ->
  `x86_64-unknown-linux-gnu`) in `release/publish-artifacts.toml:130-138`, and
  its formula template has one unconditional `on_linux` URL at
  `release/homebrew/formula.rb.j2:19-22`. Publishing the new archive does not
  break that formula, but selecting a Linux ARM asset would need a distinct,
  reviewed Homebrew follow-up that changes both the asset model and template
  to distinguish `on_arm` from `on_intel`.
- Winget and Scoop each select only
  `x86_64-pc-windows-msvc` (`release/publish-artifacts.toml:140-153`), and the
  Scoop template exposes only a `64bit` Windows architecture
  (`release/scoop/manifest.json.j2:6-13`). Linux ARM64 does not require either
  manifest to change.

## Verification and Release Gate

The implementation is complete only when all of the following are green:

1. Local/configuration checks: `python3 .github/scripts/release_artifacts.py
   validate-manifest --manifest release/publish-artifacts.toml --workspace-toml
   Cargo.toml`, its script test suite, `cargo fmt --all --check`, and `git
   diff --check`.
2. PR CI: existing x86_64 Linux, macOS, and Windows jobs remain green; the new
   `linux-arm64-release-validation` job passes native workspace tests, the
   `aarch64-unknown-linux-gnu` CLI smoke test, and an installed ARM64 wheel
   smoke test.
3. Release rehearsal (`target=testpypi`): `build` has successful
   `aarch64-unknown-linux-gnu` archive output; `build-python-wheels` has a
   successful `sc-compose` `ubuntu-24.04-arm` wheel; release asset validation
   accepts the additional CLI archive and wheel; and TestPyPI accepts the wheel.
4. Artifact inspection: verify the archive contains an executable
   `bin/sc-compose`; verify the wheel is named for `aarch64` and a manylinux
   compatible platform rather than plain `linux_aarch64`; install it on an
   independent ARM64 Linux environment and run
   `bindings/python/tests/test_smoke.py`.

## Explicit Non-Goals

- No implementation is made by this planning sprint.
- No `aarch64-unknown-linux-musl` CLI archive or `musllinux` wheel is added.
- No ARM64 changes are made for `sc-sha`, `sc-composer-beads`, macOS, Windows,
  Winget, Scoop, or Homebrew.
- No change is made to sc-lint; its lack of a Linux ARM64 release remains an
  explicit reason to keep the dedicated CI job separate from the lint matrix.
