---
id: D.4
title: template-init + verify
status: planned
branch: sprint/d-4-template-init-verify
target: develop
---

# Sprint D.4 — template-init + verify

## Goal

- Implement the `verify` library entry point (GAP-6): render a template through
  all passes, diff against a deployed file, return clean/drift status.
- Extend `frontmatter-init` → `template-init` with multi-pass support (GAP-8):
  convert a concrete file into a multi-pass stacked template.
- Add the `verify` CLI command (GAP-9): `sc-compose verify <deployed> --against <template>`.
- Keep current templates and rollback flows viable: when `template-init`
  produces an effectively single-pass template, it must normalize back to the
  current single-pass shape instead of leaving a gratuitous `pass: 1`.

Builds on D.1 (library types), D.2 (composition pipeline), and D.3 (CLI arg
infrastructure for `--pass N`).

**Split-risk mitigation:** This sprint packages two distinct features (verify +
template-init). The risk is bounded: both features share the D.2
render-path infrastructure, the prototype
(`prototype/multipass/template_init.py` + `prototype/multipass/verify.py`)
provides a validated reference implementation, and the longest-match-first
logic in template-init has been proven in the prototype test suite. If either
feature hits unexpected complexity during implementation, the sprint should
be split into D.4 (verify) and D.5 (template-init) before development
continues.

## Hard Dependencies

- [Sprint D.1 — Multi-Pass Library Foundation](sprint-d-1-library-foundation.md)
- [Sprint D.2 — Multi-Pass Composition Pipeline](sprint-d-2-composition-pipeline.md)
- [Sprint D.3 — Multi-Pass CLI Surface](sprint-d-3-cli-surface.md) (for `--pass N` CLI args)
- [Phase D README](./README.md)
- [prototype/multipass/docs/gaps.md](../../prototype/multipass/docs/gaps.md) — GAP-6, GAP-8, GAP-9
- [prototype/multipass/docs/user-stories.md](../../prototype/multipass/docs/user-stories.md) — US-2, US-4
- [prototype/multipass/template_init.py](../../prototype/multipass/template_init.py) — reference implementation
- [prototype/multipass/verify.py](../../prototype/multipass/verify.py) — reference implementation
- [docs/architecture.md](../architecture.md)
- [CLAUDE.md](../../CLAUDE.md)

## Exact Targets

- `crates/sc-composer/src/verify.rs` — NEW: verify library entry point
- `crates/sc-composer/src/lib.rs` — re-export verify
- `crates/sc-composer/src/types.rs` — `VerifyResult` type
- `crates/sc-composer/Cargo.toml` — add `similar` dependency for diff output
- `crates/sc-compose/src/commands/verify.rs` — NEW: verify CLI command
- `crates/sc-compose/src/commands/template_init.rs` — extended: multi-pass support
- `crates/sc-compose/src/cli.rs` — verify subcommand, template-init args
- `crates/sc-compose/tests/cli.rs` — verify + template-init CLI tests
- `docs/phase-D/sprint-d-4-template-init-verify.md` — this document

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `D1` — verify library entry point (GAP-6)
  - `sc_composer::verify(template_path, deployed_path, contexts) -> Result<VerifyResult, ComposeError>`
  - Renders template through all passes with provided per-pass contexts
  - Reads deployed file from disk
  - Diffs rendered output against deployed content using `similar` or
    hand-rolled unified diff
  - Returns `VerifyResult { clean: bool, diff: Option<String>, exit_code: u8 }`
  - Builtin variables (`RENDER_DATE`, `RENDER_TIMESTAMP`) are overridable for
    deterministic output
  - Observer integration: emits verify-start and verify-end events

- `D2` — verify CLI command (GAP-9)
  - `sc-compose verify <deployed-file> --against <template>`
  - `--pass N --var key=val` and `--pass N --var-file path` for per-pass vars
  - `--quiet` flag suppresses diff output (exit code only)
  - `--builtin-var KEY=VALUE` overrides builtin variables for deterministic
    comparison
  - Exit 0 if clean (identical), exit 1 if drift detected
  - Prints unified diff to stderr on drift (stdout reserved for machine output)

- `D3` — template-init multi-pass support (GAP-8)
  - Rename or extend `frontmatter-init` to `template-init` with multi-pass
    awareness
  - `sc-compose template-init <file> --pass N --var name=val [--pass M --var ...]`
  - Scans file for literal values (`val`) across all passes
  - Replaces each value with correct brace-count variable:
    - Pass 2 value → `{{{ name }}}`
    - Pass 1 value → `{{ name }}`
  - Generates stacked `---...---` headers with `pass: N`, `required_variables`,
    and `defaults`
  - If the output is effectively single-pass, omits `pass: 1` so the result
    remains compatible with the shipped `1.2.x` single-pass format
  - Longest-match-first: longest values replaced first to prevent substring
    collisions (e.g., `/home/wyvern/worktrees/wyvern` before `wyvern`)
  - `--force` overwrites existing file
  - `--dry-run` prints what would change without writing
  - Exit 1 if any value not found in file

- `D4` — Integration test coverage
  - verify: clean template, drift detected, quiet mode, file not found
  - template-init: basic conversion, dry-run, longest-match-first, value not found
  - End-to-end: template-init → parse → verify round-trip

## Required Work

### verify

