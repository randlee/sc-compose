# sc-compose v1.8.0 — Single-Pass Beads Rendering, Direct Pour, abi3 Wheels, Six Go Coordinates

**Released:** November 7, 2026 · **Install:** `cargo install sc-compose` (Rust/crates.io), `pip install sc-compose` (Python ≥3.9/PyPI), `brew install randlee/tap/sc-compose` (macOS/Homebrew), `winget install randlee.sc-compose` (Windows)

[Changelog](https://github.com/randlee/sc-compose/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-compose/releases/tag/v1.8.0) · [Upgrade notes](#upgrading-from-v170)

---

## Beads Alchemist

**As a beads alchemist, I want to compose and instantiate beads formulas — workflow, expansion, aspect, and convoy — through sc-compose, so that I can express a recurring multi-step agent workflow once and pour it across projects.**

The headline of v1.8.0 is that sc-compose now owns formula rendering end to end. `sc-compose bead --render-by <sc-compose|bd>` selects the renderer on every `bead` subcommand. The new default, `sc-compose`, renders both `{{ name }}` and `{{{ name }}}` in a single pass, so a caller value that itself contains delimiters stays data instead of being re-expanded into an expression — the class of bug the FUZZ-171 campaign kept finding. `--render-by bd` restores the v1.7.0 split deliberately: triple braces belong to sc-compose, double braces are handed to `bd`. That flag is the compatibility switch, and it is byte-for-byte the v1.7.0 behaviour.

The second addition is a pour form that doesn't need a request document: `sc-compose bead pour FORMULA [--var KEY=VALUE ...]` takes a formula path or registry name and supplies `CreatePersistentBeads` authorization itself. The JSON request form still requires `pour_authorization` explicitly — convenience on the command line, no relaxation of the boundary for scripted callers. Advanced-renderer graph beads now record stable provenance, which the ownership checks consume on re-runs, and bead identifiers containing control characters are refused up front with `BEADS_GRAPH_ID_INVALID` across constructors, the request `parent`, and `bead:` relation endpoints.

One limitation is worth stating plainly rather than burying: **mixed graph updates are refused.** With the default renderer, `pour` and `attach` reject a run that would combine planned additions with already-existing operation-owned beads, or add edges to an otherwise unchanged graph, with `BEADS_GRAPH_ATOMIC_UPDATE_UNAVAILABLE` before any write — no atomic `bd` update carrying ownership preconditions is qualified yet. Fresh creates and exact replays are unaffected. sc-compose also does not serialize concurrent pours or attaches; under a race, `bd` decides.

---

## Task-Template Author

**As a task-template author, I want to generate structured XML/JSON task assignments with declared inputs, so that agents receive well-formed, validated work orders.**

The ADR-0026 review pass lands as a batch of rendering-correctness fixes. Template-root escapes are now reported distinctly from other resolution errors, so an out-of-root include no longer looks like a missing file. Duplicate-key detection has a single visitor owner across TOML, JSON, and YAML frontmatter — complex duplicate keys are caught, not just top-level scalars. Custom delimiters survive into diagnostics, include line-ending layout is preserved byte-for-byte, and source line numbers after a conditional include point at the right line instead of drifting.

Containment is tighter in two places that matter in practice: it holds across Windows root aliases (`\\?\`, UNC, and drive-relative spellings of the same root), and re-entry through an already-visited path is refused rather than treated as a fresh include. `--strict` is available as an explicit opt-in that turns on undeclared-variable validation for templates that declare a schema; it stays off by default, templates without frontmatter or a schema render exactly as in v1.7.0, and there is no strict-only default. Inline `--var` values also now override var-file values, matching the documented precedence.

---

## Python Developer

**As a Python developer, I want Python bindings and pytest-fixture generation, so that I can drive composition from Python and generate test stubs from a list of test names.**

Wheels are now declared for six runner coordinates for each of `sc-compose`, `sc-composer-beads`, and `sc-sha`, built as abi3-py39 rather than per-interpreter. The floor drops to `requires-python >=3.9` (Windows ARM64 is the exception at 3.11+), so a project that was pinned to 3.11 purely for the binding can move down. The Python surface gains the same selections as the CLI: `render_by=` on the Beads entry points and `strict=` for declared-schema validation, both keyword arguments with unchanged request layouts.

Error contracts got specific. `compose`, `compose_file`, and `render_loaded_template` now reject malformed JSON output in line with the CLI, raising `ScRenderError` with `ERR_RENDER_JSON_MALFORMED` instead of handing back unparseable text. A template syntax error carries `ERR_RENDER_TEMPLATE`, and the exception hint sequence behaviour is preserved through the abi3 exception types.

---

## Library Consumer (Rust)

**As a Rust library consumer, I want the `sc-composer` crate with a stable API, so that I can embed composition in my own tooling.**

The Rust API exposes the renderer selection as `BeadRenderBy` (`ScCompose` / `Bd`) on the `execute_bead_request_with_options*` functions, alongside `BeadExecutionOptions { render_by, strict }`. Note the behaviour change: **`execute_bead_request` now defaults to `BeadRenderBy::ScCompose`** instead of v1.7.0's `bd`-owned rendering. To keep the old semantics, call the `_with_options` entrypoint with `BeadExecutionOptions { render_by: BeadRenderBy::Bd, strict: false }`. Request layouts are unchanged, so the migration is a function choice, not a schema change.

The dependency tree is substantially cleaned up in this release. Archived `serde_yaml` 0.9 (RUSTSEC-2024-0449) is replaced by the maintained `yaml_serde` 0.10.7 fork under the same `serde_yaml` alias, verified identical on a 2,056-case differential corpus — note that its parser, `libyaml-rs`, is a maintained libyaml port and still contains unsafe code. `sc-sha-go` scaffolding is generated by the pinned `uniffi-bindgen` 0.31.2 CLI and committed with a CI drift check, which removes `uniffi_build` and clears RUSTSEC-2026-0374 from the active graph. Also upgraded: PyO3 0.29 → 0.30 (no API change), UniFFI 0.31.2, `sha2` 0.11 (hashes byte-identical), `ulid` 3.0.0 (26-character Crockford IDs retained), and `similar` 3.2.1 (unified diffs may lay out differently on highly repetitive input; the match/mismatch decision is unchanged). Workspace MSRV is Rust 1.94.1.

---

## Release & Ops Engineer

**As a release/ops engineer, I want to compose service configs from shared fragments and generate compliance/sprint reports, so that configs stay DRY and reports are reproducible.**

`sc-sha-go` now declares six native target coordinates with canonical release staging, and the windows/amd64 and windows/arm64 GNU cross-checks run through `cargo xwin` as a compile-time check — explicitly not an MSVC validation, and Go linking/running on those coordinates remains a manual preflight step. The install matrix is wired into Release Preflight as a dispatch-only 30-cell `go-native-preflight.yml` run with a fail-closed evidence verifier: the run fails if its required artifacts are missing or don't bind to the executed cells, rather than reporting green on partial evidence.

On the tooling side, `sc-compose reference [--json]` exports the live CLI reference model and routine CI fails on drift, so the published CLI reference can no longer silently rot. sc-lint adoption moves to 0.8.6 with a consumer-owned managed-setup wrapper and the canonical `setup-sc-lint` action kept intact, and pinned rustfmt/clippy/LLVM/clang tooling is provisioned before the checks that use it. Release publishing now opts into the sc-publish prerelease path through `release/sc-publish-install.json`, with a consumer-owned `scripts/prerelease_tag.py` whose `--dry-run` is local-only and mutates nothing; minor and major numbers are never changed by a patch bump.

---

## AI-Agent Profile Author

**As an agent-profile author, I want to compose a profile once and resolve it across Claude, Codex, Gemini, and OpenCode, so that I don't maintain per-runtime copies that drift.**

One resolution fix reaches this persona: a valid profile match is retained when other candidates in the search chain are invalid, instead of the whole resolution failing because one unrelated runtime path was malformed. Containment across Windows root aliases applies to profile resolution as well. Composition syntax, per-runtime dialect resolution, and fragment reuse are otherwise unchanged in v1.8.0.

---

## Upgrading from v1.7.0

- **Beads rendering default flipped.** Without `--render-by`, formulas now render `{{ }}` and `{{{ }}}` once in a single sc-compose pass. Add `--render-by bd` (or `BeadRenderBy::Bd` / `render_by="bd"`) to keep v1.7.0 output byte-for-byte.
- **Rust:** `execute_bead_request` defaults to `ScCompose`; use `execute_bead_request_with_options*` for the 1.7.0 behaviour.
- **`--strict` is opt-in** and only affects templates that declare a schema.
- **Python floor** is now 3.9 (abi3 wheels), Windows ARM64 3.11+.
- **Mixed graph updates** are refused with `BEADS_GRAPH_ATOMIC_UPDATE_UNAVAILABLE` before any write.

---

> Nothing in v1.8.0 changes the reporting surface (`reports init` / `smoke` / `finalize` / `render-spec` / `index` / `verify` / `publish-manifest`) or the observability contract. The reporting internals were refactored under the function-length gate with no behaviour change, and the observability contract (FR-9 to FR-11) is untouched; both remain as shipped in v1.7.0.

## What's Next

Full 30-cell platform qualification is the next gate: the V7 run closes out the install matrix, which in turn unblocks the atomic mixed-graph update that `pour` and `attach` currently refuse. The sc-publish prerelease path is now in place and unused — no prerelease or release tag was created by it in this cycle.
