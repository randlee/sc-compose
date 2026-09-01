# sc-compose v1.6.1 — ARM64 Linux Support and Beads Runner Reliability

**Released:** August 30, 2026 · **Install:** `cargo install sc-compose` (Rust), `pip install sc-compose` (Python ≥3.11), `brew install randlee/tap/sc-compose` (macOS)

[Changelog](https://github.com/randlee/sc-compose/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-compose/releases/tag/v1.6.1)

---

## Python Developer

**As a Python developer, I want Python bindings and pytest-fixture generation, so that I can drive composition from Python and generate test stubs from a list of test names.**

v1.6.1 extends the pre-built wheel matrix with ARM64 Linux. `pip install sc-compose` now serves `manylinux_2_17_aarch64` wheels alongside the existing macOS, x86_64 Linux, and Windows targets, so Python developers on ARM64 Linux hosts — AWS Graviton, Ampere, and Linux VMs on Apple Silicon — get a compiled native extension straight from PyPI instead of building from source through maturin.

The same ARM64 Linux coverage flows through the whole distribution family: the `sc_composer_beads` and `sc_sha` wheels gain matching `aarch64` builds, keeping the Python surface uniform across architectures.

---

## Release & Ops Engineer

**As a release/ops engineer, I want to compose service configs from shared fragments and generate compliance/sprint reports, so that configs stay DRY and reports are reproducible.**

The release target matrix gains ARM64 Linux, shipping a native `aarch64-unknown-linux-gnu` binary alongside the existing macOS, Windows, and x86_64 Linux artifacts. The release pipeline itself also takes a meaningful step: sc-compose now adopts the generated `sc-sha-go` native-module packaging for its own build/release flow (Sprint S.11), so the pipeline that produces the binary is built on the same reproducible native-library layout it publishes to downstream Go consumers.

CI reliability gets tightened in the same pass: the `sprint/*` branch trigger cascade is fixed so stacked sprint PRs fire CI correctly, and the ARM64-dependent lint and release-validation jobs are gated and isolated so external lint integration can't stall or fail unrelated targets.

---

## Beads Alchemist

**As a beads alchemist, I want to compose and instantiate beads formulas — workflow, expansion, aspect, and convoy — through sc-compose, so that I can express a recurring multi-step agent workflow once and pour it across projects.**

The Beads formula runner gets a reliability pass (Sprint S.8): capture lifecycle is simplified, output-reader failures are contained so a stray capture error can't take down the whole run, a process-tree timing race in the runner's tests is removed, and the capture reader join is now proven by dedicated tests rather than assumed. No new formula surface ships — this is hardening, not feature work.

---

## Library Consumer (Rust)

**As a Rust library consumer, I want the `sc-composer` crate with a stable API, so that I can embed composition in my own tooling.**

Phase S hotspot remediation lands internally: complexity is reduced and boundary invariants are hardened across the extractor and template-lint seams, with zero public-API or behavior change. The `sc-composer` crate surface stays exactly as it was in v1.6.0 while the internals get measurably more maintainable. The arm64 Linux native-module build also extends the targets available to the generated `sc-sha-go` module, keeping the Go-consumer native library in lockstep with the new release matrix.

> The AI-Agent Profile Author and Task-Template Author personas are unchanged this release — the extractor and template-lint refactoring above is internal-only (no behavior change), and no new profile, task-template, or rendering surface ships in v1.6.1.

---

## What's Next

With ARM64 Linux delivery landed and the Go-native packaging adopted for sc-compose's own pipeline, subsequent work continues the Phase S hardening loop and the Beads runner reliability cadence established here.
