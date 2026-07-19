---
id: D.1
title: Multi-Pass Library Foundation
status: complete
branch: sprint/d-1-library-foundation
target: integrate/phase-d
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/d-1-library-foundation
---

# Sprint D.1 — Multi-Pass Library Foundation

## Goal

- Define the foundational types and parsing logic for multi-pass stacked-header
  templates in `sc-composer`.
- Implement brace-count-aware token discovery and validation that correctly
  discriminates `{N}` inside `{N+1}` delimiters.
- Introduce `PassConfig` types and wire them into `ComposePolicy`.
- Treat `prototype/multipass/` as the authoritative reference behavior for the
  nested-template semantics being ported into Rust.

All work is library-only. No CLI changes, no Python binding changes. Single-pass
backward compat is verified but multi-pass composition (GAP-3) is deferred to
[D.2](sprint-d-2-composition-pipeline.md).

## Hard Dependencies

- [Phase D README](./README.md)
- [prototype/multipass/docs/gaps.md](../../prototype/multipass/docs/gaps.md) — GAP-1, GAP-2, GAP-5
- [prototype/multipass/docs/user-stories.md](../../prototype/multipass/docs/user-stories.md) — US-6
- [docs/architecture.md](../architecture.md)
- [CLAUDE.md](../../CLAUDE.md)

## Exact Targets

- `crates/sc-composer/src/frontmatter.rs` — stacked header parsing, `ParsedTemplate` type change
- `crates/sc-composer/src/validation.rs` — brace-count-aware `discover_tokens`, `next_delimiter`
- `crates/sc-composer/src/types.rs` — `PassConfig` struct, `ComposePolicy` extension
- `crates/sc-composer/src/observer.rs` — add `on_pass_start()`, `on_pass_end()`, `on_verify_start()`, and `on_verify_end()` trait methods
- `crates/sc-composer/src/lib.rs` — re-exports
- `crates/sc-composer/Cargo.toml` — no dependency changes expected
- `docs/phase-D/sprint-d-1-library-foundation.md` — this document

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `D1` — Stacked header parsing (GAP-1)
  - `ParsedTemplate` changes from `frontmatter: Option<Frontmatter>` to
    `passes: Vec<Frontmatter>`
  - `split_frontmatter()` loops only while the next bytes at the current parse
    cursor start another leading `---...---` header; later `---` lines in the
    body remain literal content
  - `parse_template_document()` returns the new shape
  - `Frontmatter` gains a `pass_number: u8` field (default 1)
  - `RawFrontmatter` gains a `pass: Option<u8>` field
  - Empty `---\n---\n` headers produce `Frontmatter { pass_number: 1, ..Default::default() }`
  - `...` delimited headers supported identically to `---`
  - Malformed YAML frontmatter fails closed instead of silently normalizing to
    an empty pass
  - Duplicate explicit `pass` values fail validation/parsing as invalid stacked
    headers

- `D2` — Brace-count-aware token discovery (GAP-2)
  - `discover_tokens(text: &str) -> BTreeSet<VariableName>` remains the public
    API for double-brace (backward compat)
  - New `discover_tokens_with_brace_count(text: &str, brace_count: usize) -> BTreeSet<VariableName>`
    for arbitrary brace counts
  - New `discover_all_pass_tokens(parsed: &ParsedTemplate) -> BTreeMap<usize, BTreeSet<VariableName>>`
  - `next_delimiter()` accepts `brace_count: usize` parameter
  - Exact-match check: after finding `"{" * N`, verify the next character is NOT `{`
    (rejects `{N}` matching inside `{N+1}`)
  - `{% %}` block delimiters are unchanged across all brace counts

- `D3` — Pass config types (GAP-5)
  - `PassConfig` struct in `types.rs`:
    ```rust
    pub struct PassConfig {
        pub pass_number: u8,
        pub required_variables: Vec<VariableName>,
        pub defaults: BTreeMap<VariableName, InputValue>,
        pub metadata: BTreeMap<String, MetadataValue>,
    }
    ```
  - `ComposePolicy` gains `passes: Vec<PassConfig>` field
  - `Frontmatter` methods: `pass_number() -> u8`, `required_variables()`,
    `defaults()`, `metadata()` — existing accessors preserved

- `D4` — Unit test coverage
  - Stacked header parsing: 0, 1, 2, and 3-header templates
  - `pass` field present/absent/default
  - `...` delimiter variants
  - Brace-count-aware discovery: brace_count=2, 3, 4
  - Exact-match: `{{` NOT matching inside `{{{`, `{{{` NOT matching inside `{{{{`
  - Mixed brace-count text: scanning for `{{` ignores `{{{ }}}` blocks
  - Backward compat: existing single-header tests pass unchanged

## Required Work

- Rewrite `split_frontmatter()` to loop over stacked `---...---` blocks
- Preserve `---` lines in the template body as ordinary content once the
  leading header stack ends
- Change `ParsedTemplate` struct from `Option<Frontmatter>` to `Vec<Frontmatter>`
- Add `pass_number` field to `Frontmatter` and `RawFrontmatter`
- Add `passes` field to `ComposePolicy`
- Add `PassConfig` struct
- Add `discover_tokens_with_brace_count()` and `discover_all_pass_tokens()`
- Extend `next_delimiter()` with brace_count parameter
- Add exact-match guard in delimiter scanning
- Update all internal callers of `parsed.frontmatter` → `parsed.passes`
- Extend `CompositionObserver` trait in `observer.rs`:
  `fn on_pass_start(&mut self, event: &PassStartEvent)`,
  `fn on_pass_end(&mut self, event: &PassEndEvent)`,
  `fn on_verify_start(&mut self, event: &VerifyStartEvent)`, and
  `fn on_verify_end(&mut self, event: &VerifyEndEvent)`
  — all with default no-op impls
