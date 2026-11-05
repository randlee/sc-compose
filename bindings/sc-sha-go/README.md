# `sc-sha-go`

This directory contains the generated Go adapter for the two `sc-sha` domain
operations: `CalculateHash` and `CalculateCompositionHash`.

The public Go source under [`go/`](go/) is generated from
[`src/sc_sha_go.udl`](src/sc_sha_go.udl) with `uniffi-bindgen-go v0.7.1+v0.31.0`; the native
scaffolding uses UniFFI `0.31.2`. Do not edit generated Go files by hand.

## Local development

Install the pinned generator once:

```sh
cargo install uniffi-bindgen-go \
  --git https://github.com/NordSecurity/uniffi-bindgen-go \
  --tag v0.7.1+v0.31.0 --locked
```

The Rust scaffolding is generated independently with the matching UniFFI CLI,
which is installed outside the workspace dependency graph:

```sh
cargo install uniffi --version '=0.31.2' --features cli --bin uniffi-bindgen --locked
just generate-sc-sha-rust check
```

Use `just generate-sc-sha-rust` to regenerate it after changing the UDL. CI
checks for drift. Do not edit the generated Rust file by hand.

Then regenerate the Go bindings and test with normal CGo safety checks:

```sh
just generate-sc-sha-go
just prepare-sc-sha-go-native
cd bindings/sc-sha-go
CGO_ENABLED=1 go test ./...
```

No command, workflow, or documentation disables Go pointer checks.

## Released module and native targets

The released module path is
`github.com/randlee/sc-compose/bindings/sc-sha-go`. Each GitHub Release
attaches one bundle per supported target (`sc-sha-go-<rust-target>.zip` for
Linux/amd64 and arm64, macOS/amd64 and arm64, and Windows/amd64 and arm64) containing the generated Go
source, header, conformance fixtures, and the matching static library under
`native/<rust-target>/`. Unsupported or mismatched targets fail
deterministically. Static libraries are not committed to the repository, so
`go get` alone is not a complete installation: download the bundle and point a
`replace` directive at it, as shown in
[`docs/integrations/sc-sha-go.md`](../../docs/integrations/sc-sha-go.md). A
released consumer does not need a Cargo checkout, `go generate`,
`LD_LIBRARY_PATH`, or a system-wide Rust installation.

Use `just generate-sc-sha-go check` to verify that the pinned generator does
not change committed output.

### Native ABI and local preparation

| Go coordinate | Rust target | CGo compiler |
| --- | --- | --- |
| linux/amd64 | x86_64-unknown-linux-gnu | GCC or Clang with glibc |
| linux/arm64 | aarch64-unknown-linux-gnu | GCC or Clang with glibc |
| darwin/amd64 | x86_64-apple-darwin | Apple Clang |
| darwin/arm64 | aarch64-apple-darwin | Apple Clang |
| windows/amd64 | x86_64-pc-windows-gnu | MinGW-w64 GCC |
| windows/arm64 | aarch64-pc-windows-gnullvm | LLVM MinGW Clang |

Each target uses `libsc_sha_go.a`. Windows archives use the GNU ABI required
by the corresponding CGo compiler, not an MSVC `.lib` renamed to `.a`.
For ARM64, use LLVM MinGW's `aarch64-w64-mingw32-clang` as `CC` and
`CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER`; install the Rust target with
`rustup target add aarch64-pc-windows-gnullvm`. Rust documents this ABI's
[compatible LLVM C toolchain](https://doc.rust-lang.org/rustc/platform-support/windows-gnullvm.html).
The x64 GNU target and compiler remain supported.

`just prepare-sc-sha-go-native` selects the archive ABI from the native Go
host, builds that explicit Rust target, reads Cargo's actual target directory
(including `CARGO_TARGET_DIR`), and calls the canonical peer's `stage` command.
It copies the validated archive into the source loader's ignored native
directory. Install the selected Rust target and C toolchain first. Cross-Go
execution settings are rejected; foreign targets use the manual integration
matrix instead. Released modules need only Go and the matching CGo compiler.

The peer installer input remains `release/go-native-module-install.json`
(package `0.1.0`). It was reinstalled and checked byte-for-byte using upstream
`sc-publish` revision `209681d096bd8bbfbddef51339b9323e0be5681a`:

```sh
python3 <pinned-sc-publish>/plugins/go-native-module/install.py \
  --input release/go-native-module-install.json .
python3 <pinned-sc-publish>/plugins/go-native-module/install.py \
  --input release/go-native-module-install.json --dry-run .
```

`python3 bindings/sc-sha-go/tests/test_release_layout.py` exercises six-target
ZIP layout, actual Go linker-directive selection, C preprocessor acceptance
and rejection, and preparation argv/errors with controlled inputs. Its
`tests/fixtures/release-targets.toml` is a boundary fixture; full external
consumer builds/installs and the combined release manifest are validated by
the Go native integration matrix in `.github/workflows/go-native-preflight.yml`,
triggered through `workflow_dispatch`. Routine CI runs the adapter conformance
tests and these controlled layout fixtures without building or installing the
six-target matrix. Fixture archives are deliberately not executable native
libraries and do not establish foreign-platform runtime success.
