# Gaps in sc-compose for Multi-Pass Support

> Audit of existing crates against multi-pass requirements (2026-07-16)
> Based on `develop` branch code as of commit `3e6242b`

## Summary

| Layer | Gap count | Severity |
|-------|-----------|----------|
| `sc-composer` (library) | 6 | Blocking |
| `sc-compose` (CLI) | 5 | Blocking |
| **Total** | **11** | — |

---

## sc-composer Library Gaps

### GAP-1: `frontmatter.rs` — Single-header only

**Current:** `parse_template_document` parses exactly ONE `---...---` header. `ParsedTemplate` holds `Option<Frontmatter>`.

**Needed:** Parse stacked leading `---...---` headers. Return `Vec<PassHeader>`
where each entry has `pass_number`, `required_variables`, `defaults`,
`metadata`. The `RawFrontmatter` struct needs a `pass` field.

**Files:**
- `crates/sc-composer/src/frontmatter.rs` — `split_frontmatter()`, `parse_template_document()`, `ParsedTemplate`, `RawFrontmatter`

**Change:**
```rust
// Current
pub struct ParsedTemplate {
    frontmatter: Option<Frontmatter>,
    body: String,
}

// Needed
pub struct ParsedTemplate {
    passes: Vec<Frontmatter>,  // ordered outer-to-inner
    body: String,
}
```
`split_frontmatter()` must loop only while the next bytes at the current parse
cursor start another leading header. Later `---` lines in the body must remain
literal template content.

### GAP-2: `validation.rs` — Hardcoded `{{`/`{%` delimiters

**Current:** `next_delimiter()` calls `text.find("{{")` and `text.find("{%")`. `discover_tokens()` always scans for `{{` and `{%`.

**Needed:** `discover_tokens(text, brace_count)` accepts a brace count parameter. `next_delimiter()` must accept `brace_count` and construct the start delimiter as `"{".repeat(brace_count)` + space check. Must NOT match `{N}` inside `{N+1}`.

**Files:**
- `crates/sc-composer/src/validation.rs` — `next_delimiter()`, `discover_tokens()`, `Delimiter`

**Change:** Two new functions:
```rust
pub(crate) fn discover_tokens_with_brace_count(text: &str, brace_count: usize) -> BTreeSet<VariableName>
pub(crate) fn discover_all_pass_tokens(parsed: &ParsedTemplate) -> BTreeMap<usize, BTreeSet<VariableName>>
```

The exact-match check: after finding `"{".repeat(N)`, verify the next character is NOT `{` (to reject `{N}` matching inside `{N+1}`).

### GAP-3: `composer.rs` — Single-pass only

**Current:** `compose()` calls render exactly once. No concept of multi-pass.

**Needed:** Multi-pass loop: for each pass header (in outer-to-inner order), create a pass-specific render context with custom delimiters, render, feed output forward to next pass.

**Files:**
- `crates/sc-composer/src/composer.rs` — `compose()`, `compose_with_observer()`

**Change:** After validation, if `parsed.passes.len() > 1`, loop:
```rust
for pass_header in &parsed.passes {
    let brace_count = pass_header.pass_number + 1;
    let env = build_pass_environment(brace_count);
    body = env.render_str(&body, &pass_vars)?;
}
```

### GAP-4: `renderer.rs` — `with_options()` not public

**Current:** `Renderer::with_options()` is `pub(crate)`. No way to set custom variable delimiters from outside the crate.

**Needed:** Public API to create a `Renderer` with custom `SyntaxConfig` (custom `variable_delimiters`). Either make `with_options` public or add a new constructor like `Renderer::with_delimiters(brace_count)`.

**Files:**
- `crates/sc-composer/src/renderer.rs` — `Renderer::with_options()`

**Change:**
```rust
pub fn with_delimiters(open: &str, close: &str) -> Self {
    Self::with_options(|env| {
        env.set_syntax(minijinja::SyntaxConfig::builder()
            .variable_delimiters(open, close)
            .block_delimiters("{%", "%}")
            .build()
            .unwrap());
    })
}
```

