# Changelog

All notable changes to this project will be documented in this file.

## [1.2.0] - 2026-07-17

### Added

- Phase C (C.1 through C.3): Python bindings for sc-compose composition APIs,
  published as the `sc-compose` package on PyPI/TestPyPI.
  - Sprint C.1: maturin-based `bindings/python` crate foundation, PyO3
    extension module scaffolding, and Python packaging shape.
  - Sprint C.2: the Python API surface — `Renderer`, `ComposeRequest`,
    `ComposeResult`, `Frontmatter`/`ParsedTemplate`, resolver and validation
    types, and the `compose`/`render_template`/`resolve_profile`/`validate`
    module-level functions.
  - Sprint C.3: the release train and packaging hardening — `workflow_dispatch`
    release workflow with staged TestPyPI rehearsal and production PyPI
    publish targets, release-gate enforcement, and GitHub Release attachment
    handling.

### Fixed

- Issue #117: `__repr__` on PyO3 pyclass types (e.g. `ComposeRequest`) now
  routes correctly and produces an informative repr instead of the default
  opaque object repr.

### Changed

- Workspace version bump: `1.1.0` -> `1.2.0`.

## [1.1.0] - 2026-05-26 (updated 2026-07-15)

### Added

- Phase HTML-Report (FR-12 through FR-15): map/object variable inputs, arrays
  of objects, HTML template output, and the bundled `sprint-report-html`
  example.
- Sprint S7: `sc-compose examples` and `sc-compose templates` commands,
  including bundled examples and template-pack workflows.
- Sprint S8: release engineering and distribution support, including release
  gate automation, Homebrew tap automation, winget manifests, and idempotent
  publish handling for already-published crate versions.
- Phase A (A1 through A9): the SC-Reporting contract foundation, covering the
  report artifact catalog, producer recipe surface, source-collection and
  render-many contract, semantic diagram spec, template families, shared panel
  chrome, latest/archive policy, and publish-manifest handoff.
- Phase B (B1 through B15, integrated via PR #87): the public reporting CLI
  surface with `reports init`, `reports smoke`, `reports finalize`,
  `reports render-spec`, `reports index`, `reports verify`, and
  `reports publish-manifest`, plus the shared report scaffold, semantic-spec
  rendering path, latest/archive materialization, and publish-manifest CI
  handoff.

### Fixed

- `observer_impl.rs`: removed the `.expect()` panic risk in `health()`;
  `shutdown()` now surfaces flush errors cleanly.
- Added text-mode `observability-health` test coverage alongside the JSON
  checks.
- Consolidated the shared reporting boundary rule in `docs/requirements.md`.
- Documentation follow-ups: corrected the `requirements.md` HTML-report section
  header and added the missing H4 row to the FR coverage matrix.
- Homebrew release automation and formula rendering fixes that removed the
  manual tap-push requirement after `1.0.1`.
- PR #95: removed the stale `.atm.toml` pane-hook wiring so repo-local ATM
  configuration no longer depends on tmux pane metadata or post-send hooks.

### Changed

- Workspace version bump: `1.0.2` -> `1.1.0`.
- Sprint B9 / PR #85: `sc-compose` now adopts `sc-observability` `1.2.0`
  directly for CLI logging, including retained-log maintenance defaults,
  `Logger::log(...)` queue-admission semantics, and the shutdown-to-stopped
  typestate path used by `observability-health`.

## [1.0.1] - previous release

See git history prior to `v1.1.0` for earlier changes.
