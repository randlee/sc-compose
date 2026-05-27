---
id: B10
title: Built-In Render Context Variables
status: complete
branch: feat/sprint-B10
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B10
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
  4. user-template `input_defaults`
  5. frontmatter defaults
- one explicit guarantee that caller-provided values always win:
  - explicit `--var HOSTNAME=...` overrides the built-in
  - `--env-prefix` absorbed `HOSTNAME` overrides the built-in
  - frontmatter defaults do not override built-ins
- one explicit rule that `TEMPLATE_NAME` reflects the template filename
  actually rendered rather than a caller-supplied alias
- one explicit call-path rule that validation-state collection injects
  built-ins from the resolved root template path after template-owned defaults
  are merged and before environment-derived or explicit caller values are
  applied
- one explicit implementation constraint that hostname/username lookup may use
  `gethostname`, `whoami`, or `std::env::var`, but must not add
  observability-, ATM-, or daemon-lifecycle-specific dependencies

## Explicit Code Samples

```rust
struct BuiltinVarContext {
    template_name: String,
    hostname: String,
    username: String,
    render_date: String,
    render_timestamp: String,
}

fn inject_builtin_vars(
    state: &mut ValidationState,
    template_path: &Path,
) {
    BuiltinVarContext::for_template(template_path).inject_into(state);
}

fn collect_validation_state(request: &ComposeRequest, expanded: &ExpandedTemplate) -> ValidationState {
    let mut state = merge_frontmatter_defaults(expanded);
    inject_builtin_vars(&mut state, expanded.resolved_files.first().unwrap());
    apply_env_and_explicit_inputs(&mut state, request);
    state
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