### GAP-5: `types.rs` — No pass config types

**Current:** `ComposePolicy` has no field for pass configuration. No `PassConfig` struct exists.

**Needed:**
```rust
pub struct PassConfig {
    pub pass_number: u8,
    pub required_variables: Vec<VariableName>,
    pub defaults: BTreeMap<VariableName, InputValue>,
    pub metadata: BTreeMap<String, MetadataValue>,
}
```
`ComposePolicy` needs `passes: Vec<PassConfig>` field.

**Files:**
- `crates/sc-composer/src/types.rs` — `ComposePolicy`

### GAP-6: Missing `verify` entry point

**Current:** No `verify` function in the library.

**Needed:** `sc_composer::verify(template_path, deployed_path, vars)` — renders template with all passes, diffs against deployed file, returns `Result<(), VerifyError>` with unified diff.

**Files:** New: `crates/sc-composer/src/verify.rs`

---

## sc-compose CLI Gaps

### GAP-7: `--all` flag and `--pass N` args

**Current:** `InputArgs` has flat `--var`, `--var-file`. No per-pass grouping.

**Needed:**
- `--all` flag on `render` command to enable multi-pass mode
- `--pass N --var-file path` for per-pass variable files
- `--pass N --var key=val` for per-pass inline variables

**Files:**
- `crates/sc-compose/src/cli.rs` — `InputArgs`, `RenderArgs`
- `crates/sc-compose/src/render_request.rs` — `build_request()`

### GAP-8: Missing `template-init` multi-pass support

**Current:** `frontmatter-init` command exists but operates on single files with no pass concept. No `--pass N --var name=val` flags.

**Needed:** New `template-init` command (or extend existing `frontmatter-init`):
```
sc-compose template-init <file> \
  --pass 2 --var team_name=wyvern \
  --pass 1 --var codex_agent=cwy
```

**Files:** New or extended: `crates/sc-compose/src/commands/template_init.rs`

### GAP-9: Missing `verify` command

**Current:** No `verify` subcommand.

**Needed:**
```
sc-compose verify <deployed-file> --against <template> \
  --pass 2 --var-file vars/deploy.json \
  --pass 1 --var-file vars/install.json
```

**Files:** New: `crates/sc-compose/src/commands/verify.rs`

### GAP-10: No `--delimiter` / custom delimiter flag

**Current:** Variable delimiters are hardcoded to `{{`/`}}`. No way to specify custom surrounds from CLI.

**Needed for prototyping and edge cases:** `--variable-delimiters "<<" ">>"` or `--brace-count 3` flag. This is a lower-priority gap but useful for testing custom delimiter scenarios without multi-pass.

### GAP-11: `parse_template_document` re-exports

**Current:** Library re-exports `ParsedTemplate` and `parse_template_document` from `lib.rs`.

**Needed:** When `ParsedTemplate` changes from single `Option<Frontmatter>` to
`Vec<Frontmatter>`, the re-export and all downstream consumers must update. The
real compatibility question is the public `frontmatter()` accessor semantics,
not public field access, because the current Rust field is already private.

---

## Items We Can Leverage (No Changes Needed)

| Component | Why it works as-is |
|-----------|--------------------|
| `Renderer::with_options()` | Already supports custom `SyntaxConfig` internally — just needs to be made public |
| `ComposeRequest.vars_input/vars_env/vars_defaults` | Precedence model (input > env > defaults) works for per-pass resolution |
| `validate_input_value()` | No changes needed — pass variables use same `InputValue` type |
| `ConfiningRoot` | Path confinement unchanged for multi-pass |
| `expand_includes()` | Include expansion unchanged (multi-pass applies to resolved body) |
| `BUILTIN_VARIABLE_NAMES` | Builtins available in every pass (but `RENDER_DATE`/`RENDER_TIMESTAMP` should be overridable for `verify`) |
| `Diagnostic` system | Existing diagnostic codes cover new validation needs |
| `Observer` traits | Existing observer callbacks can wrap multi-pass events |