- Update `lib.rs` re-exports
- Write unit tests for all new behavior
- Verify existing `cargo test` passes
- Follow the committed `prototype/multipass/parser.py` and
  `prototype/multipass/discover.py` semantics unless an ADR explicitly says
  otherwise

## Explicit Code Samples

### Frontmatter with pass_number (GAP-1)

```rust
#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    #[serde(default = "default_pass_number")]
    pass: u8,
    #[serde(default)]
    required_variables: Vec<String>,
    #[serde(default)]
    defaults: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    metadata: BTreeMap<String, serde_yaml::Value>,
}

fn default_pass_number() -> u8 { 1 }
```

### ParsedTemplate type change (GAP-1)

```rust
// Before (single-header)
pub struct ParsedTemplate {
    pub(crate) frontmatter: Option<Frontmatter>,
    pub(crate) body: String,
}

// After (stacked headers)
pub struct ParsedTemplate {
    pub(crate) passes: Vec<Frontmatter>,
    pub(crate) body: String,
}
```

### Brace-count-aware delimiter scanning (GAP-2)

```rust
pub(crate) fn next_delimiter(text: &str, brace_count: usize) -> Option<Delimiter> {
    let var_start = "{" .repeat(brace_count);
    let var_end = "}" .repeat(brace_count);
    // ... find var_start, verify NOT followed by '{'
    // ... find stmt_start "{%"
}

pub fn discover_tokens(text: &str) -> BTreeSet<VariableName> {
    discover_tokens_with_brace_count(text, 2)
}

pub fn discover_tokens_with_brace_count(
    text: &str,
    brace_count: usize,
) -> BTreeSet<VariableName> {
    // ... scan with configurable delimiters
}
```

### PassConfig (GAP-5)

```rust
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PassConfig {
    #[serde(default = "default_pass_number")]
    pub pass_number: u8,
    #[serde(default)]
    pub required_variables: Vec<VariableName>,
    #[serde(default)]
    pub defaults: BTreeMap<VariableName, InputValue>,
    #[serde(default)]
    pub metadata: BTreeMap<String, MetadataValue>,
}
```

## This Sprint Does Not Close

- Multi-pass composition loop (GAP-3) — deferred to D.2
- `--all` flag and `--pass N` CLI args (GAP-7) — deferred to D.3
- `template-init` multi-pass support (GAP-8) — deferred to D.4
- `verify` library entry point (GAP-6) — deferred to D.4
- `verify` CLI command (GAP-9) — deferred to D.4
- Custom delimiter CLI flag (GAP-10) — deferred to D.3
- Any Python binding changes
- Any CLI changes whatsoever

## Acceptance Criteria

- `AC1` for `D1`
  - `parse_template_document("hello world")` → `ParsedTemplate { passes: [], body: "hello world" }`
  - `parse_template_document("---\npass: 2\n---\nbody")` → `passes[0].pass_number == 2`
  - `parse_template_document("---\n---\n---\n---\nbody")` → `passes.len() == 2`, both `passes[0].pass_number == 1`, `passes[1].pass_number == 1`
  - `parse_template_document("---\n...\n---\n...\nbody")` → `passes.len() == 2`
  - `parse_template_document("---\ndefaults: {name: world}\n---\nhello\n---\nrule")`
    preserves the trailing `---\nrule` in the body instead of parsing it as a
    later header
  - malformed YAML frontmatter and duplicate explicit `pass` values fail closed
- `AC2` for `D2`
  - `discover_tokens_with_brace_count("{{ a }}", 2)` returns `{"a"}`
  - `discover_tokens_with_brace_count("{{{ a }}}", 3)` returns `{"a"}`
  - `discover_tokens_with_brace_count("{{{ outer }}} {{ inner }}", 3)` returns `{"outer"}` (NOT `inner`)
  - `discover_tokens_with_brace_count("{{ a }}", 3)` returns `{}` (double-brace NOT matched by triple-brace scan)
  - `discover_all_pass_tokens(parsed)` returns `{pass_number → set of var names}`
  - `discover_tokens()` without brace_count still works (backward compat)
- `AC3` for `D3`
  - `PassConfig` struct exists with all fields
  - `ComposePolicy.passes` field accessible
  - `Frontmatter.pass_number()` returns `u8`
- `AC4` for `D4`
  - All new behavior covered by unit tests
  - Existing single-header tests pass unchanged
- `AC5` backward compat guard
  - `ParsedTemplate` gains `passes: Vec<Frontmatter>` for multi-pass callers.
  - The real compatibility surface is the public
    `frontmatter() -> Option<&Frontmatter>` accessor, not direct field access:
    the field is already private in current Rust code.
  - For single-header templates, `frontmatter()` preserves current semantics.
  - For stacked templates, `frontmatter()` is documented as returning the first
    (outermost) pass for backward-compatible callers while `passes` carries the
    full multi-pass shape.
  - `Renderer` API unchanged
  - `ComposeRequest` default behavior unchanged

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `git diff --check`
