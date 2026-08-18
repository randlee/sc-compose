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
cargo build -p sc-sha-go
cd bindings/sc-sha-go
CGO_ENABLED=1 \
  CGO_LDFLAGS="-L$PWD/../../target/debug -lsc_sha_go" \
  DYLD_LIBRARY_PATH="$PWD/../../target/debug" \
  go test ./go/sc_sha_go
```

No command, workflow, or documentation disables Go pointer checks.

Use `just generate-sc-sha-go check` to verify that the pinned generator does
not change committed output.
