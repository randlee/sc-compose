# ATM Adapter Notes

These notes define the intended seam between this repository and ATM-specific
integration code.

## Integration Boundary

- The primary ATM adapter seam is the public observer API exposed by
  `sc-composer` and consumed directly or indirectly by `sc-compose`.
- Typestate and other internal pipeline markers are library implementation
  details, not adapter extension points.
- Command lifecycle events are emitted by the CLI layer. The library emits
  composition-stage events only.

## Configuration Ownership

- ATM adapters own mailbox-aware configuration, path translation, and any host
  policy projection needed to build a `ComposeRequest`.
- `sc-composer` intentionally does not expose a `ComposerConfig` object for ATM
  to fill. Adapters construct request values directly.
- Runtime-specific home resolution, spool paths, and ATM transport concerns stay
  outside this repository.

## Breaking-Change Context

- `sc-composer` and `sc-compose` intentionally replace equivalent crates that
  previously lived in `agent-team-mail`.
- This is an intentional breaking change and migration step, not a temporary
  compatibility layer.
- The crate version is `1.0.0` for the first standalone release from this
  repo.
- The standalone release checklist lives in `docs/release-checklist.md`; ATM
  cutover follows that published release.

## Checked-render handoff

An ATM adapter that will send or cache a rendered template must supply the
exact context it intends to use and run `validate --check-render --json` (or
call the library's `check_rendered_output` after rendering). It must inspect
the structured `payload.state`, not infer success from human-readable output:

- `render_checked` permits sending or caching the exact checked result.
- `static_only`, `context_required`, `contract_invalid`, and `render_invalid`
  deny sending or caching.

Plain `validate` is intentionally static-only and emits no rendered body.
Validation and checked-render responses contain diagnostics and a redacted
context summary; adapters must not require the complete prompt body merely to
decide whether a render is safe. A JSON parser failure uses the stable
`ERR_RENDER_JSON_MALFORMED` diagnostic and includes source location without
echoing variable values. Multi-pass failures identify the final render pass
that produced the rejected body.
