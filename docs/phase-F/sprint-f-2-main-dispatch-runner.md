---
id: F.2
title: Main Dispatch Runner and Process Boundary
status: complete
branch: sprint/f-2-main-dispatch-runner
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-2-main-dispatch-runner
target: develop
---

# Sprint F.2 — Main Dispatch Runner and Process Boundary

## Goal

- Make the sc-compose process boundary intentionally thin by isolating startup, command dispatch, error routing, and logger shutdown from main.rs, reducing the historical shotgun-surgery and defect surface without changing runtime behavior.
## Hard Dependencies

- F.1 must merge to develop before F.2 starts or rebases. F.2 is second in the Phase F test-file sequence: F.1 -> F.2 -> F.4 -> F.5 -> F.3. This ordering prevents the runner characterization changes from colliding with the earlier CLI-suite additions; F.4 and F.5 must rebase onto F.2's merged develop state.
- The current main.rs, commands::dispatch, observability::build_logger, observer_impl::CliObserver, and main tests define the baseline.
- The architecture boundary is normative: sc-compose owns CLI wiring and concrete observability; sc-composer remains a pure library and must not acquire process or logger concerns.
## Exact Targets

- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/src/commands/dispatch.rs`
- `crates/sc-compose/src/observability.rs`
- `crates/sc-compose/src/observer_impl.rs`
- `crates/sc-compose/src/main_tests.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- F2-D1 — Extract a CLI application runner or equivalent explicit seam for logger construction, command dispatch, error reporting, observer shutdown, and final exit-code selection.
- F2-D2 — Make the reporting-related coordination currently hidden by main.rs explicit at the appropriate sc-compose dispatch boundary; do not create a static dependency from main.rs to reporting internals merely to mirror history.
- F2-D3 — Preserve startup logger failure behavior, command failure behavior, JSON-vs-text error routing, observer shutdown before process exit, and all existing exit codes.
- F2-D4 — Add characterization tests around successful command execution, logger construction failure, command errors in both output modes, and shutdown ordering without requiring process-wide test mutation.
- F2-D5 — Plan artifact provenance is recorded honestly: the document was authored/edited outside the templated pipeline because `sc-compose validate --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --json` reproducibly returns exit 3 while parsing the canonical template's nested Jinja frontmatter. The tooling defect is tracked as unnumbered Phase F follow-on work in `docs/project-plan.md`; this sprint does not claim templated-render evidence.
## Required Work

- Keep main.rs as the process boundary and move only orchestration responsibilities that can be expressed through explicit runner inputs/outputs; avoid a speculative framework or generic application abstraction.
- Preserve the current `process::exit` behavior and ensure logger shutdown occurs before the final exit on both success and command-error paths.
- Preserve JSON diagnostics on stdout and human-readable diagnostics on stderr, including the fallback when JSON serialization itself fails.
- Use existing command dispatch and observer lifecycle types rather than duplicating command execution paths.
- Keep all runtime ownership in sc-compose. Do not move process management or observability wiring into sc-composer, bindings/python, or ATM.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
struct CliRun<'a> {
    cli: Cli,
    observer: &'a mut CliObserver,
}

impl CliRun<'_> {
    fn execute(self) -> i32;
}

// main remains the process boundary:
// let cli = parse_cli();
// let code = run_cli(cli);
// std::process::exit(code);
```
The exact type names may differ, but the extracted seam must own the sequence `build_logger -> dispatch -> report_error -> shutdown -> exit-code`, with no hidden global logger or process-exit behavior in lower layers.
## This Sprint Does Not Close

- This sprint does not redesign commands::dispatch or reporting APIs, and does not resolve every historical co-change with reporting modules.
- This sprint does not change command semantics, public flags, JSON schemas, diagnostic codes, logger event vocabulary, or sc-composer observer traits.
- This sprint does not make sc-composer depend on sc-observability, sc-observability-types, ATM, or process/runtime helpers.
## Acceptance Criteria

- main.rs is a thin, readable process boundary whose remaining responsibilities are explicit and documented by tests.
- Success, logger-startup failure, and command-failure paths all retain current exit codes and shutdown ordering.
- Text mode emits human diagnostics as before; JSON mode emits machine-readable envelopes without logger noise on stdout.
- Existing CLI, JSON CLI, observability, and report command tests pass without weakening assertions.
- The dependency direction remains sc-compose -> sc-composer/sc-observability only; no ATM or process concerns enter sc-composer.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test cli`
- `cargo test -p sc-compose --test json_cli`
- `git diff --check`
