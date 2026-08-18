# P.2 consumer handoff record

Date: 2026-08-18  
Module: `github.com/randlee/sc-compose/bindings/sc-sha-go`
Package import: `github.com/randlee/sc-compose/bindings/sc-sha-go/go/sc_sha_go`
Release tag convention: `bindings/sc-sha-go/v<version>`

## Contract sent to consumers

- API: generated `CalculateHash` and `CalculateCompositionHash`.
- Compatibility oracle: `bindings/sc-sha-go/testdata/conformance-v1.json`.
- Native targets: Linux/amd64, macOS/arm64, Windows/amd64.
- Linkage: target-specific packaged static library; no Cargo checkout or
  `LD_LIBRARY_PATH` fallback.
- Errors: preserve the generated typed error mapping and stable error codes.
- Updates: consume a released module tag, run the committed vectors, and
  independently verify the target-native bundle before adoption.

## Requests

### sc-dolt

Please verify module resolution, the documented target matrix, both typed API
calls, conformance vectors, and error mapping in an independent consumer
module. Record the adopted module tag and verification result in the sc-dolt
integration evidence.

Status: handoff requested; external adoption is not claimed by this sprint.

### atm-core

Please perform the same independent verification and confirm that the module
can be consumed without an atm-core or Cargo checkout dependency. Record the
adopted module tag, target, vectors, and any compatibility findings in the
atm-core integration evidence.

Status: handoff requested; external adoption is not claimed by this sprint.
