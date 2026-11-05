# ADR-0017: sc-lint Runner Allowlist and Reporting

## Status

Accepted

## Context

Phase L target sprints need one safe invocation and reporting path. Copying
sc-lint's Python utilities into sc-compose would create divergent behavior and
would leave every consumer with a different `just lint` contract.

## Decision

- `.sc/sc-lint/targets/<id>.toml` is the sole target registry. The descriptor
  supplies the stable dotted command identity and report kind; the runner
  validates the command shape and invokes only the fixed `sc-lint` executable.
  Descriptors cannot introduce arbitrary executables or shell arguments.
- The subprocess receives `--json --root <root>` and its stdout and stderr are
  captured independently. The upstream JSON envelope and exit status are
  preserved in the result and raw artifact.
- sc-compose owns report materialization; sc-lint owns analyzers, diagnostics,
  and the JSON protocol. One generic HTML path renders command, status,
  diagnostics, findings, stderr, and a raw payload link.
- The upstream configuration/capability classes remain in the raw envelope.
  Descriptor reads, descriptor parsing, unavailable executables, and artifact
  writes use distinct consumer diagnostic codes (`ERR_CONFIG_READ`,
  `ERR_CONFIG_PARSE`, `ERR_CONFIG_MODE`, and `ERR_RENDER_WRITE`). The consumer
  result uses a closed outcome enum so a new ad-hoc boolean cannot accidentally
  erase a failure class.
- The canonical Just recipes are `lint`, `view`, `check`, `clippy`, and `ci`.
  Feature sprints add a registry descriptor/fixture only and do not add a
  target-specific recipe or Python converter.
- Composite profile fix ownership remains with the Phase L Atomic target
  ownership section. This ADR governs the runner/security boundary only.

## Consequences

The runner is auditable and deterministic, but sc-lint must be installed by
the shared setup action. A missing utility is reported as a structured
capability/configuration result rather than hidden by a copied script. Raw
artifacts make CI failures independently reviewable, while later target
sprints can execute in parallel after L.2.

## References

- [Phase L plan](../phase-L/phase-L-plan.md)
- [sc-lint reporting contract](../phase-L/sc-lint-reporting-contract.md)
- [sc-lint #83](https://github.com/randlee/sc-lint/issues/83)

## Phase V compatibility amendment — 2026-11-04

The external version is now sc-lint 0.8.6 under FR-27 and amended ADR-0016.
Supported installer/runtime materialization may change during migration;
runner allowlisting, stable report identities and honest failure propagation
remain required. External docli publish preflight is governed by FR-26.

Phase V update (2026-11-05): the owner selected the final v0.8.6 release
(tag commit `6142682313171a7a714edee99942146415329f11`), superseding
the earlier 0.8.5 migration pin. Analyzer and reporting obligations remain.
