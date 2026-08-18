# Consuming `sc-sha-go`

The released Go module path is:

```text
github.com/randlee/sc-compose/bindings/sc-sha-go
```

The generated package import path is:

```text
github.com/randlee/sc-compose/bindings/sc-sha-go/go/sc_sha_go
```

Release tags use the Go submodule convention:

```text
bindings/sc-sha-go/v<version>
```

The module bundle contains generated UniFFI Go code, the matching C header,
conformance fixtures, and one static Rust library selected for the target:

| OS | Go architecture | Rust target | Library |
| --- | --- | --- | --- |
| Linux | amd64 | `x86_64-unknown-linux-gnu` | `libsc_sha_go.a` |
| macOS | amd64 | `x86_64-apple-darwin` | `libsc_sha_go.a` |
| macOS | arm64 | `aarch64-apple-darwin` | `libsc_sha_go.a` |
| Windows | amd64 | `x86_64-pc-windows-msvc` | `sc_sha_go.a` |

Consumers use the released bundle directly. They do not need a Cargo
checkout, `go generate`, `LD_LIBRARY_PATH`, or a manually installed Rust
library. Unsupported targets and missing or mismatched libraries fail during
build with a diagnostic naming the target and the supported remediation.

The public API is generated from the pinned UniFFI definition. The two typed
operations are:

```go
result, err := scsha.CalculateHash([]byte("hello\r\n"))
if err != nil { /* inspect the typed error */ }

composition, err := scsha.CalculateCompositionHash(entries)
if err != nil { /* inspect the typed error */ }
```

The committed `testdata/conformance-v1.json` vectors are the compatibility
oracle for both calls. Do not duplicate hash logic in Go or edit generated Go
files by hand. Regenerate with the pinned toolchain and run the drift check
when the UniFFI definition changes.

## Handoff status

sc-dolt and atm-core are consumers, not implementation dependencies of this
crate. They must independently verify the documented module tag, target
matrix, conformance vectors, and typed error mapping before adopting it.
