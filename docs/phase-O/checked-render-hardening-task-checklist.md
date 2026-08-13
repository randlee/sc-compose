# Checked-render hardening task checklist

This checklist tracks the hardening assignment for the direct-library and
catalog-backed checked-render callers.

- [x] Use the caller-supplied `OutputFormat` for validation and recorded
      metadata, even when the template path extension disagrees.
- [x] Make `RenderCheckMeta` fields and `CheckedOutput` metadata read-only;
      expose accessors and move metadata into the checked value.
- [x] Cover direct library compose -> check -> emit for valid JSON.
- [x] Verify malformed JSON fails before an output emitter can receive a
      checked value, including the CLI stdout/file no-emission paths.
- [x] Preserve text output byte-for-byte, including Unicode and CRLF bytes.
- [x] Check complete final-body assembly, including guidance and user-prompt
      blocks, before emission.
- [x] Preserve failing-pass diagnostics and reject partial multi-pass output.
- [x] Keep checked/render-invalid responses free of rendered bodies and secret
      variable values.
- [x] Add a catalog-admission fixture and metadata-preservation test for
      atm-core render-on-read integration.

## Closure review

The implementation was re-read against every item after the first test pass.
The only gap found was that the API documentation still described mutable
metadata and omitted the metadata-preserving checker; the phase-O plan, ADR,
and ATM adapter notes now document the accessors and
`check_rendered_output_with_meta` contract. Focused composer and CLI tests
pass; the final `just test` run is the release gate for closure.
