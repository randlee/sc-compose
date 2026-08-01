---
id: F.1
title: CLI Input Parsing and JSON Capability Seams
status: planned
branch: sprint/f-1-cli-input-parsing
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-1-cli-input-parsing
target: develop
---

# Sprint F.1 — CLI Input Parsing and JSON Capability Seams

## Goal

- Reduce defect risk in the worst-health CLI module by separating process-argument normalization, repeated pass-group parsing, and JSON-output capability decisions into pure, testable seams without changing the public command line contract.
## Hard Dependencies

- F.1 is the first sprint in the Phase F test-file merge sequence: F.1 -> F.2 -> F.4 -> F.5 -> F.3. Start from develop, then merge this sprint before F.2; the later sprints must rebase onto the updated develop branch because they all touch the shared CLI integration suites.
- The existing sc-compose CLI contract, crates/sc-compose/src/cli.rs, crates/sc-compose/src/commands/compose.rs call sites, and current CLI integration tests are the implementation baseline.
- The sc-composer pure-library boundary is a hard constraint: pass parsing and JSON capability logic remain in sc-compose and must not move into sc-composer.
## Exact Targets

- `crates/sc-compose/src/cli.rs`
- `crates/sc-compose/src/commands/compose.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- F1-D1 — Extract process-independent pass-group parsing from parse_pass_inputs while preserving --pass N, --pass=N, --var, --var=, --var-file, --var-file=, ordering, and existing error text/diagnostic mapping.
- F1-D2 — Keep filtered_args_for_clap as a distinct normalization step and make its behavior independently testable without changing which pass-scoped arguments Clap receives.
- F1-D3 — Reduce command_wants_json's exhaustive decision logic to an explicit, maintainable capability seam while preserving every command and subcommand's current JSON behavior.
- F1-D4 — Add focused unit coverage for valid groups, malformed/missing values, misplaced --var/--var-file, mixed equals syntax, multiple groups, and all JSON-capable commands; retain end-to-end CLI coverage.
- F1-D5 — Plan artifact provenance is recorded honestly: the document was authored/edited outside the templated pipeline because `sc-compose validate --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --json` reproducibly returns exit 3 while parsing the canonical template's nested Jinja frontmatter. The tooling defect is tracked as unnumbered Phase F follow-on work in `docs/project-plan.md`; this sprint does not claim templated-render evidence.
## Required Work

- Define a parser input type or iterator boundary that does not read process-global argv, then adapt parse_pass_inputs at the CLI edge.
- Preserve parse_pass_inputs callers in render --all and validate --all, including stdin var-file counting and pass_inputs_parse_error recovery hints.
- Use characterization tests before changing the command capability mapping; include report and nested subcommand cases represented by the current Command enum.
- Measure the resulting seam with unit tests rather than claiming coverage from integration tests alone; do not introduce a second implementation of CLI semantics in tests.
- Keep all changes in sc-compose CLI code and tests. Do not move this behavior into sc-composer, add ATM dependencies, or change process::exit/error-output behavior.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
fn parse_pass_inputs<I>(args: I, command_name: &str) -> Result<Vec<PassInputArgs>, PassInputError>
where
    I: IntoIterator<Item = OsString>,
{
    // Parse only the supplied argument stream; the process boundary adapts std::env::args_os().
}

fn filtered_args_for_clap<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    // Remove pass-scoped arguments before Clap parsing, preserving all other tokens.
}
```
The final capability seam must preserve the current meaning of `command_wants_json(&Command) -> bool`; an equivalent command-owned helper or explicit table is acceptable only if every existing command/subcommand remains covered.
## This Sprint Does Not Close

- This sprint does not change the user-visible CLI grammar, pass ordering rules, diagnostic codes, recovery hints, stdout/stderr routing, or process exit behavior.
- This sprint does not move parsing, validation semantics, or JSON schema ownership into sc-composer.
- This sprint does not refactor commands/compose.rs beyond adapting preserved call sites.
## Acceptance Criteria

- Unit tests can invoke pass parsing and Clap filtering with supplied argument vectors, without mutating process-global argv.
- All existing --all render/validate integration tests pass unchanged or with only mechanically updated test setup, including inline variables, var-files, stdin reads, equals syntax, wrong ordering, and missing values.
- Every current JSON-capable Command and Reports/Templates/Examples subcommand retains its current JSON-vs-text selection and JSON stdout cleanliness.
- Malformed pass input still maps through pass_inputs_parse_error to the same exit code, diagnostic code, and recovery hint contract.
- No code or dependency crosses the sc-composer purity boundary, and the repository hard boundaries remain satisfied.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test cli`
- `cargo test -p sc-compose --test json_cli`
- `git diff --check`
