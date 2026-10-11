# `sc-sha-go`

This directory contains the generated Go adapter for the two `sc-sha` domain
operations: `CalculateHash` and `CalculateCompositionHash`.

The public Go source under [`go/`](go/) is generated from
[`src/sc_sha_go.udl`](src/sc_sha_go.udl) with UniFFI `0.31.0` and
`uniffi-bindgen-go v0.7.1+v0.31.0`. Do not edit generated Go files by hand.

## Local development

Install the pinned generator once:

```sh
cargo install uniffi-bindgen-go \
  --git https://github.com/NordSecurity/uniffi-bindgen-go \
  --tag v0.7.1+v0.31.0 --locked
```

Then regenerate and test with normal CGo safety checks:

```sh
just generate-sc-sha-go
just prepare-sc-sha-go-native
cd bindings/sc-sha-go
CGO_ENABLED=1 go test ./go/sc_sha_go
```

No command, workflow, or documentation disables Go pointer checks.

## Released module and native targets

The released module path is
`github.com/randlee/sc-compose/bindings/sc-sha-go`. Each GitHub Release
attaches one bundle per supported target (`sc-sha-go-<rust-target>.zip` for
Linux/amd64, macOS/arm64, and Windows/amd64) containing the generated Go
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
