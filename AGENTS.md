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
