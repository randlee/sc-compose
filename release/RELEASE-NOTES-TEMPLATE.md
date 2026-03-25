# Release Notes — sc-compose v0.46.2

## Summary

- First standalone release of `sc-composer` and `sc-compose` from the `sc-compose` repo,
  replacing the previously ATM-workspace-published versions of these crates
- Full FR-1 through FR-9 implementation: resolver, include engine, validation pipeline,
  renderer, typestate document model, observer/sink hooks, CLI commands, and JSON output
- Failure-mode matrix (`ERR_*` codes) fully implemented and snapshot-tested across all
  command families
- Observer integration test suite covering all 6 documented emission points
- End-to-end smoke test validating frontmatter, includes, explicit/env/file vars, and
  profile resolution

## Included Crates

- `sc-composer` v0.46.2 — reusable template-composition library
- `sc-compose` v0.46.2 — CLI wrapper and observer binding

## Compatibility Notes

- **Breaking change** relative to the last ATM-workspace-published versions of these crates.
  See `docs/migration-notes.md` for the full cutover guide and API change summary.
- ATM adapters must update to the new observer trait surface. See `docs/atm-adapter-notes.md`
  for the integration guide.
- Publish is deferred until downstream product integration is complete. This template will be
  filled in at the time of the actual release. See `docs/release-checklist.md` for the
  pre-release gate checklist.
