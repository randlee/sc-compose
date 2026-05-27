---
id: B3
title: Source Collection, Metadata Extraction, And Render-Many
status: complete
branch: feat/sprint-B3
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B3
---

# Sprint B3 — Source Collection, Metadata Extraction, And Render-Many

## Goal

Implement the generic source-driven rendering runtime from GitHub issue `#56`
so text assets with embedded metadata can generate one artifact per source
plus aggregate pages without custom wrapper scripts per repo.

## Hard Dependencies

- [docs/phase-A/sprint-A3.md](../phase-A/sprint-A3.md)
- [docs/phase-B/sprint-B1.md](./sprint-B1.md)

## Exact Targets

- `crates/sc-compose/src/reporting/source_entry.rs`
- `crates/sc-compose/src/reporting/render_many.rs`
- `crates/sc-compose/src/render_request.rs`
- `crates/sc-composer/src/renderer.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B3.md`

## Deliverables

- one collection-input runtime for discovering source files by glob or other
  stable collection definition
- one metadata-extraction runtime for at least:
  - comment-prefix metadata
  - block-comment metadata
  - body/raw source access
- one render-many runtime for one output per source file
- one `sc-composer` library entry point exposed from `renderer.rs` that
  renders pre-loaded template content for `render_many.rs` without adding
  filesystem I/O, path walking, or repo-runtime discovery behavior to
  `sc-composer`
- one generated manifest runtime that aggregate templates and review tooling
  can consume
- one explicit statement in runtime/docs that these collection capabilities are
  generic and are not Mermaid-only
- one collection-discovery model for:
  - `glob`
  - optional `sets`
  - metadata extraction
  - deterministic `output_path`
- one deterministic sort rule so repeated runs over the same source set produce
  the same output order and manifest order

## Explicit Code Samples

```rust
pub struct SourceEntry {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub sets: Option<Vec<String>>,
}

pub struct LoadedTemplateRequest {
    pub template_name: String,
    pub template_text: String,
    pub context: BTreeMap<String, serde_json::Value>,
}

pub fn render_loaded_template(
    request: LoadedTemplateRequest,
) -> Result<RenderedArtifact, RenderError>;

pub fn render_many(request: RenderManyRequest) -> Result<RenderManyResult, RenderManyError>;
```

```toml
[[report.source_set]]
id = "state-machines"
glob = "docs/state-machines/*.yaml"
template = "shared:diagram"
output_dir = "reports/latest/state-diagrams/panels"
sets = ["publish", "latest"]
```

## This Sprint Does Not Close

- semantic diagram-spec design
- shared panel chrome/copy behavior
- archive output policy
- publish-manifest behavior

## Acceptance Criteria

- the runtime covers collection discovery, metadata extraction, render-many,
  and manifest output as one coherent source-driven path
- one catalog entry can render multiple outputs from a deterministic source set
- `sets` is optional and defaults to `None` when absent
- every rendered source entry records a deterministic `output_path`
- the manifest output is sufficient for later latest/archive and publish
  pipeline stages
- the runtime keeps the mechanism generic across Mermaid, SVG, and other text
  assets
- the `sc-composer` boundary remains runtime-agnostic and gains no filesystem
  traversal or source-discovery behavior
- the runtime keeps browser automation and site hosting out of scope

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
