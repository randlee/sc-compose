# sc-compose v1.5.0 — Hermes Runtime, sc-publish Migration, and Go Distribution

**Released:** August 2026 · **Install:** `cargo install sc-compose` (Rust), `pip install sc-compose` (Python ≥3.11)

[Changelog](https://github.com/randlee/sc-compose/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-compose/releases/tag/v1.5.0)

---

## AI-Agent Profile Author

**As an agent-profile author, I want to compose a profile once and resolve it across Claude, Codex, Gemini, and OpenCode, so that I don't maintain per-runtime copies that drift.**

v1.5.0 adds Hermes Agent as a first-class `RuntimeKind`. Profiles composed with `sc-compose` now resolve natively to Hermes Agent profiles alongside the existing Claude, Codex, Gemini, and OpenCode targets. The same fragment you author once renders correctly to Hermes's profile structure — no per-runtime duplication, no drift.

This is the first expansion of the runtime matrix since the original GA launch. Hermes support uses the same multi-pass template resolution and declared-input validation surface as every other runtime, so existing profiles targeting Claude or Codex gain Hermes compatibility with zero structural changes. If you're already declaring inputs and using fragments, add `--agent hermes` and compose.

---

## Release & Ops Engineer

**As a release/ops engineer, I want to compose service configs from shared fragments and generate compliance/sprint reports, so that configs stay DRY and reports are reproducible.**

v1.5.0 cuts sc-compose's own release pipeline over to `sc-publish`, dogfooding the composition engine's publishing surface end-to-end. The legacy publishing workflow is removed; all channel dispatch, manifest rendering, and credential verification now flow through the composable publishing pipeline.

Preflight checks are hardened: credential verification is actionable (not opaque), channel recovery is manifest-driven, and every recovery workflow is idempotent. Winget and Scoop manifests are now rendered through sc-compose templates — no hand-edited YAML drift. The Winget publish workflow is a standalone, re-runnable dispatch. PyPI uploads are re-runnable with `maturin --skip-existing` semantics baked into the workflow contracts.

Phase P gates close the production-readiness loop: Intel-macOS compatibility guard, generator retry on transient failures, and CI fixes for cargo-deny ordering and Windows GNU executable packaging.

---

## Python Developer

**As a Python developer, I want Python bindings and pytest-fixture generation, so that I can drive composition from Python and generate test stubs from a list of test names.**

Python packaging receives targeted fixes in v1.5.0: the PyPI upload workflow is re-runnable, Python manifest alignment is corrected across both manifest files, and Maturin invocation is pinned with `--skip-existing` invariants in CI. The `requires-python >=3.11` contract is unchanged; no API surface has changed in the Python bindings.

---

## Library Consumer (Rust)

**As a Rust library consumer, I want the `sc-composer` crate with a stable API, so that I can embed composition in my own tooling.**

The Rust crate boundary remains stable through v1.5.0. The `sc-composer` crate maintains its existing API surface; no breaking changes ship. The error-code registry (`ERR_*`) is preserved. Phase P production-readiness gates ensure cross-platform stability (including Intel macOS) and generator retry reliability. Generated Go bindings for `sc-sha` ship as a companion distribution — Rust consumers are unaffected but gain confidence that the interop surface is expanding cleanly.

---

## What's Next

v1.5.0 is a feature release spanning 62 commits from Phase P and Q. The sc-publish migration is the architectural centerpiece; Hermes runtime support is the most visible end-user feature. Subsequent releases will build on this foundation with expanded runtime coverage and deeper publishing automation.