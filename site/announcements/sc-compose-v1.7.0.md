# sc-compose v1.7.0 — Beads Attach & Graph Pour, sc-sha-go Bundles, JSON Append Rendering

**Released:** October 11, 2026 · **Install:** `cargo install sc-compose` (Rust/crates.io), `pip install sc-compose` (Python ≥3.11/PyPI), `brew install randlee/tap/sc-compose` (macOS/Homebrew)

[Changelog](https://github.com/randlee/sc-compose/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-compose/releases/tag/v1.7.0)

---

## Beads Alchemist

**As a beads alchemist, I want to compose and instantiate beads formulas — workflow, expansion, aspect, and convoy — through sc-compose, so that I can express a recurring multi-step agent workflow once and pour it across projects.**

v1.7.0 is the Beads release. The headline capability is `sc-compose bead attach` (and its `preview-attach` sibling), which attaches the steps of a rendered formula under an existing bead in a single `bd create --graph` transaction. Where `pour` materialized a fresh tree of dependency-tracked child issues, `attach` grafts a workflow onto a bead that already exists — existing beads are never edited, a re-run classifies each planned bead and edge as *created* or *existing*, and any missing edges are reported with concrete `bd dep add` recovery commands. Receipts carry `pour_mode`, `graph`, and `missing_edges` so the alchemist can see exactly what the graph plan produced versus what was already in place.

Alongside attach, **by-path graph pour** (`pour` and `preview-pour`) removes the registry-locality constraint: when the rendered formula lives outside the active Beads registry, the pour is planned and applied as a graph instead of requiring a copy under `formulas/`. Registry pour is unchanged, so both modes coexist without forcing a migration.

Rendering is also meaningfully hardened for formula authors. Undefined composition variables — a `{{{ var }}}` with no caller value and no frontmatter default — are now refused at render time with `BEADS_RENDER_FAILED` naming the variable, instead of silently emitting `""`/`null`. Template frontmatter is honored (declared `required` variables are enforced and `default` values apply), `@` includes expand, and relative `template`/`rendered_formula` paths resolve against `working_directory` rather than the process cwd. Windows path handling got a full pass: rendered output replaces an existing file without unlinking it first, UNC roots are preserved, and diagnostics present public paths consistently.

---

## Python Developer

**As a Python developer, I want Python bindings and pytest-fixture generation, so that I can drive composition from Python and generate test stubs from a list of test names.**

The `sc-composer-beads` Python adapter exposes the new attach and graph-pour surface, including `missing_edges` on receipts. The operation-aware formula-name boundary is shared with the CLI: `BeadComposeRequest.formula_name` is now `Option` (required only for registry pour and preview-pour), so a Python caller driving attach doesn't have to supply a name that only pour semantics demand. `pour_authorization` returns the stored wire token, and adapters classify failures through the new `stage()`/`class()` accessors with the `*_CODE` constants, matching the Rust library's diagnostic surface.

---

## Library Consumer (Rust)

**As a Rust library consumer, I want the `sc-composer` crate with a stable API, so that I can embed composition in my own tooling.**

The `sc-composer-beads` crate gains a substantial, clearly-enumerated API expansion: new request fields `parent`, `ref_`, and `relations`; new receipt fields `pour_mode`, `graph`, and `missing_edges`; and new enums — `BeadOperation::{PreviewAttach, Attach}`, `BeadPourMode`, the graph node/edge action types, and `BeadErrorClass`. The error surface adds 13 `BeadComposeError` variants (request-read, relation-endpoint, and eleven graph variants from `GraphParentNotFound` through `GraphApplyUnconfirmed`), and the enum is now `#[non_exhaustive]`, so exhaustive matches must add a wildcard arm. Adapters classify errors through `stage()`/`class()` and the `GRAPH_CODES` set. `sc-composer` also adds an opt-in `Renderer::refusing_undefined()` so library embedders can opt into the same undefined-variable refusal that Beads rendering now uses, while the default renderer stays lenient.

---

## Release & Ops Engineer

**As a release/ops engineer, I want to compose service configs from shared fragments and generate compliance/sprint reports, so that configs stay DRY and reports are reproducible.**

GitHub Releases now actually attach the `sc-sha-go` Go module bundles (`sc-sha-go-<rust-target>.zip` for Linux/amd64, macOS/arm64, and Windows/amd64) that earlier releases documented but never shipped — downstream Go consumers can pull the native module straight from the release page instead of building it. The release workflow is also consolidated to a single-sourced Go bundle toolchain. The CLI moves to the sc-observability v2 logger with the observability contract (FR-9 to FR-11) unchanged, and the error-code registry is completed: every `BEADS_*` code plus previously-unlisted `ERR_*` codes (including the render JSON depth-limit and malformed cases) are now registered.

---

## Task-Template Author

**As a task-template author, I want to generate structured XML/JSON task assignments with declared inputs, so that agents receive well-formed, validated work orders.**

`sc-compose render --append <file>` appends each rendered JSON object as one compact line to a JSON-lines file — a clean way to batch-generate task assignments from a template — refusing non-objects and files without a trailing newline, and rolling back a failed write with an inspect-path recovery hint surfaced in the `--json` envelope as `diagnostics[].path`.

---

> The **AI-Agent Profile Author** persona is unchanged this release — profile composition, resolution, and fragment reuse see no behavior change in v1.7.0, so no section is warranted.

---

## What's Next

Attach and graph pour open the door to incremental workflow grafting on existing bead graphs; the follow-on work continues the Beads surface (error-classification ergonomics for adapters, further graph diagnostics) while keeping registry pour stable.
