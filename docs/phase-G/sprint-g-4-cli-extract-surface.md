---
id: G.4
title: CLI Extract Surface
status: planned
branch: sprint/g-4-cli-extract-surface
worktree: ../sc-compose-worktrees/sprint/g-4-cli-extract-surface
target: develop
---

# Sprint G.4 — CLI Extract Surface

## Goal

Expose the known-template XML extractor as a thin, scriptable `sc-compose
extract` command. The command reads two explicit files, delegates all
semantic extraction to `sc-composer`, and presents stable human and JSON
results without writing files or invoking the renderer.

## Hard dependencies

- G.4 depends on G.1 and G.2 only; G.3 is independent and does not gate G.4.
- G.1's public contract and G.2's XML engine must be merged into the working
  baseline.
- Existing CLI command dispatch, JSON capability mapping, error-code registry,
  and stdout-cleanliness conventions are authoritative.

## Exact targets

- `crates/sc-compose/src/cli.rs`
- `crates/sc-compose/src/commands/extract.rs`
- `crates/sc-compose/src/commands/dispatch.rs`
- `crates/sc-compose/src/commands/mod.rs`
- `crates/sc-compose/src/json_output.rs` only for shared envelope wiring
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `crates/sc-compose/tests/support/mod.rs` only for reusable test mechanics
- `docs/error-code-registry.md`
- `docs/requirements.md`

## Deliverables

- `G4-D1` — Add `sc-compose extract TEMPLATE RENDERED` with repeatable
  `--include NAME` and `--exclude NAME` selection, a documented XML format
  default, `--json`, and clear help text stating that the template is
  required.
- `G4-D2` — Map file-read, malformed-output, unsupported-syntax, ambiguity,
  and extraction diagnostics through the existing CLI error and exit-code
  conventions; add stable registry entries where new codes are required.
- `G4-D3` — Preserve machine-readable stdout cleanliness. JSON output must
  include schema version, values, occurrence provenance, confidence, warnings,
  and diagnostics; logs and incidental progress must remain off stdout.
- `G4-D4` — Human output must identify the template/output inputs, recovered
  variables, confidence, warnings, and actionable unsupported/ambiguous
  guidance without dumping unbounded rendered content.
- `G4-D5` — Add text and JSON integration tests for success, filtering,
  malformed files, unsupported constructs, ambiguity, missing paths, empty
  values, exit codes, and stdout/stderr behavior.

## CLI contract

```text
sc-compose extract TEMPLATE.xml.j2 RENDERED.xml [--include NAME]...
    [--exclude NAME]... [--json]
```

The initial command is read-only and known-template only. It does not include
`identify`, directory scanning, output-file generation, or automatic template
rewriting.

Representative JSON shape:

```json
{
  "schema_version": 1,
  "values": {"task_id": "G-1"},
  "occurrences": [],
  "confidence": 1.0,
  "warnings": [],
  "diagnostics": []
}
```

The final envelope must follow the existing CLI JSON envelope conventions;
the shape above describes the extraction payload, not permission to create a
second top-level error schema.

## This sprint does not close

- changes to the extraction algorithm or supported XML subset;
- JSON/Markdown adapters or unknown-template identification;
- typed-value inference, loop reconstruction, or output-file writes;
- additional Python binding or external runtime integrations beyond G.3.

## Acceptance criteria

- The command delegates to the library API and has no duplicate extraction
  logic in `sc-compose`.
- Successful text and JSON invocations return the same recovered values and
  provenance, subject only to presentation formatting.
- Every documented failure class has a stable diagnostic, expected exit code,
  and test coverage; no failure is silently treated as an empty success.
- JSON stdout contains no logs, progress text, or backtraces on success or
  expected failure.
- Existing commands, command JSON capability decisions, and repository
  boundary tests remain green.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo test -p sc-compose --test cli extract`
- `cargo test -p sc-compose --test json_cli extract`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `git diff --check`
