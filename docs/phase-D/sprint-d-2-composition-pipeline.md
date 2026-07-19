---
id: D.2
title: Multi-Pass Composition Pipeline
status: complete
branch: sprint/d-2-composition-pipeline
target: integrate/phase-d
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/d-2-composition-pipeline
---

# Sprint D.2 — Multi-Pass Composition Pipeline

## Goal

- Implement the multi-pass compose loop in `sc-composer`.
- Wire `Renderer::with_delimiters()` (public since C.2) into per-pass rendering.
- Deliver `render_all()` for programmatic multi-pass rendering.
- Verify single-pass backward compatibility end-to-end.

Builds on the stacked-header types and brace-count-aware validation from
[D.1](sprint-d-1-library-foundation.md). All work is library-only; CLI surface
is deferred to [D.3](sprint-d-3-cli-surface.md).

## Hard Dependencies

- [Sprint D.1 — Multi-Pass Library Foundation](sprint-d-1-library-foundation.md)
- [Phase D README](./README.md)
- [prototype/multipass/docs/gaps.md](../../prototype/multipass/docs/gaps.md) — GAP-3
- [prototype/multipass/docs/user-stories.md](../../prototype/multipass/docs/user-stories.md) — US-1, US-5
- [docs/architecture.md](../architecture.md)
- [CLAUDE.md](../../CLAUDE.md)

## Exact Targets

- `crates/sc-composer/src/composer.rs` — multi-pass compose loop
- `crates/sc-composer/src/renderer.rs` — verify `with_delimiters` public API (no changes needed)
- `crates/sc-composer/src/lib.rs` — re-export `render_all`, `protect_higher_braces`
- `crates/sc-composer/tests/` — new or extended integration tests
- `docs/phase-D/sprint-d-2-composition-pipeline.md` — this document

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `D1` — Multi-pass compose loop (GAP-3)
  - `compose()` and `compose_with_observer()` detect `parsed.passes.len() > 1`
    and enter a multi-pass rendering loop
  - For each pass (outer-to-inner order):
    1. Calculate `brace_count = pass_header.pass_number + 1`
    2. Build renderer via `Renderer::with_delimiters(open, close)` where
       `open = "{".repeat(brace_count)`, `close = "}".repeat(brace_count)`
    3. Protect higher-brace-count variables: wrap `{N+1}...{N+1}` blocks in
       `{% raw %}...{% endraw %}` before rendering
    4. Merge `pass_header.defaults` with caller-provided variables (caller wins)
    5. Render the body through this pass's renderer
    6. Feed rendered output forward as the next pass's body
  - Single-pass (`passes.len() <= 1`): behavior identical to current `compose()`
  - Observer events emitted per pass (command lifecycle + pass-start/pass-end)

- `D2` — Programmatic `render_all()` API
  - Public function `sc_composer::render_all(parsed: &ParsedTemplate, contexts: &[(u8, HashMap<...>)]) -> Result<String>`
  - Validates that context count matches pass count
  - Renders all passes in sequence, returns final output
  - Used by both `compose()` and future `verify()` (D.4)

- `D3` — Protection logic for higher-brace-count variables
  - `protect_higher_braces(text: &str, brace_count: usize) -> String`
  - Wraps `{N+1}...{N+1}` blocks in `{% raw %}...{% endraw %}`
  - Called before each pass's render step
  - Verified: `{% raw %}` is supported by minijinja (confirmed in prototype)

- `D4` — Integration test coverage
  - 2-pass template: deploy-time → invocation-time
  - 3-pass template: deploy → install → invocation
  - Single-pass backward compat: identical output to current behavior
  - Observer event coverage: each pass emits start/end events
  - Edge cases: empty passes list, pass_number mismatch between header and context,
    duplicate pass numbers (two passes with same pass_number → error)
  - Defaults merging: provided variables override header defaults
  - Higher-brace protection: `{{{ }}}` in pass 1 body renders as literal text

## Required Work

- Implement `compose_multi_pass()` private function in `composer.rs`
- Branch in `compose()`: if `parsed.passes.len() > 1`, delegate to `compose_multi_pass()`
- Implement `protect_higher_braces()` with the same algorithm as the
  Python prototype (confirmed correct in the committed prototype test suite)
