# Claude Instructions for sc-compose

## Critical Workflow Rule

Do not switch the main checkout away from `main` for sprint work.

- Keep the primary repo checkout on `main`
- Use git worktrees for feature work when parallel branches are needed
- Prefer short-lived feature branches targeting `develop`

## Honest Work, No Ceremony (binding on agents and humans)

Goal: working, deployable capability. Process serves it and never becomes the product.

- **Process artifacts** (certificates, ledgers, dashboards, matrices, meta-reports, speculative checks) need four things to exist: a named consumer, the feature it gates, the observed defect class that justifies it, and a deletion condition. Missing any of these → don't create it. Test: if running code branches on it, it's product. If only humans or reports read it, it's process and needs the four things. Writing code just to flip that answer is itself the pathology. Exception: a minimal integrity/recovery control (crash-recovery state, provenance snapshot) that prevents a named evidence-loss or corruption mode.
- **Real code + real tests in the same unit of work.** Forbidden: faked tests, mocks/fixtures presented as live proof, weakened assertions, regenerating goldens to force green, hard-coded success paths, placeholder macros in commits, editing the spec instead of implementing it, narrowing scope while claiming full success.
- **No self-certification.** An independent verifier closes the work, citing evidence at an exact revision. In solo sessions, re-verify by re-running, and state what wasn't independently verified.
- **Typed refusal beats fabrication but is worth less than real capability.** Work that ends only in a refusal stays open and says so.
- **Null results are successes** ("checked X, found no material increment"). Unsupported claims are worse than silence.
- **Metrics** predeclare their denominator and countermetric. Agreement between agents can raise confidence but is never independent evidence. Never silence stderr in commands that produce evidence.
- **Name pathologies when they occur:** gate self-weakening, proof-class inflation, golden regeneration, tolerance widening, suppression-pragma laundering, refusal farming, follow-up laundering. The full catalog and countermeasures are in the `just-say-no-to-process-porn-and-ceremony` skill. Ask the operator for it if you can't find it.

## Project Overview

`sc-compose` is a standalone template-composition workspace.

It contains:
- `sc-composer`: reusable rendering library
- `sc-compose`: CLI wrapper around the library

This repo is intentionally independent from ATM. Do not introduce
`agent-team-mail-*` dependencies or ATM path/runtime assumptions.

## Key Documents

- [`docs/requirements.md`](./docs/requirements.md)
- [`docs/architecture.md`](./docs/architecture.md)
- [`docs/project-plan.md`](./docs/project-plan.md)
- [`docs/git-workflows.md`](./docs/git-workflows.md)
- [`docs/cross-platform-guidelines.md`](./docs/cross-platform-guidelines.md)
- [`docs/team-protocol.md`](./docs/team-protocol.md)
- [`.claude/skills/rust-development/guidelines.txt`](./.claude/skills/rust-development/guidelines.txt)

## Boundary Rules

1. `sc-composer` must remain a pure library.
2. `sc-compose` may depend on `sc-composer`, `sc-composer-beads`, and standalone observability crates only.
3. `bindings/python` is a Python-facing adapter package that may depend on `sc-composer` only.
4. `bindings/python` must not depend on `sc-compose`, `sc-observability`, or ATM-specific crates.
5. `sc-composer` must not depend on `bindings/python`.
6. Do not read `ATM_HOME`.
7. Any ATM integration belongs in ATM adapters, not in this repo.
8. `sc-sha` is a pure computation crate. It may depend only on approved
   hashing/encoding dependencies, and may be depended on only by
   `sc-composer`, `bindings/sc-sha-python`, and `bindings/sc-sha-go`. See
   [ADR-0018](./docs/adrs/0018-sc-sha-hash-ownership.md).
9. `bindings/sc-sha-python` is a Python-facing adapter for `sc-sha` only. It
   may depend on published `sc-sha` plus PyO3/maturin packaging dependencies
   only. See [ADR-0018](./docs/adrs/0018-sc-sha-hash-ownership.md).
10. `bindings/sc-sha-go` is a generated UniFFI adapter for `sc-sha` only. It
    may depend on `sc-sha` plus its approved pinned UniFFI generation/runtime
    dependencies; it must not depend on `sc-compose`, `sc-composer`, ATM code,
    filesystem/resolver policy, or another adapter. `sc-sha` must not depend
    on it. See [ADR-0018](./docs/adrs/0018-sc-sha-hash-ownership.md) and
    [ADR-0020](./docs/adrs/0020-generated-go-binding-strategy.md).
11. `sc-composer-beads` is the host-neutral Beads formula-composition library.
    It may depend only on `sc-composer`, workspace serde/error dependencies,
    and the approved `process-wrap` platform-containment dependency. It must
    not depend on `sc-compose`, a foreign-language adapter, Beads source or
    database libraries, or ATM/runtime code. See
    [ADR-0021](./docs/adrs/0021-beads-formula-composition-integration.md).
12. `sc-compose` is the CLI adapter over `sc-composer-beads`; the Beads
    library must not depend on the CLI.
13. `bindings/sc-composer-beads-python` is the Python-facing adapter for
    `sc-composer-beads` only. It may depend on that crate plus approved
    PyO3/maturin/serde dependencies. It must not depend on `sc-compose`,
    `sc-composer`, a Beads source/database library, ATM/runtime code, or
    another adapter.

## Team Communication

If this repo is being run with ATM team workflow enabled, follow
[`docs/team-protocol.md`](./docs/team-protocol.md) for all ATM messages.
