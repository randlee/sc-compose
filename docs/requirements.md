# SC-Compose Requirements

## Purpose

`sc-compose` is a standalone template-composition toolchain.

It contains:
- `sc-composer`: a reusable rendering library
- `sc-compose`: a CLI built on top of that library

Its job is to render structured outputs from templates and input data. It is not
part of ATM runtime management, mailbox handling, daemon control, or team
configuration.

## Product Requirements

1. `sc-composer` must be a pure library crate.
2. `sc-compose` must be a standalone CLI that can be used outside ATM.
3. Neither crate may depend on any `agent-team-mail-*` crate.
4. Neither crate may depend on ATM runtime concepts such as:
   - `ATM_HOME`
   - daemon sockets
   - team config stores
   - inbox or message schemas
5. If the CLI needs local state or configuration, it must use its own naming and
   paths, not ATM names or directories.
6. Logging and observability are allowed only through standalone
   `sc-observability*` crates or standard Rust logging facilities.
7. The library must be usable without the CLI.
8. The CLI must remain functional when observability is disabled.

## Boundary Rules

1. `sc-composer` may depend only on generic rendering/data dependencies.
2. `sc-compose` may depend on:
   - `sc-composer`
   - `sc-observability*`
   - generic CLI/serialization/runtime libraries
3. `sc-compose` must not import ATM home-resolution helpers.
4. `sc-compose` must not read `ATM_HOME`.
5. Any compatibility shim needed by ATM must live in ATM, not in this repo.

## Configuration Rules

1. Any repo-specific home/config path must use `SC_COMPOSE_*` naming or
   platform-default directories.
2. `HOME`/platform home resolution is acceptable.
3. ATM-specific env vars are not part of this repo's public contract.

## Non-Goals

This repo does not own:
- ATM daemon integration
- team orchestration
- ATM message or inbox formats
- ATM release compatibility shims
