---
id: R.1
title: Beads Contract and Execution Engine
status: planned
branch: sprint/r-1-beads-contract-and-engine
target: integrate/phase-r
---

# Sprint R.1 — Beads Contract and Execution Engine

## Goal

Deliver the production-ready `sc-composer-beads` library: it renders a Beads
formula with existing sc-composer behavior, calls the real `bd` executable for
validation and dry-run pour preview, and returns a host-neutral receipt.

## Pre-source gate

No Rust/Python/CLI source is authored until all of these are accepted in the
same review:

- ADR-0021 is `Accepted`.
- `docs/architecture.md`, `CLAUDE.md`, and `docs/project-plan.md` describe the
  new dependency direction.
- sc-lint boundary configuration and a negative boundary fixture enforce it.

The approved direction is:

```text
sc-compose -> sc-composer-beads -> sc-composer
bindings/sc-composer-beads-python -> sc-composer-beads
```

`sc-composer-beads` may use only `sc-composer`, workspace serde/error
dependencies, and Rust standard-library filesystem/process APIs. It may not
depend on `sc-compose`, an adapter, Beads source, ATM, or a Beads database
library.

## Exact targets

- `Cargo.toml`
- `crates/sc-composer-beads/Cargo.toml`
- `crates/sc-composer-beads/src/lib.rs`
- `crates/sc-composer-beads/src/{contract,error,execute,render,runner}.rs`
- `crates/sc-composer-beads/tests/{contract,runner,bd_integration}.rs`
- `crates/sc-composer-beads/tests/fixtures/beads/`
- `.github/actions/setup-beads/action.yml`
- `.github/workflows/ci.yml`
- `docs/adrs/0021-beads-formula-composition-integration.md`
- `docs/adrs/README.md`
- `docs/architecture.md`
- `CLAUDE.md`
- `docs/project-plan.md`
- `sc-lint.toml` and the existing sc-boundary test fixture location

## Deliverables

1. Add the pure adapter crate and its mechanically enforced boundary. It must
   expose only stable Rust contract types and direct library functions; it
   must not expose CLI structs.
2. Define `sc-compose/beads/v1` request/receipt/error types exactly as
   ADR-0021 describes. Reject an unknown schema, a non-file formula extension,
   duplicate or malformed Beads variable keys, a template escaping the working
   directory, an output outside the operation's allowed location, and a
   missing/invalid pour authorization before spawning `bd`.
3. Implement rendering through `sc-composer` with fixed triple-brace variable
   delimiters. Compose variables are structured `serde_json::Value` values;
   Beads runtime variables are sorted scalar `--var key=value` arguments.
4. Implement an injectable process-runner trait for deterministic unit tests,
   plus the production runner using `std::process::Command` with a working
   directory and argv vector. Bound captured stdout/stderr and record exit
   status, elapsed time, and stage outcome in receipts. Never use a shell.
5. Implement `Render`, `Validate`, `PreviewPour`, and `Pour` stage ordering.
   `Validate` invokes `bd cook <rendered-file> --dry-run --json`; preview and
   real pour invoke `bd where --json` and require the output formula to equal
   `<active-beads-dir>/formulas/<formula-name>.formula.{toml,json}` before
   invoking `bd mol pour <formula-name> [--dry-run] --json`. Reject a
   same-name TOML/JSON pair in the active registry. A failed stage prevents all
   later stages.
6. Add a reusable pinned-Beads setup action. Pin Beads `v1.2.2`, select the
   target-native release archive, and verify its published checksum before
   executing it. Do not rely on a developer's locally installed `bd`.
7. Add contract, runner, and real-binary integration tests. The latter create
   a temporary Beads workspace with `bd init`, then prove TOML and JSON formulas, Jinja loops
   over arrays of objects, Unicode/multiline Markdown values, preserved
   `{{ bead_var }}` placeholders, invalid formula rejection, required Beads
   variables, missing executable, redirected Beads directories, extension
   shadowing, process failure, and no real pour.

The upstream parser's shared Go `Formula` model and real `bd cook` invocation
are the formula-input authority. Do not consume `bd schema` or the Beads MCP
Pydantic issue models: neither defines a formula schema.

## Acceptance criteria

- [ ] `cargo test -p sc-composer-beads` proves every request validation and
      stage transition, including exact `bd` argv, without invoking a shell.
- [ ] Real CI runs the pinned Beads binary on Linux, macOS, and Windows and
      proves both `bd cook --dry-run` and `bd mol pour --dry-run` on rendered
      TOML and JSON fixtures.
- [ ] A request using structured JSON personae renders all expected steps and
      leaves Beads double-brace runtime placeholders unchanged.
- [ ] A request may write only its explicit output; preview/pour reject an
      output outside the `bd where` active registry, a mismatched formula name,
      or an ambiguous same-name TOML/JSON entry.
- [ ] An absent `PourAuthorization::CreatePersistentBeads` returns a stable
      refusal receipt/error and spawns no non-dry-run command.
- [ ] The crate boundary is documented and rejected dependency directions have
      a negative sc-lint fixture.

## Required validation

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p sc-composer-beads
cargo test --workspace
sc-lint --json --root . lint sc-boundary
git diff --check
```

## Out of scope

CLI argument parsing, Python packaging, package publication, `bd compose`,
formula-schema reimplementation, and a real non-dry-run pour are not R.1
deliverables.
