# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Changed

- Migrated the six in-repository JSON assignment templates to explicit
  `json_escape_mode: auto`, with semantic hostile-value fixtures and a
  documented legacy compatibility fixture. See
  `docs/migration/json-escape-mode.md` for the source-shape matrix. This is
  repository-local migration evidence; cross-repository release readiness
  remains a Phase O.5 responsibility.
- Added the Phase O.5 pinned release-corpus inventory and parser-backed fuzz
  gate. The campaign records actual consumer-root counts, rejects malformed
  JSON before emission, preserves the auto/legacy compatibility probe, and
  reports external migration owners without editing their repositories. The
  current 1.4.1 recommendation is conditional pending migration or explicit
  legacy disposition for 28 external templates across six downstream roots
  (atm-core, cpo, raptor, sc-lint, synaptic-canvas, and roslyn-lint); see
  `docs/phase-O/evidence/o5-release-corpus.md`.

## [1.4.0] - 2026-08-12

### Added

- Added the standalone `sc-sha` crate to the release pipeline, together with
  the `sc-sha` Python distribution. `sc-sha` provides the portable,
  LF-normalized content and composition hashing contract consumed by
  `sc-composer` and other projects without bringing in renderer or CLI code.

### Changed

- Bumped the Rust workspace and both PyPI packages from `1.3.1` to `1.4.0`.
  This is a semver-minor release because the public `ComposePolicy`,
  `ComposeResult`, and `ExpandedTemplate` structures gained fields and the
  rendering/validation surface grew; a patch release would not be appropriate
  for literal construction by downstream users.
- Corrected the dependency-aware publish order to `sc-sha` → `sc-composer` →
  `sc-compose`. `sc-composer` has a real path dependency on `sc-sha`, so
  `sc-sha` must be available in the crates.io index before Cargo can resolve
  the published `sc-composer` package. The missing manifest entry previously
  made `cargo publish --dry-run -p sc-composer` fail.
- Added release metadata checks that keep workspace-inherited crate versions,
  explicit Cargo path-dependency pins, and the hard-coded PyPI package
  versions in lockstep, preventing Rust/Python release-version skew.

### Changed

- Upgraded `anyhow` from `1.0.102` to `1.0.103` and `quick-xml` from `0.38.4`
  to `0.41.0` to resolve RUSTSEC-2026-0190, RUSTSEC-2026-0194, and
  RUSTSEC-2026-0195; removed the deprecated `cargo-deny` 0.19.4 keys from
  `deny.toml`.
- XML attribute extraction now follows `quick-xml` 0.41's XML 1.0 AVNormalize
  behavior, collapsing embedded tab, carriage-return, and line-feed characters
  to spaces after entity decoding.

## [1.3.1] - 2026-08-05

### Fixed

- Fix issue #238 where an adjacent rendered-document frontmatter block
  containing Jinja syntax was incorrectly parsed as a second YAML config block,
  causing `ERR_CONFIG_PARSE` during validation and rendering.

## [1.3.0] - 2026-08-04

This release covers all work landed on `develop` since `1.2.0`: Phases D
through J. `1.3.0` was bumped in `Cargo.toml` when Phase D landed but was
never tagged or published, so this release folds every phase merged in the
interim into the single `1.3.0` line rather than burning additional version
numbers on never-published intermediate states — none of Phases E through J
introduce a breaking or consumer-facing incompatible change.

### Added

- Phase D (D.1 through D.4, plus D.1-py through D.4-py): first-class nested
  template support with stacked frontmatter passes, brace-count-aware variable
  discovery, multi-pass composition, `render --all`, pass-scoped CLI variable
  groups, `verify`, multi-pass `template-init`, and tandem Python bindings for
  the library-owned Phase D surface.
- ADR-0010: a narrowly-scoped stability-policy exception for
  `Renderer::with_delimiters`, documenting why the constructor's move from a
  panic path to `Result<Self, RenderError>` ships in the `1.3.0` line without
  a major-version bump.
