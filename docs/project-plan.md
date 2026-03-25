# SC-Compose Project Plan

## Status

This repo is in initial extraction/setup.

The immediate goal is to establish:
- correct standalone crate boundaries
- zero `agent-team-mail-*` dependencies
- a clean publishable workspace structure

## Near-Term Work

1. Set up repository git flow:
   - use `main` and `develop`
   - feature branches target `develop`
   - release tags and release publication come from `main`
   - keep repo workflow and review discipline aligned with ATM
2. Match GitHub automation and protection to ATM:
   - CI triggers match the ATM repo pattern for `pull_request` and `push`
   - branch protection and rulesets match ATM for `main` and `develop`
   - GitHub secrets and environments are configured and use the same variable
     names as ATM where the workflows overlap
3. Verify repository setup end to end:
   - release preflight validates publish order and version alignment
   - release workflow is ready to publish `sc-composer` then `sc-compose`
   - workspace version stays above the source ATM workspace version that last
     published these crate names
4. Complete crates.io ownership and release readiness:
   - verify crate ownership/maintainers for `sc-composer` and `sc-compose`
   - verify publish tokens and first-release permissions
   - document the handoff from ATM-published crates to this repo
5. Make `sc-composer` fully standalone.
6. Remove any `ATM_HOME` or ATM path assumptions from `sc-compose`.
7. Verify ATM cutover readiness:
   - published crate names match the existing names used in ATM
   - replacement instructions are documented
   - no `agent-team-mail-*` dependencies remain
8. Write the migration plan after the agents are live and operating on the new
   repos.

## Implementation Phase

After repository extraction is stable, the next implementation phase is the
FR-1 through FR-9 redesign defined in:

- `docs/requirements.md`
- `docs/architecture.md`

The implementation phase is split into four ordered sprints. A later sprint may
start only after the prior sprint exit gate passes.

For sprint exit gates in this document, `qm-comp` is the QA/review agent
responsible for validating that implementation matches the normative docs and
the active quality gates.

### Sprint 1: Core Type Model and Error Boundaries

Deliverables:

- Canonical public types for frontmatter, scalar values, diagnostics, compose
  requests, compose modes, and compose results.
- Crate-owned error hierarchy for resolve, include, validation, render, and
  configuration failures.
- Public API surface aligned to the architecture document, including
  `compose() -> Result<ComposeResult, ComposeError>`.
- Workspace lint baseline and crate-level API docs.

Acceptance criteria:

- Public APIs do not leak template-engine or third-party error types.
- Frontmatter schema normalization matches the requirements doc exactly.
- `ComposeMode` uses variant-specific data rather than option soup.
- All new public types compile cleanly with `cargo clippy --all-targets -- -D warnings`.

Exit gate:

- Local `cargo build --workspace` passes.
- Local `cargo clippy --all-targets -- -D warnings` passes.
- Architecture and requirements docs still match the implemented public API.

### Sprint 2: Resolver, Includes, and Validation Semantics

Deliverables:

- Resolver policy for `.claude/{agents,commands,skills}` and
  `.agents/{agents,commands,skills}`.
- Include expansion with confinement, cycle detection, include depth limits,
  and include-chain reporting.
- Variable discovery and validation covering required variables, undeclared
  tokens, and extra provided variables.
- Validation diagnostics with stable codes and machine-readable output.

Acceptance criteria:

- Omitted runtime behavior matches the ambiguity contract in `docs/requirements.md`.
- Default mode preserves undeclared tokens and strict mode fails on them.
- Include-derived defaults and required variables merge in the specified order.
- Validation reports distinguish diagnostics from fatal errors.

Exit gate:

- Automated tests cover resolver ambiguity, include cycles, path escape
  failures, and undeclared-token behavior.
- `qm-comp` review finds no blocking mismatch between requirements and
  implementation semantics.

### Sprint 3: Composition Pipeline and CLI Surface

Deliverables:

- End-to-end composition pipeline for file and profile modes.
- CLI support for `render`, `resolve`, `validate`, `frontmatter-init`, and
  `init`.
- Guidance and prompt input handling, variable files, environment-prefix
  loading, deterministic output path derivation, and dry-run behavior.
- Typed exit code handling and JSON diagnostics output.

Acceptance criteria:

- `render`, `resolve`, and `validate` all route through the same core library
  semantics.
- `resolve` is profile-only and reports attempted paths.
- CLI guidance and prompt flags are unambiguous, including stdin behavior.
- Output-path behavior matches the architecture doc for file mode, profile
  mode, and explicit `--output`.

Exit gate:

- CLI integration tests cover each command and all documented aliases.
- Docs and CLI help text use the same option names and command semantics.

### Sprint 4: Initialization, Observability, and Release Readiness

Deliverables:

- `frontmatter-init` that inserts or rewrites normalized frontmatter.
- `init` bootstrap that creates `.prompts/`, updates `.gitignore`, scans
  templates, validates them, and emits recommendations.
- Trait-hook observability in `sc-composer` with CLI-side binding for the
  concrete observer implementation.
- Release-readiness checks for standalone boundaries and publishable crates.

Acceptance criteria:

- `init` fails when invalid templates are discovered before a user starts work.
- No `ATM_HOME` or ATM-specific path/runtime assumptions remain in either
  crate.
- Observability remains optional and degrades to a no-op when no observer is
  attached.
- Standalone publish/readiness checks pass for both crates.

Exit gate:

- `qm-comp` QA pass on the final redesign slice.
- Branch is ready for review against `develop`.

If implementation is re-sliced, each replacement plan must preserve these sprint
dependencies and the normative behavior defined in the requirements and
architecture docs.

## Rule

Any sprint plan added here must preserve the standalone boundary defined by:
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/git-workflows.md`
- `docs/publishing.md`
