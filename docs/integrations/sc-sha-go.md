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
| macOS | arm64 | `aarch64-apple-darwin` | `libsc_sha_go.a` |
| Windows | amd64 | `x86_64-pc-windows-gnu` | `libsc_sha_go.a` |

Consumers use the released bundle directly. They do not need a Cargo
checkout, `go generate`, `LD_LIBRARY_PATH`, or a manually installed Rust
library. Unsupported targets and missing or mismatched libraries fail during
build with a diagnostic naming the target and the supported remediation.

Install a released target bundle from the repository tag in a consumer module:

```sh
go mod init example.invalid/my-consumer
go get github.com/randlee/sc-compose/bindings/sc-sha-go@bindings/sc-sha-go/v1.4.1
```

The generated package exposes concrete manifest types. This complete example
uses the ordered two-node vector from `testdata/conformance-v1.json`:

```go
package main

import (
    "fmt"
    scsha "github.com/randlee/sc-compose/bindings/sc-sha-go/go/sc_sha_go"
)

func main() {
    fileHash, err := scsha.CalculateHash([]byte("hello\r\n"))
    if err != nil { panic(err) }
    fmt.Println(fileHash.Sha256)

    manifest := scsha.ResolvedTemplateManifest{
        Schema: "sc-sha/manifest/v1",
        Nodes: []scsha.ResolvedTemplateNode{
            {Source: scsha.CanonicalSourceLocalPath{Value: "root.md"}, Sha256: "53175bcc0524f37b47062fafdda28e3f8eb91d519ca0a184ca71bbebe72f969a"},
            {Source: scsha.CanonicalSourceLocalPath{Value: "child.md"}, Sha256: "2fa14f53e6b15cac9ac77846c7be87862c2a7e9ec0c6cea319db939317f126ed"},
        },
        Edges: []scsha.ResolvedIncludeEdge{{
            Parent: scsha.CanonicalSourceLocalPath{Value: "root.md"},
            Child: scsha.CanonicalSourceLocalPath{Value: "child.md"},
            Occurrence: 0,
        }},
    }
    compositionHash, err := scsha.CalculateCompositionHash(manifest)
    if err != nil { panic(err) }
    fmt.Println(compositionHash.Sha256)
}
```

The expected composition digest for this fixture is
`80c55ea43eaa4c0453fe189c5aa0bbc1f523b8c66cc23ab990ec0356acd737ac`.

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
