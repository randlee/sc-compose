# sc-compose v1.6.0 — Beads Composition Engine and Renderer Hardening

**Released:** August 28, 2026 · **Install:** `cargo install sc-compose` (Rust), `pip install sc-compose` (Python ≥3.11)

[Changelog](https://github.com/randlee/sc-compose/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-compose/releases/tag/v1.6.0)

---

## Beads Alchemist

**As a beads alchemist, I want to compose and instantiate beads formulas — workflow, expansion, aspect, and convoy — through sc-compose, so that I can express a recurring multi-step agent workflow once and pour it across projects.**

v1.6.0 ships the Phase R Beads composition integration (ADR-0021 accepted). sc-compose can now render `.formula.toml.j2` / `.formula.json.j2` templates into beads formula files that `bd cook` / `bd mol pour` then compile. The new host-neutral composition engine keeps sc-compose from knowing anything about `bd`'s internals — `bd` stays the sole validator and state-owner — while sc-compose supplies the templating and rendering half of the workflow.

A pinned binary integration path and a JSON-protocol CLI adapter round out the runtime: formulas composed in sc-compose flow into `bd` through a stable request/response protocol rather than ad-hoc CLI scraping. Hardening lands alongside the happy path — TOML escaping is selected at the caller boundary, invalid variable argv values are rejected, render-failure details are preserved, and rendered output is capped so a runaway formula can't flood a terminal.

---

## Release & Ops Engineer

**As a release/ops engineer, I want to compose service configs from shared fragments and generate compliance/sprint reports, so that configs stay DRY and reports are reproducible.**

The release pipeline gets a consolidation pass in v1.6.0: registry status is centralized into one component, the build ref is emitted once from a resolved main SHA, and published-channel recovery is manifest-driven and idempotent. Homebrew keyed release assets are preserved across retries, and Windows GNU release executables are packaged correctly for the first time.

These are the fixes that make the 1.6.0 bump itself reliable: unpublished-sibling package-check failures no longer block a release, and the redundant isolated verification build is skipped in preflight.

---

## Python Developer

**As a Python developer, I want Python bindings and pytest-fixture generation, so that I can drive composition from Python and generate test stubs from a list of test names.**

The Beads integration arrives with its Python half: `beads-python` gains a PyO3 adapter scaffold, wheel tests, and release metadata, plus an in-process sdist parity test that pins the Python and Rust rendering paths together. The existing `sc-compose` PyPI bindings are otherwise unchanged — this is additive.

---

## Library Consumer (Rust)

**As a Rust library consumer, I want the `sc-composer` crate with a stable API, so that I can embed composition in my own tooling.**

Renderer robustness is the Rust story this release. Three confirmed renderer bugs from the adversarial fuzz campaign are pinned as permanent regression tests, legacy JSON null serialization is made consistent, and unused escape APIs are removed. No public API break: the `sc-composer` crate surface stays stable while rendering gets measurably more correct at the edges.

> The AI-Agent Profile Author and Task-Template Author personas are unchanged this release — the renderer fixes above benefit their rendering paths indirectly, but no new profile or template surface ships in v1.6.0.

---

## What's Next

With Phase R landed, subsequent work extends the beads composition surface and continues the adversarial-fuzz hardening loop.
