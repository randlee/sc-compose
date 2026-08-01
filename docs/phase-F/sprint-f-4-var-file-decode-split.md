---
id: F.4
title: Var-File Decode and Validation Split
status: complete
branch: sprint/f-4-var-file-decode-split
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-4-var-file-decode-split
target: develop
---

# Sprint F.4 — Var-File Decode and Validation Split

## Goal

- Reduce concentrated churn and defect risk in the sc-compose var-file boundary by separating filesystem loading, JSON/YAML decoding, and common variable-map validation/conversion while preserving all current input and diagnostic semantics.
## Hard Dependencies

- F.1 and F.2 must merge to develop before F.4 starts or rebases. F.4 is third in the Phase F test-file sequence: F.1 -> F.2 -> F.4 -> F.5 -> F.3; F.5 must rebase onto F.4 and F.3 must wait until all four earlier sprints are merged.
- The current crates/sc-compose/src/var_file.rs, CommandError mapping, sc-composer InputValue/VariableName APIs, and CLI var-file tests are the baseline.
- Do not duplicate semantic validation in sc-composer. The format-ingress and CLI diagnostic boundary remains owned by sc-compose.
## Exact Targets

- `crates/sc-compose/src/var_file.rs`
- `crates/sc-compose/src/commands/mod.rs`
- `crates/sc-compose/src/command_error.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- F4-D1 — Preserve load_var_file as the filesystem-facing CLI boundary while extracting JSON and YAML decoding into focused internal functions or modules.
- F4-D2 — Introduce one common validated conversion path for decoded object values, preserving string-key checks, VariableName validation, InputValue validation, and existing diagnostic codes/messages.
- F4-D3 — Preserve JSON-first/YAML-fallback behavior and duplicate-key rejection, including nested duplicate objects and malformed input classification.
- F4-D4 — Add focused unit and CLI regression coverage for nested arrays/objects, duplicate JSON/YAML keys, non-string nested YAML map keys, top-level non-object files, malformed input, and diagnostic preservation.
- F4-D5 — Plan artifact provenance is recorded honestly: the document was authored/edited outside the templated pipeline because `sc-compose validate --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --json` reproducibly returns exit 3 while parsing the canonical template's nested Jinja frontmatter. The tooling defect is tracked as unnumbered Phase F follow-on work in `docs/project-plan.md`; this sprint does not claim templated-render evidence.
## Required Work

- Keep load_var_file responsible for read_to_string and filesystem error mapping; keep decoding/conversion independently callable for unit tests.
- Represent decoded object data with a clear intermediate shape or equivalent that allows JSON and YAML to share validation/conversion without erasing source-specific errors.
- Preserve the current JSON duplicate-aware visitor behavior and YAML string-key boundary; do not silently accept a top-level scalar or sequence as a var-file.
- Use existing DiagnosticCode values and CommandError constructors unless a compatibility-preserving change is explicitly justified by tests and docs.
- Keep the implementation in sc-compose. Do not move var-file parsing into sc-composer, add ATM/runtime dependencies, or duplicate library input semantics.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
fn load_var_file(path: &Path) -> Result<BTreeMap<VariableName, InputValue>, CommandError>;
fn decode_var_file(contents: &str) -> Result<DecodedVarObject, VarFileDecodeError>;
fn validate_var_object(object: DecodedVarObject) -> Result<BTreeMap<VariableName, InputValue>, CommandError>;
```
The exact intermediate type may be JSON/YAML-neutral or an equivalent enum, but format decoding and common key/value validation must be separate seams and only load_var_file performs filesystem I/O.
## This Sprint Does Not Close

- This sprint does not change the supported recursive value contract established by Phase E or add schema validation, deep merging, bracket-path syntax, or a new resource limit.
- This sprint does not move semantic validation or InputValue ownership into sc-composer and does not duplicate that validation there.
- This sprint does not change CLI flags, var-file precedence, stdin behavior, diagnostic codes, or user-facing error classification.
## Acceptance Criteria

- JSON and YAML object var-files produce the same validated variable map for equivalent values, including nested arrays and objects.
- Duplicate JSON and YAML keys remain rejected at the same logical boundary with stable configuration diagnostics.
- Top-level non-object files, non-string YAML keys, malformed JSON/YAML, invalid variable names, and invalid values retain current diagnostic categories.
- load_var_file remains the only filesystem-loading entry point and parser/conversion functions are directly testable.
- No dependency direction or hard boundary is violated, especially the prohibition on moving this CLI ingress behavior into sc-composer.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test cli render_rejects_duplicate_json_and_yaml_var_file_keys`
- `cargo test -p sc-compose --test json_cli invalid_var_file_json_reports_config_varfile`
- `git diff --check`
