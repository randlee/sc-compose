# Release Notes — sc-compose v1.0.0

## Summary

- First standalone `1.0.0` release of `sc-composer` and `sc-compose` from the
  `sc-compose` repo, replacing the previously ATM-workspace-published versions
  of these crates
- Full FR-1 through FR-11 implementation: resolver, include expansion,
  validation pipeline, renderer, observer hooks, CLI commands, JSON output, and
  structured logging integration
- Failure-mode matrix (`ERR_*` codes) fully implemented and snapshot-tested across all
  command families
- Observer and logging integration test suite covering command lifecycle, pipeline
  events, `observability-health`, shutdown, and sink degradation behavior
- End-to-end smoke test validating frontmatter, includes, explicit/env/file vars,
  profile resolution, and `observability-health`

## Included Crates

- `sc-composer` v1.0.0 — reusable template-composition library
- `sc-compose` v1.0.0 — CLI wrapper and observer binding

## Compatibility Notes

- **Breaking change** relative to the last ATM-workspace-published versions of these crates.
  See `docs/migration-notes.md` for the full cutover guide and API change summary.
- ATM adapters must update to the new observer trait surface. See `docs/atm-adapter-notes.md`
  for the integration guide.
- CLI users now have file-backed structured logging plus the `observability-health`
  command for local logger health inspection.
- This template is finalized when the Sprint 4 release gate clears. See
  `docs/release-checklist.md` for the pre-release and post-publish checklist.
