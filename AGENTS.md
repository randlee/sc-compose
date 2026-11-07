# AGENTS Instructions for sc-compose

## Must Read

Before participating in team work for this repo, read:
- `docs/team-protocol.md`

## Quick Rule

Always follow this sequence for every ATM message:
1. Immediate acknowledgement
2. Do the work
3. Completion summary
4. Immediate completion acknowledgement by receiver

No silent processing.

---

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

`sc-compose` is a standalone template-composition toolchain with no ATM
runtime dependencies.

Two crates:
- `crates/sc-composer` — pure rendering library (minijinja, serde)
- `crates/sc-compose` — CLI wrapper over the library (clap, anyhow)

Dependency direction: `sc-compose` → `sc-composer` only.

## Key Documents

- [`docs/requirements.md`](docs/requirements.md) — product requirements
- [`docs/architecture.md`](docs/architecture.md) — crate architecture and boundaries
- [`docs/project-plan.md`](docs/project-plan.md) — phase and sprint plan
- [`docs/git-workflows.md`](docs/git-workflows.md) — branch and PR conventions
- [`docs/cross-platform-guidelines.md`](docs/cross-platform-guidelines.md) — portability rules
- [`.claude/skills/rust-development/guidelines.txt`](.claude/skills/rust-development/guidelines.txt) — Rust coding guidelines (read before writing code)

## Build and Test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
```

## Hard Boundaries

Violations are blocking — no exceptions:

- No `agent-team-mail-*` crate in any `Cargo.toml`
- No `ATM_HOME` env var referenced anywhere in source
- No `use agent_team_mail::` or `use atm_*::` imports
- `sc-composer` must remain a pure library — no CLI parsing, no ATM code
- Any ATM integration belongs in ATM, not here

## Agent Startup Files

- `pm/arch-comp.md` — arch-comp (lead, Claude)
- `pm/arch-ccomp.md` — arch-ccomp (developer, Codex)