- Implement `render_all()` public API
- Wire observer callbacks for pass lifecycle events
- Write integration tests
- Verify `cargo test --workspace` passes with zero regressions
- Verify single-header templates produce identical output
- Update `lib.rs` re-exports for `render_all` and `protect_higher_braces`

## Explicit Code Samples

### Multi-pass compose loop (GAP-3)

```rust
fn compose_multi_pass(
    parsed: &ParsedTemplate,
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
) -> Result<ComposeResult, ComposeError> {
    let mut body = parsed.body.clone();

    for pass_header in &parsed.passes {
        let brace_count = pass_header.pass_number as usize + 1;
        let open = "{".repeat(brace_count);
        let close = "}".repeat(brace_count);
        let renderer = Renderer::with_delimiters(&open, &close);

        // Protect higher-brace-count variables in this pass's body
        let protected_body = protect_higher_braces(&body, brace_count);

        // Merge defaults + provided variables
        let mut pass_vars = BTreeMap::new();
        for (k, v) in &pass_header.defaults { pass_vars.insert(k.clone(), v.clone()); }
        // ... merge request vars (request wins over defaults)

        observer.on_pass_start(&PassStartEvent::new(pass_header.pass_number));
        body = renderer.render(&protected_body, &pass_vars)
            .map_err(|e| ComposeError::Render(e))?;
        observer.on_pass_end(&PassEndEvent::new(pass_header.pass_number));
    }

    Ok(ComposeResult {
        rendered_text: body,
        // ...
    })
}
```

### Higher-brace protection

```rust
fn protect_higher_braces(text: &str, brace_count: usize) -> String {
    let higher_brace = brace_count + 1;
    let open_delim = "{".repeat(higher_brace);
    let close_delim = "}".repeat(higher_brace);

    if !text.contains(&open_delim) {
        return text.to_owned();
    }

    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(idx) = text[cursor..].find(&open_delim) {
        let abs_idx = cursor + idx;
        result.push_str(&text[cursor..abs_idx]);

        let after_open = abs_idx + higher_brace;
        if let Some(end_idx) = text[after_open..].find(&close_delim) {
            result.push_str("{% raw %}");
            result.push_str(&text[abs_idx..after_open + end_idx + higher_brace]);
            result.push_str("{% endraw %}");
            cursor = after_open + end_idx + higher_brace;
        } else {
            result.push_str(&text[abs_idx..]);
            cursor = text.len();
            break;
        }
    }
    result.push_str(&text[cursor..]);
    result
}
```

## This Sprint Does Not Close

- `--all` flag and `--pass N` CLI args (GAP-7) — deferred to D.3
- `template-init` multi-pass support (GAP-8) — deferred to D.4
- `verify` library entry point (GAP-6) — deferred to D.4
- `verify` CLI command (GAP-9) — deferred to D.4
- Custom delimiter CLI flag (GAP-10) — deferred to D.3
- Re-exports of new types (GAP-11) — deferred to D.3
- Any Python binding changes
- Any CLI changes

## Acceptance Criteria

- `AC1` for `D1`
  - 2-pass template renders correctly: pass 2 variables resolved, pass 1
    variables resolved, final output is fully concrete
  - 3-pass template renders correctly: all three passes consume their
    respective brace-count delimiters
  - Single-pass template renders identically to current `compose()`
  - Pass defaults are merged before caller variables (caller wins)
  - Higher-brace-count variables in pass N body are protected and render as
    literal text for pass N-1
- `AC2` for `D2`
  - `render_all(parsed, contexts)` returns correct output for 2-pass template
  - `render_all` errors if context count != pass count
  - `render_all` errors on pass_number mismatch
- `AC3` for `D3`
  - `protect_higher_braces("{{{ x }}}", 2)` → `"{% raw %}{{{ x }}}{% endraw %}"`
  - `protect_higher_braces("{{ x }}", 2)` → `"{{ x }}"` (no change)
- `AC4` for `D4`
  - All new compose paths covered by integration tests
  - Observer events verified per pass
  - Single-header backward compat: golden output unchanged
- `AC5` backward compat guard
  - Existing `compose()` callers see zero behavioral change for single-header
    templates
  - `Renderer` API unchanged
  - `ComposeRequest` default behavior unchanged

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test --test integration` (all integration tests pass)
- `git diff --check`
