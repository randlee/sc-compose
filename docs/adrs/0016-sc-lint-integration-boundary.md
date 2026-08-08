# ADR-0016: sc-lint Integration Boundary

## Status

Accepted

## Context

Phase L makes sc-compose a consumer of the released sc-lint 0.4.0 command
contract. The repository has three packages with a deliberately one-way
dependency graph: the CLI and Python adapter depend on the pure composer
library. The integration must add lint coverage without moving process or
tooling concerns into `sc-composer`, and without creating a second copy of
sc-lint's analyzer or Python runner logic.

## Decision

- Pin the supported external tool to sc-lint `0.4.0` and verify it through
  `sc-lint version --json`.
- Keep sc-lint responsible for analyzer rules, dispatch, findings, and its
  machine-readable envelope. Keep sc-compose responsible for invocation,
  normalization, and report materialization.
- Represent the three package ownership boundaries in canonical TOML under
  `boundaries/`; the inventory must reflect the actual Cargo dependency graph
  and contain no reverse or undeclared edge.
- Install the released sibling backend binaries together through the reusable
  CI setup action. A missing release asset or version mismatch is an
  actionable setup failure, not a fallback to a duplicated local runner.
- Use `CLI.CONFIG_ERROR` for invalid repository/tool configuration and
  `CLI.CAPABILITY_ERROR` for a correctly configured repository whose host lacks
  a required capability. L.2 will make these classes explicit in its result
  type.
- Do not vendor sc-lint's Python utilities. Their consumer-relative `.just/`
  lookup and the resulting packaging gap are recorded against
  [sc-lint #83](https://github.com/randlee/sc-lint/issues/83) for the later
  shared-package/maturin work.

## Consequences

The initial setup is explicit and independently testable. Feature sprints can
run in parallel after L.2 because they consume the same pinned CLI and
boundary contract. A future sc-lint packaging improvement can remove the
consumer-relative utility gap without changing sc-compose's ownership model.
Until then, utility targets must fail with the documented configuration class
rather than silently using a copied implementation.

## References

- [Phase L plan](../phase-L/phase-L-plan.md)
- [sc-lint bootstrap contract](../phase-L/sc-lint-bootstrap-contract.md)
- [sc-lint issue #83](https://github.com/randlee/sc-lint/issues/83)
