# SC-Compose Test Strategy

This document defines the planned verification model for `sc-composer` and
`sc-compose` implementation work.

## Test Layers

### Unit Tests

Owned primarily by `sc-composer` modules.

Use unit tests for:

- frontmatter parsing and normalization
- strong-type constructors and invariants
- resolver precedence
- include confinement, cycles, and depth
- unknown-variable policy handling (`error`, `warn`, and `ignore` modes, FR-2b)
- include-driven defaults and required-variable propagation (FR-3a)
- variable precedence and validation behavior
- `Renderer`, `compose()`, and `validate()` edge cases
- observer/sink no-op and injected behavior

### Integration Tests

Use integration tests for:

- CLI command behavior
- JSON output and dry-run schemas
- `--dry-run` no-write filesystem guarantee
- var-file and environment-prefix inputs
- output-path derivation
- `frontmatter-init` and `init`

### Golden / Snapshot Tests

Use snapshot-style tests for:

- `render --json`
- `resolve --json`
- `validate --json`
- `frontmatter-init --json`
- `init --json`
- dry-run outputs
- diagnostics and `ERR_*` code emission

### Smoke Tests

Use end-to-end smoke tests for a representative composed template including:

- frontmatter
- includes
- explicit vars
- env vars
- var-files
- profile resolution
- file output

### Cross-Platform Tests

Use path-focused tests to verify:

- canonicalization under macOS, Linux, and Windows assumptions
- path-separator handling
- drive-letter and rooted-path behavior
- confinement behavior under symlink or canonical-path edge cases

## Planned Fixture Sets

Create a reusable fixture tree under a test fixture directory containing:

- plain templates
- `.j2` templates
- typed templates such as `.md.j2`
- shared/runtime profile layouts
- nested include graphs
- invalid confinement/escape cases
- frontmatter/no-frontmatter variants
- golden JSON output examples

## Sprint-to-Test Mapping

| Sprint | Required verification emphasis |
| --- | --- |
| Sprint 1 | blocker audit, contract closure, and schema/document consistency |
| Sprint 2 | logger wiring, command/event mapping, and observability command scaffolding |
| Sprint 3 | production hardening for `observability-health`, `--json` cleanliness, sink degradation, shutdown, and downstream notes |
| Sprint 4 | full smoke tests, release checklist closure, and final gate suite |

## Mandatory Gates

Per sprint, the minimum validation is:

- targeted unit/integration tests for the sprint scope
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`

For the final gate, additionally require:

- `cargo fmt --all --check`
- full end-to-end smoke test
- QA approval on JSON schemas and diagnostics

## Guidance for Dev Agents

- Add tests in the same sprint that introduces behavior; do not defer coverage
  to a later sprint unless the plan explicitly says so.
- When introducing a new `ERR_*` code, add at least one assertion that the code
  appears in diagnostics output.
- Prefer golden tests for externally visible JSON and text output.
- Prefer unit tests for typestate transitions and normalization behavior.
- If a behavior is specified in `docs/requirements.md`, it is not complete until
  there is at least one automated test covering it.

## Review Expectations

- `qm-comp` should be able to trace each implemented requirement to automated
  coverage.
- Failing snapshots or golden tests should be treated as spec regressions until
  proven otherwise.
