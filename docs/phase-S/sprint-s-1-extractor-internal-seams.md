---
id: S.1
title: Extractor Internal Seams
status: complete
branch: sprint/s-1-extractor-internal-seams
worktree: ../sc-compose-worktrees/sprint/s-1-extractor-internal-seams
target: integrate/phase-s
---

# Sprint S.1 — Extractor Internal Seams

## Goal

Reduce complexity in the JSON, YAML, and XML extractors without changing a
public extraction result, diagnostic code, recovery hint, span, path, or
dependency. This closes S-T1, S-T2, and S-T3.

## Hard Dependencies

- `integrate/phase-s` exists from `develop` before this sprint branch exists.
- No ADR is required: the work remains private code inside `sc-composer`.

## Exact Targets

- `crates/sc-composer/src/extract/yaml.rs`
- `crates/sc-composer/src/extract/json.rs`
- `crates/sc-composer/src/extract/xml.rs`
- existing extractor unit/integration fixture locations only
- `docs/plans/phase-S.md`

## Deliverables

- Private YAML/JSON validation and raw-text diagnostic helper seams that retain
  format-specific codes and recovery hints.
- A narrow XML raw-text error helper that retains occurrence-path and
  adjacent-variable behavior.
- Regression coverage for malformed input, limit overflow, static mismatch,
  ambiguous delimiters, UTF-8 spans, and nested path output in all formats.

## Required Work

- Preserve `extract_yaml`, `extract_json`, and `extract_xml` entry points; do
  not turn format-specific types into a generic public API.
- Reject the detector's broad XML four-file split unless review demonstrates a
  smaller private split with no semantic churn.
- Compare serialized extraction reports on the existing corpus before/after.
- Follow `CLAUDE.md` Rule 1: this remains pure-library work with no CLI,
  adapter, process, filesystem-policy, or ATM dependency.

## Explicit Code Samples

```rust
// Private-only seam; exact names may vary.
fn map_raw_text_error(error: RawTextMatchError, context: ErrorContext) -> ExtractError;
```

The helper may take format-specific context, but may not erase concrete
JSON/YAML/XML diagnostic identity at the caller boundary.

## This Sprint Does Not Close

- CLI lint/refactoring work (S.2).
- Beads process-runner lifecycle changes (S.8).
- A public extraction API redesign or format-unifying trait.

## Acceptance Criteria

- [ ] JSON/YAML/XML success and failure reports are behaviorally identical
  before and after the seam changes.
- [ ] Existing diagnostic codes, recovery hints, paths, and UTF-8 byte offsets
  remain stable.
- [ ] No dependency or public API changes occur in `Cargo.toml` or exports.
- [ ] Focused complexity reduction is visible in the named private seams.

## gh-stack Workflow

```bash
# One-time phase-close stack, created only after all sprint PRs have merged.
git switch develop
git pull --ff-only origin develop
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base develop integrate/phase-s

# Sprint-local stack: root directly at the integration branch so its merge
# cannot merge the phase branch into develop early.
git switch integrate/phase-s
gh stack init --base integrate/phase-s sprint/s-1-extractor-internal-seams
git add crates/sc-composer/src/extract docs/plans/phase-S.md docs/phase-S/sprint-s-1-extractor-internal-seams.md
git commit -m "refactor(extract): isolate private extractor seams"
gh stack submit --auto
gh stack view --json

# Merge this sprint layer into integrate/phase-s after required review/CI.
gh stack merge <sprint-s-1-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-composer --all-targets --all-features -- -D warnings`
- `cargo test -p sc-composer`
- `cargo test --workspace`
- `git diff --check`
