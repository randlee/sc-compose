---
id: D.3
title: Multi-Pass CLI Surface
status: complete
branch: sprint/d-3-cli-surface
target: integrate/phase-d
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/d-3-cli-surface
---

# Sprint D.3 — Multi-Pass CLI Surface

## Goal

- Add `--all` flag and `--pass N --var/--var-file` args to the `render` and
  `validate` commands in `sc-compose`.
- Add `--brace-count N` / `--variable-delimiters` flag for custom delimiter
  testing (GAP-10).
- Update `lib.rs` re-exports for the new multi-pass types (GAP-11).

Builds on the library foundation from [D.1](sprint-d-1-library-foundation.md)
and the composition pipeline from [D.2](sprint-d-2-composition-pipeline.md).

## Hard Dependencies

- [Sprint D.1 — Multi-Pass Library Foundation](sprint-d-1-library-foundation.md)
- [Sprint D.2 — Multi-Pass Composition Pipeline](sprint-d-2-composition-pipeline.md)
- [Phase D README](./README.md)
- [prototype/multipass/docs/gaps.md](../../prototype/multipass/docs/gaps.md) — GAP-7, GAP-10, GAP-11
- [prototype/multipass/docs/user-stories.md](../../prototype/multipass/docs/user-stories.md) — US-1, US-3
- [docs/architecture.md](../architecture.md)
- [CLAUDE.md](../../CLAUDE.md)

## Exact Targets

- `crates/sc-compose/src/cli.rs` — `--all`, `--pass N`, `--variable-delimiters` flags
- `crates/sc-compose/src/render_request.rs` — multi-pass request building
- `crates/sc-compose/src/commands/compose.rs` — multi-pass render and validate dispatch
- `crates/sc-composer/src/frontmatter.rs` — validated public `ParsedTemplate::from_parts_validated`
- `crates/sc-composer/src/lib.rs` — re-exports
- `crates/sc-compose/tests/cli.rs` — new CLI integration tests
- `docs/phase-D/sprint-d-3-cli-surface.md` — this document

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `D1` — `--all` flag and `--pass N` args (GAP-7)
  - `render` command gains `--all` flag enabling multi-pass mode
  - `--pass N --var key=val` provides per-pass inline variables
  - `--pass N --var-file path` provides per-pass variable files (JSON or YAML)
  - Passes render in outer-to-inner order (highest pass number first)
  - Single-pass templates without `--all` render identically to current behavior
  - `validate` command gains `--all` flag for multi-pass validation
  - If `--all` is used on a template with no stacked headers: emit a warning
    (via logger) and proceed with single-pass rendering — this is valid input,
    not an error

- `D2` — `--variable-delimiters` flag (GAP-10)
  - `--variable-delimiters "<open> <close>"` or `--brace-count N` on `render`
    for custom delimiter testing
  - When `--brace-count 3` is specified, delimiters are `{{{`/`}}}`
  - Mutually exclusive with `--all` (custom delimiters override per-pass
    brace-count logic)
  - Useful for testing non-standard delimiter scenarios without multi-pass

