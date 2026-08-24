---
phase: R
title: Beads Formula Composition Integration
status: planned
branch: integrate/phase-r
target: develop
related_issue: https://github.com/randlee/sc-compose/issues/551
---

# Phase R — Beads Formula Composition Integration

## Goal

Ship a Beads-aware integration that composes a `.formula.toml.j2` or
`.formula.json.j2` through the existing renderer, proves the output with the
real `bd` executable, and exposes the same operation to the CLI and Python.
The library contract is intentionally host-neutral so a future `bd compose`
implementation can invoke it without a Rust reverse dependency.

## Decisions already made

- `sc-composer` remains a generic, pure renderer. It gets no formula mode,
  Beads types, list feature, state model, or `bd` dependency.
- `crates/sc-composer-beads` owns the one render-to-`bd` integration seam.
- Beads owns formula schema, runtime variable semantics, validation, closure
  semantics, and persistent state. sc-compose does not infer or redefine any
  of them.
- Formula composition uses existing structured JSON variables and Jinja
  control blocks. The Beads request contract uses triple-brace sc-compose
  expressions so ordinary Beads `{{ variable }}` placeholders survive.
- Python is a separate Maturin adapter over `sc-composer-beads`, not a Python
  subprocess wrapper over `sc-compose`.
- Real persistent pour requires an explicit authorization value. Validation
  and preview are safe-by-default dry runs.

## Non-goals

- Change Beads or implement `bd compose` here.
- Parse/validate Beads syntax in Rust, convert Markdown into Beads structures,
  or determine Beads closure criteria.
- Add a formula-specific renderer mode, hidden copy into `.beads/formulas`, or
  a custom list/foreach language.
- Publish Python packages or execute a real non-dry-run pour as part of a test.

## Architecture

```text
CLI (`sc-compose bead`) ─┐
Python (`sc_composer_beads`) ─┼─> sc-composer-beads ─> sc-composer
future `bd compose` ─────┘              │
                                       └─> pinned `bd` executable
```

`sc-composer-beads` defines the versioned `sc-compose/beads/v1` JSON request
and receipt. It owns deterministic argument construction, stage ordering,
authorization, and process-result classification. `bd` is the only formula
validator and the only state writer.

## Sprint sequence

| Sprint | Scope | Depends on | Unblocks |
| --- | --- | --- | --- |
| R.1 | Boundary gate, host-neutral core, and real `bd` validation/preview engine | ADR-0021 approval | R.2, R.3 |
| R.2 | `sc-compose bead` CLI and JSON protocol adapter | R.1 | CLI users; future `bd compose` caller |
| R.3 | Maturin/PyO3 Python adapter and cross-surface conformance | R.1; R.2 JSON contract | Python extensions |

R.2 and R.3 may run in parallel after R.1. No sprint may treat a mocked
process runner as the sole proof: every operation claimed must also be proven
against the pinned Beads release binary on supported platforms.

## Phase acceptance criteria

- [ ] Boundary documents and sc-lint rules prohibit reverse, ATM, CLI, and
      adapter dependencies before core source is added.
- [ ] A host-neutral request can render a Beads formula with structured values
      while retaining Beads `{{ runtime_var }}` placeholders.
- [ ] `Validate` runs real `bd cook --dry-run`; `PreviewPour` runs real
      `bd mol pour --dry-run` after validation and active-registry resolution;
      failure prevents later stages.
- [ ] `Pour` cannot run unless its explicit authorization sentinel is present,
      and it never runs in CI.
- [ ] CLI JSON and Python return the same versioned request/receipt semantics
      as the Rust library.
- [ ] The exact pinned Beads binary is verified on Linux, macOS, and Windows;
      `cargo test --workspace`, Python tests, formatting, clippy, and boundary
      checks pass.

## Required phase-close validation

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
python3 -m pytest -q bindings/sc-composer-beads-python/tests
sc-compose bead validate --request <fixture-request.json> --json
sc-compose bead preview-pour --request <fixture-request.json> --json
git diff --check
```

The real Beads fixture runs on every supported CI platform using the pinned
release binary. It must cover TOML and JSON formula fixtures, structured list
expansion, multiline/Unicode Markdown values, runtime-placeholder retention,
missing `bd`, invalid formula output, missing runtime variables, and a
refused unauthorized pour. It must also prove that `bd where --json` selects
the target formula directory and that same-name TOML/JSON registry entries are
rejected rather than silently shadowed.

## Sprint index

- [Sprint R.1 — Beads contract and execution engine](sprint-r-1-beads-contract-and-engine.md)
- [Sprint R.2 — Bead CLI and JSON protocol](sprint-r-2-bead-cli-and-json-protocol.md)
- [Sprint R.3 — Beads Python bindings](sprint-r-3-beads-python-bindings.md)
