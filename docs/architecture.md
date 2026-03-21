# SC-Compose Architecture

## Overview

This repo contains a small two-layer architecture:

1. `sc-composer`
   - pure composition/rendering library
2. `sc-compose`
   - CLI wrapper around `sc-composer`

The dependency direction is one way:

```text
sc-compose -> sc-composer
```

If observability is used:

```text
sc-compose -> sc-observability*
```

`sc-composer` must not depend on `sc-observability*` unless there is a strong,
library-level reason to expose instrumentation hooks. The default design is to
keep observability at the CLI edge.

## Architectural Constraints

1. No crate in this repo may depend on `agent-team-mail-*`.
2. No crate in this repo may assume ATM directory layout or ATM environment
   variables.
3. Public APIs must be generic and reusable outside ATM.
4. Any ATM integration belongs in ATM adapters, not in this repo.

## Crate Roles

### `sc-composer`

Responsibilities:
- parse/load templates
- render outputs from explicit inputs
- expose reusable library APIs

Must not own:
- CLI argument parsing
- ATM compatibility code
- daemon-aware behavior
- persistent app state conventions

### `sc-compose`

Responsibilities:
- CLI argument parsing
- file/path handling for local user workflows
- invocation of `sc-composer`
- optional logging/observability wiring

Must not own:
- ATM-specific state resolution
- daemon lifecycle behavior
- message-queue or plugin behavior

## State and Paths

If the CLI persists state, logs, or config:
- path ownership belongs to `sc-compose`
- names must be `SC_COMPOSE_*` or platform-default locations
- path resolution logic must live in this repo

ATM-specific path fallback is forbidden.

## Integration Boundary

ATM may consume published crates from this repo, but this repo must not contain
ATM-specific compatibility logic as part of its core architecture.

If ATM needs custom behavior, the adaptation layer belongs in ATM.