- Create `crates/sc-composer/src/verify.rs` with `verify()` function
- Define `VerifyResult` in `types.rs`
- Parse template with `parse_template_document()`, render with `render_all()`
- Read deployed file, compute unified diff
- Re-export from `lib.rs`
- Emit `verify-start` and `verify-end` events via `CompositionObserver`
  (new trait methods `emit_verify_start()`/`emit_verify_end()` with default
  no-op impls added to the trait in D.1 alongside pass lifecycle events)
- Create `crates/sc-compose/src/commands/verify.rs`
- Add `verify` subcommand to `cli.rs`
- Wire per-pass vars from D.3's `--pass N` CLI infrastructure
- Implement `--quiet` and `--builtin-var` flags

### template-init

- Extend `frontmatter_init` command in `commands/template_init.rs` (or rename)
- Accept `--pass N --var name=val` repeated args
- For each pass, scan file for each variable's value
- Replace with appropriate brace-count variable (`"{" * (pass_number + 1) } name }}`)
- Generate stacked YAML headers in outer-to-inner order
- Normalize single-pass output back to the current single-header shape by
  omitting `pass: 1`
- Implement longest-match-first replacement ordering
- Preserve existing single-pass `frontmatter-init` behavior as backward compat

## Explicit Code Samples

### verify library signature (GAP-6)

```rust
/// Result of a drift check between a template and its deployed output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyResult {
    /// True if the template renders to exactly the deployed content.
    pub clean: bool,
    /// Unified diff if drift detected, None if clean.
    pub diff: Option<String>,
    /// Suggested exit code: 0 if clean, 1 if drift.
    pub exit_code: u8,
}

/// Verify that a deployed file matches its multi-pass template source.
///
/// Renders the template through all passes with the provided per-pass
/// contexts, then diffs the rendered output against the deployed file.
pub fn verify(
    template_path: impl AsRef<Path>,
    deployed_path: impl AsRef<Path>,
    contexts: &[(u8, HashMap<String, InputValue>)],
    overrides: Option<HashMap<String, String>>, // builtin var overrides
) -> Result<VerifyResult, ComposeError>;
```

### template-init multi-pass (GAP-8)

```rust
// In sc-compose/src/commands/template_init.rs (CLI crate)
/// Convert a concrete file into a multi-pass stacked template.
pub fn template_init(
    file_path: impl AsRef<Path>,
    passes: &[InitPass],
    force: bool,
    dry_run: bool,
) -> Result<FrontmatterInitResult, ComposeError>;

/// One pass's worth of variable replacements for template-init.
pub struct InitPass {
    pub pass_number: u8,
    pub variables: HashMap<String, String>, // var name → concrete value
}
```

### Longest-match-first replacement order

```rust
// Sort passes outer-to-inner (highest pass_number first) so
// outer-pass variables (longer, more specific values) are
// replaced before inner-pass variables (shorter, more general).
let mut ordered_passes: Vec<_> = passes.iter().collect();
ordered_passes.sort_by_key(|p| std::cmp::Reverse(p.pass_number));

// Within each pass, sort variable replacements by value length descending
// so "/home/wyvern/worktrees/wyvern" is replaced before "wyvern".
for pass in &ordered_passes {
    let mut vars: Vec<_> = pass.variables.iter().collect();
    vars.sort_by_key(|(_, val)| std::cmp::Reverse(val.len()));
    for (name, value) in vars {
        let brace_count = pass.pass_number + 1;
        let open = "{".repeat(brace_count);
        let close = "}".repeat(brace_count);
        let replacement = format!("{open} {name} {close}");
        content = content.replace(value, &replacement);
    }
}
```

## This Sprint Does Not Close

- `verify` builtin variable override persistence (builtin vars reset after
  verify; no config file persistence)
- `template-init` for non-text files (binary files out of scope)
- `template-init` value ambiguity detection (two different variables with the
  same concrete value; warning only)
- Python bindings for verify and template-init
- Multi-pass `frontmatter-init` deprecation (old command retained for backward
  compat alongside new `template-init`)

## Acceptance Criteria

- `AC1` for `D1`
  - `verify(template, deployed, contexts)` returns `VerifyResult { clean: true, .. }`
    when rendered output matches deployed file
  - `verify(..., overrides)` with builtin var overrides produces deterministic output
  - `verify()` returns `VerifyResult { clean: false, diff: Some(...), exit_code: 1 }`
    on drift
  - Observer events: verify-start and verify-end emitted
- `AC2` for `D2`
  - `sc-compose verify deployed.md --against template.2.j2 --pass 2 --var-file vars/p2.json --pass 1 --var-file vars/p1.json`
    works end-to-end
  - `--quiet` exits 0/1 with no diff output
  - `--builtin-var RENDER_DATE=2026-01-01` overrides builtin
  - Missing `--against` flag produces a stable missing-argument error
  - File not found produces a stable `deployed file not found` or
    `template file not found` message
- `AC3` for `D3`
  - `sc-compose template-init agent.md --pass 2 --var team=wyvern --pass 1 --var task=test`
    converts file to 2-pass template
  - `--dry-run` prints what would change, does not modify file
  - `--force` overwrites existing file
  - Single-pass output omits `pass: 1`
  - Longest-match-first: `/home/wyvern/worktrees/wyvern` replaced before `wyvern`
  - Value not found → exit 1 with `values not found in file`
  - Existing single-pass `frontmatter-init` behavior preserved
- `AC4` for `D4`
  - End-to-end round-trip: `template-init → parse → render_all → verify` produces clean
  - Drift manually introduced → verify detects it
  - All paths covered by integration tests

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test --test cli` (verify + template-init CLI tests)
- `git diff --check`