- Phase E (E.1 through E.3): recursive structured-input support for
  `--var-file` and frontmatter/`template.json` defaults — arrays of objects,
  jagged scalar arrays, and other finite nested JSON/YAML value trees (closes
  issue #157) — plus the reusable `adversarial-fuzzing` skill (coordinator and
  bounded background probe agents) that classifies confirmed rendering-boundary
  bugs and promotes them into permanent regression tests.
- Phase G (G.1 through G.7): the first `sc-compose extract` feature —
  deterministic recovery of scalar string variable bindings from a known
  `.xml.j2` template plus its rendered XML output, exposed through the CLI and
  matching `sc_composer` library/Python API. Extraction uses structural
  occurrence matching and fails closed (an explicit unsupported/ambiguous
  result) instead of silently returning a wrong value for repeated sibling
  tags or unsupported Jinja constructs. Includes a corpus of realistic and
  adversarial extraction fixtures promoted into permanent Rust/Python/CLI
  regression coverage.
- Phase H (H.1 through H.8): `sc-compose extract` and the underlying
  library/Python API now support JSON, YAML, and TOML rendered-output
  extraction alongside XML, using the same fail-closed, string-value report
  model and structural provenance (closes issue #193's JSON/YAML/TOML gaps).
- Phase I (I.1 through I.6): a first-class `raw` extraction mode for Markdown
  and other plain-text documents, reused by XML block/mixed-content extraction
  so a placeholder occupying an XML element's content can recover a full
  text-plus-markup block (closes issue #193 Gap 1), plus narrow, observable
  normalization of non-XML preamble text before the document root during
  rendered-XML extraction (closes issue #193 Gap 5).
- Phase J (J.1 through J.4): internal decomposition of the CLI argument/
  JSON-capability surface, the validation-state assembly and diagnostic-policy
  layers, and the frontmatter parser/normalizer, reducing hot-spot risk
  (issue #212) with zero public-API or behavior change.

### Fixed

- `Renderer::with_delimiters` no longer panics on invalid delimiters; it now
  returns a typed `RenderError`, and the CLI/Python surfaces document the same
  fail-closed behavior.
- Multi-pass validation now discovers undeclared tokens per pass and direct
  `render_all()` calls correctly apply frontmatter defaults beneath caller
  values.
- Phase D documentation now reflects the landed delimiter-hardening state,
  verify/template-init Python scope, and the point-in-time nature of the final
  consolidated review artifact snapshot.
- The artificial nested-array validation restriction that previously rejected
  valid recursive structured input is removed, while top-level var-file and
  YAML string-key boundaries are preserved. `ERR_VAL_NESTED_ARRAY_UNSUPPORTED`
  remains a reserved compatibility code but is no longer emitted for supported
  recursive input values.
- Dotted expressions (e.g. `{{ user.name }}`) passed to `extract` are now
  rejected as unsupported object-field access instead of being misread as a
  literal variable name `"user.name"`.
- YAML alias/anchor expansion and JSON/YAML/XML input depth are now bounded
  during extraction parsing, closing a resource-exhaustion path where a
  malicious rendered document with recursive aliases or excessive nesting
  could exhaust the process before extraction could fail closed.
- Jinja loop-context built-ins (`loop.last`, etc.) are no longer misreported as
  undeclared variables inside a `for` scope, while a user variable literally
  named `loop` outside a loop still validates normally (closes issue #167).
- YAML merge keys (`<<: *defaults`) in var-files no longer silently discard
  inherited fields; merge-key handling is now explicit and diagnostic (closes
  issue #166).

### Changed

- Workspace version bump: `1.2.0` -> `1.3.0`.
- Internal restructuring of `crates/sc-compose/src/cli.rs`, `main.rs`,
  `var_file.rs`, and `observer_impl.rs` (Phase F) into smaller, independently
  testable modules; the CLI contract, JSON output shape, and the
  `sc-composer` pure-library boundary are unchanged.

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
