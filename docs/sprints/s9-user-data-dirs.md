---
id: S9
title: User Data Directory Unification (~/.sc-compose)
status: planned
branch: feat/s9-user-data-dirs
worktree: ../sc-compose-worktrees/feat/s9-user-data-dirs
target: develop
---

# Sprint S9 — User Data Directory Unification (~/.sc-compose)

## Goal

- Unify default examples and user-template directories under `~/.sc-compose/`
- Preserve `SC_COMPOSE_DATA_DIR` and `SC_COMPOSE_TEMPLATE_DIR` env overrides exactly as today
- Make Homebrew `post_install` create `~/.sc-compose/examples` (populated) and `~/.sc-compose/templates` (empty)

## Hard Dependencies

- `develop` branch at HEAD (no blocked sprints)

## Exact Targets

- `crates/sc-compose/src/template_store.rs`
- `release/homebrew/sc-compose.rb.j2`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `data_dir()` default fallback changed from install-relative `../share/sc-compose` to `~/.sc-compose`
- `user_templates_dir()` default fallback changed from `platform_user_data_dir()/sc-compose/templates` to `~/.sc-compose/templates`
- `platform_user_data_dir()` simplified or removed if no longer needed for default resolution
- Homebrew formula `post_install` block creates `~/.sc-compose/examples`, populates from `#{share}/sc-compose/examples/`, and creates `~/.sc-compose/templates`
- unit and integration tests for new defaults, env-override precedence, and clear first-run error when dirs are absent
- docs updated so default user-facing examples/templates paths point to `~/.sc-compose/`

## Required Work

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

## This Sprint Does Not Close

- No change to `SC_COMPOSE_DATA_DIR` semantics
- No change to `SC_COMPOSE_TEMPLATE_DIR` semantics
- No runtime writes into package-managed install prefixes
- No publish/reporting behavior

## Acceptance Criteria

- `sc-compose examples` resolves from `~/.sc-compose/examples` when `SC_COMPOSE_DATA_DIR` is unset
- `sc-compose templates` resolves from `~/.sc-compose/templates` when `SC_COMPOSE_TEMPLATE_DIR` is unset
- `SC_COMPOSE_DATA_DIR` and `SC_COMPOSE_TEMPLATE_DIR` env overrides still respected
- No install-relative fallback remains for default bundled-example discovery
- Homebrew `post_install` creates `~/.sc-compose/examples` (populated) and `~/.sc-compose/templates` (empty)
- Tests cover new defaults and preserved env overrides on macOS, Linux, and Windows path conventions

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
