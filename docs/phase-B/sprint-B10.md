---
id: B10
title: Built-In Render Context Variables
status: draft
branch: plan/phase-B
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/phase-B
---

# Sprint B10 — Built-In Render Context Variables

## Goal

Inject a standard set of built-in variables into every render context so
templates can reference the template name, hostname, username, and render
timestamp without the caller needing to pass them as `--var` flags.

## Hard Dependencies

- [docs/phase-B/sprint-B1.md](./sprint-B1.md)

## Exact Targets

- `crates/sc-composer/src/composer.rs`
- `crates/sc-composer/src/validation.rs`
- `crates/sc-compose/src/render_request.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B10.md`

## Deliverables

- one built-in variable injection path that runs for every render context
- one standard built-in variable set:
  - `TEMPLATE_NAME`
  - `HOSTNAME`
  - `USERNAME`
  - `RENDER_DATE`
  - `RENDER_TIMESTAMP`
- one explicit merge-order rule:
  1. explicit `--var` flags
  2. `--env-prefix` absorbed environment variables
  3. built-in injected values
  4. frontmatter defaults
- one explicit guarantee that caller-provided values always win:
  - explicit `--var HOSTNAME=...` overrides the built-in
  - `--env-prefix` absorbed `HOSTNAME` overrides the built-in
  - frontmatter defaults do not override built-ins
- one explicit rule that `TEMPLATE_NAME` reflects the template filename
  actually rendered rather than a caller-supplied alias
- one explicit call-path rule that `build_render_context()` calls
  `inject_builtin_vars(...)` after caller values are merged, with
  `template_name` sourced from the resolved `BuiltinVarContext` template path
- one explicit implementation constraint that hostname/username lookup may use
  `gethostname`, `whoami`, or `std::env::var`, but must not add
  observability-, ATM-, or daemon-lifecycle-specific dependencies

## Explicit Code Samples

```rust
fn inject_builtin_vars(
    context: &mut BTreeMap<String, serde_json::Value>,
    template_name: &str,
) {
    context.entry("TEMPLATE_NAME".into()).or_insert_with(|| template_name.into());
    context.entry("HOSTNAME".into()).or_insert_with(|| hostname().into());
    context.entry("USERNAME".into()).or_insert_with(|| username().into());
    context.entry("RENDER_DATE".into()).or_insert_with(|| today_iso().into());
    context
        .entry("RENDER_TIMESTAMP".into())
        .or_insert_with(|| now_iso().into());
}

fn build_render_context(request: &BuiltinVarContext) -> BTreeMap<String, serde_json::Value> {
    let mut context = merge_caller_values(request);
    inject_builtin_vars(&mut context, request.template_path.file_name().unwrap().to_str().unwrap());
    apply_frontmatter_defaults(&mut context, request);
    context
}
```

## This Sprint Does Not Close

- report catalog behavior
- source-collection/render-many behavior
- publish-manifest behavior
- repo-specific producer command behavior

## Acceptance Criteria

- all five built-ins are present in the render context when no caller value is
  supplied
- caller `--var` overrides any built-in
- `--env-prefix` absorbed value overrides any built-in
- frontmatter default does not override a built-in
- `TEMPLATE_NAME` reflects the template filename actually rendered, not a
  caller-supplied alias
- tests cover all five built-in variables across all four priority levels

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
