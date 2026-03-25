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
- The crate version remains `0.46.2` during downstream integration.
- A crates.io publish checklist is deferred until downstream ATM integration and
  cutover are complete.