- `D3` — Re-exports (GAP-11)
  - `sc_composer::lib.rs` re-exports `PassConfig`, updated `ParsedTemplate`,
    `discover_tokens_with_brace_count`, `discover_all_pass_tokens`, `render_all`
    (builds on D.1's re-exports; D.1 ships first, no merge conflict)
  - `sc_compose` CLI imports updated to use new re-exports
  - Downstream consumers can access multi-pass types through one public API

- `D4` — CLI integration tests
  - `sc-compose render --all --pass 2 --var team=wyvern --pass 1 --var task=test template.j2`
  - `sc-compose render --all --pass 2 --var-file vars/pass2.json --pass 1 --var-file vars/pass1.json template.j2`
  - `sc-compose render --brace-count 3 template.j2` (custom delimiter)
  - `sc-compose validate --all template.j2`
  - Backward compat: `sc-compose render template.j2` without `--all` works
  - Error handling: missing `--pass N` vars, wrong pass order, invalid brace count

## Required Work

- Add `PassInputArgs` struct to `cli.rs` for per-pass variable grouping
- Add `--all` flag to `RenderArgs` and `ValidateArgs`
- Add `--pass` repeated arg (with sub-args `--var`, `--var-file`) to both commands
- Add `--variable-delimiters` or `--brace-count` to `RenderArgs`
- Update `build_request()` in `render_request.rs` to construct multi-pass
  `ComposeRequest` when `--all` is set
- Build `PassConfig` vec from `--pass N --var/--var-file` args
- Dispatch to `compose()` with multi-pass policy when `--all` active
- Update `lib.rs` re-exports
- Write CLI integration tests
- Update help text and documentation

## Explicit Code Samples

### CLI arg additions (GAP-7)

```rust
#[derive(Args)]
pub struct RenderArgs {
    // ... existing args ...

    /// Enable multi-pass rendering (stacked headers)
    #[arg(long)]
    pub all: bool,

    /// Per-pass variable inputs (outer-to-inner order)
    #[arg(long = "pass", num_args = 1.., value_name = "N")]
    pub pass_config: Vec<PassConfigArg>,

    /// Custom variable delimiter brace count (e.g., 3 → {{{ }}})
    #[arg(long = "brace-count", conflicts_with = "all")]
    pub brace_count: Option<u8>,

    /// Custom variable delimiters as two tokens (e.g., "<< >>")
    #[arg(
        long = "variable-delimiters",
        num_args = 2,
        value_names = ["OPEN", "CLOSE"],
        conflicts_with_all = ["all", "brace_count"]
    )]
    pub variable_delimiters: Option<Vec<String>>,
}

#[derive(Args, Clone, Debug)]
pub struct PassConfigArg {
    /// Pass number (1-based)
    pub pass_number: u8,

    /// Per-pass variable: key=value
    #[arg(long = "var", value_name = "KEY=VALUE")]
    pub var: Vec<String>,

    /// Per-pass variable file
    #[arg(long = "var-file", value_name = "PATH")]
    pub var_file: Option<PathBuf>,
}
```

### Multi-pass request building

```rust
fn build_multi_pass_request(
    template: &ParsedTemplate,
    pass_configs: &[PassConfigArg],
    base_policy: ComposePolicy,
) -> Result<ComposeRequest, CliError> {
    let mut passes = Vec::with_capacity(pass_configs.len());
    for cfg in pass_configs {
        passes.push(PassConfig {
            pass_number: cfg.pass_number,
            // ... build from --var and --var-file
        });
    }
    Ok(ComposeRequest {
        policy: ComposePolicy { passes, ..base_policy },
        // ...
    })
}
```

## This Sprint Does Not Close

- `template-init` multi-pass support (GAP-8) — deferred to D.4
- `verify` library entry point (GAP-6) — deferred to D.4
- `verify` CLI command (GAP-9) — deferred to D.4
- Any Python binding changes
- Multi-pass `frontmatter-init` command updates

## Acceptance Criteria

- `AC1` for `D1`
  - `sc-compose render --all --pass 2 --var team=wyvern --pass 1 --var task=test template.2.j2`
    renders correctly: pass 2 resolves `{{{ team }}}`, pass 1 resolves `{{ task }}`
  - `sc-compose render --all --pass 2 --var-file vars/p2.json --pass 1 --var-file vars/p1.json template.2.j2`
    works
  - `sc-compose render template.j2` (no `--all`) produces identical output to current behavior
  - `sc-compose validate --all template.2.j2` validates all passes
- `AC2` for `D2`
  - `sc-compose render --brace-count 3 template.j2` renders with `{{{ }}}` delimiters
  - `sc-compose render --variable-delimiters "<<" ">>" template.j2` uses custom delimiters
  - `--brace-count` and `--all` are mutually exclusive (CLI enforces)
- `AC3` for `D3`
  - `use sc_composer::{PassConfig, discover_tokens_with_brace_count, render_all};` works
  - Existing import paths unchanged
- `AC4` for `D4`
  - All new CLI paths covered by integration tests
  - Backward compat: existing CLI tests pass unchanged
  - Error handling produces non-empty stderr for: missing `--pass N` vars,
    invalid brace count (< 2), `--all` with no stacked headers
- `AC5` backward compat guard
  - `sc-compose render template.j2 --var name=world` works identically
  - `sc-compose validate template.j2` works identically
  - No new required args for single-pass templates

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test --test cli` (all CLI tests pass)
- `git diff --check`
