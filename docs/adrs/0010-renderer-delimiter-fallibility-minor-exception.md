# ADR-0010: Narrow Stability Exception for `Renderer::with_delimiters`

## Status

Accepted

## Context

`docs/requirements.md` Section 6 defines the general `sc-composer` stability
policy:

- before `1.0`, breaking API changes require a minor version bump,
- after `1.0`, patch releases contain backward-compatible bug fixes only,
- after `1.0`, minor releases contain backward-compatible new features, and
- after `1.0`, major releases contain breaking changes.

Phase D also closes GAP-4 by routing multi-pass rendering through
`Renderer::with_delimiters(open, close)`. The original public constructor
returned `Self` and could panic on invalid delimiter pairs. The corrected API
returns `Result<Self, RenderError>` so invalid delimiters become a typed,
recoverable error instead of an unrecoverable process abort.

This changes the Rust type signature and is therefore semver-visible, but the
old behavior was not a supported success path:

- valid delimiter callers keep the same successful rendering behavior,
- invalid delimiter callers previously could only crash the process, and
- no caller could correctly rely on that panic as a documented or recoverable
  contract.

## Decision

`sc-compose` ships the `Renderer::with_delimiters(open, close) ->
Result<Self, RenderError>` change in the `1.3.0` line under one narrow
stability-policy exception.

That exception is limited to this single constructor change and this single
release train:

- the general Section 6 policy remains unchanged,
- this is not a general alpha/pre-GA carve-out,
- this does not license skipping semver for other public API breaking changes,
  and
- any future breaking change after `1.0` still requires a major release unless
  it is separately justified and accepted through its own explicit policy
  amendment.

## Consequences

- Rust callers must now handle `RenderError` when constructing a renderer with
  custom delimiters.
- Invalid delimiter pairs now fail gracefully with a typed error instead of
  aborting the process.
- `docs/requirements.md` Section 6 includes a narrowly-scoped cross-reference
  back to this ADR so the unconditional major-version rule remains the default
  interpretation everywhere else.
