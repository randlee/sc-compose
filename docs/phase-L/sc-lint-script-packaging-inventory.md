# sc-lint 0.4.0 Script Packaging Inventory

Status: inventory prepared from sc-compose commit `4586830` (integrated
Phase L.1–L.16) and the local sc-lint `v0.4.0` tag (`a2d8cce`).

Issue: [#86](https://github.com/randlee/sc-lint/issues/86) — Package sc-lint's reusable Python utilities for clean consumers.

## Executive summary

sc-compose does not own or track copies of sc-lint's Python utilities. The
repository has no tracked `.just/*.py` lint scripts. Its CI setup action
downloads the pinned sc-lint source archive and copies the archive's `.just`
Python files into the CI checkout, which is a temporary consumer-side copy.

That arrangement works for the current CI workflow but does not provide a
reusable distribution for clean consumers. In sc-lint 0.4.0, the Rust Python
adapter resolves `lint_line_counts.py`, `lint_identity_literals.py`, and
`view_findings.py` relative to the analyzed repository's `.just/` directory.
The profile workflow similarly invokes several `.just` scripts relative to
the consumer root. This is the packaging gap: the implementation is owned by
sc-lint, but the execution contract requires each consumer to materialize the
files.

The recommended ownership model is a sc-lint-owned, pip-installable package
with stable module/console entry points and package resources. sc-compose
should continue invoking the supported `sc-lint --json --root .` contract and
should not vendor or copy these scripts.

## Evidence and scope

### sc-compose ownership

The final sc-compose tree contains no tracked `.just` directory and no copied
sc-lint utility scripts. The relevant consumer-side evidence is:

| Evidence | Result |
| --- | --- |
| `git ls-files` for `.just/*.py` | No tracked files |
| `.github/actions/setup-sc-lint/action.yml` | Downloads the v0.4.0 binary archive, then separately downloads the v0.4.0 source archive and copies `*.py` into `$GITHUB_WORKSPACE/.just/` |
| `crates/sc-compose/tests/sc_lint_*.rs` | Exercises the public sc-lint command contract using checked-in fixtures; these are tests, not Python utility copies |
| `tests/fixtures/sc-lint/**` | Contains Cargo/config/report fixtures only; no reusable `.just` implementation scripts |
| `Justfile` | Calls `sc-compose lint --root . --target <profile> --json`; it does not call a copied Python utility directly |

The setup action's materialization step is useful evidence of the current
failure mode, but it is deliberately not a packaging solution: a clean
consumer outside this workflow has no supported way to obtain the utility
modules without reproducing the source-archive copy operation.

### Target-to-implementation matrix

| Phase L target | Current sc-lint 0.4.0 implementation | Supporting evidence | Packaging disposition |
| --- | --- | --- | --- |
| L.1 bootstrap/version | Rust CLI and release setup; no target-specific Python implementation | `crates/sc-lint/src/cli.rs`, `crates/sc-lint/src/tests.rs`, `sc-lint version --json` | Keep in the sc-lint binary/release contract |
| L.2 runner/reports | Rust `command.rs`/`workflow.rs` plus `.just/run_lint.py`, `.just/python_adapter.py`, and shared helpers | `crates/sc-lint/src/workflow.rs`, `crates/sc-lint/src/python_adapter.rs`, `.just/tests/test_run_lint.py`, `.just/tests/test_python_adapter.py` | Package the profile/adapter module; keep report schema versioned |
| L.3 `lint sc-boundary` | Legacy `.just/lint_sc_boundary.py` wrapper invokes `sc-lint-boundary`; supported path is the Rust backend/CLI | `.just/tests/test_lint_sc_boundary.py`, `crates/sc-lint/src/tests.rs`, `crates/sc-lint-boundary/` | Retain only as a compatibility wrapper; do not require consumers to copy it |
| L.4 `lint sc-portability` | `.just/lint_sc_portability.py` wrapper invokes the Rust backend and emits the common report | `.just/tests/test_lint_sc_portability.py`, `crates/sc-lint-portability/` | Package the adapter or replace the wrapper with the supported module entry point |
| L.5 `lint sc-runtime` | Rust backend and dispatch; no representative Python implementation | `crates/sc-lint/src/dispatch.rs`, `crates/sc-lint-runtime/`, `crates/sc-lint/src/tests.rs` | No Python package row needed; preserve the Rust CLI |
| L.6 `lint line-counts` | `.just/lint_line_counts.py` using `.just/python_adapter.py`, `.just/lint_common.py`, and `.just/view_common.py` | `.just/tests/test_lint_line_counts.py`, `crates/sc-lint/src/python_adapter.rs` | First-class package module/console entry point |
| L.7 `lint identity-literals` | `.just/lint_identity_literals.py` using the shared adapter/common modules | `.just/tests/test_lint_identity_literals.py`, `crates/sc-lint/src/python_adapter.rs` | First-class package module/console entry point |
| L.8 `view findings` | `.just/view_findings.py` plus `.just/view_common.py`, `.just/python_adapter.py` | `.just/tests/test_view_findings.py`, `.just/tests/test_view_common.py`, `crates/sc-lint/src/python_adapter.rs` | First-class package module/console entry point |
| L.9 `check native` | Rust dispatch/workflow; no Python implementation | `crates/sc-lint/src/workflow.rs`, `crates/sc-lint/src/tests.rs` | Keep in the sc-lint binary |
| L.10 `check xwin` | Rust dispatch/workflow; no Python implementation | `crates/sc-lint/src/workflow.rs`, `crates/sc-lint/src/tests.rs` | Keep in the sc-lint binary; capability-gate xwin |
| L.11 `clippy native` | Rust dispatch/workflow; no Python implementation | `crates/sc-lint/src/workflow.rs`, `crates/sc-lint/src/tests.rs` | Keep in the sc-lint binary |
| L.12 `clippy xwin` | Rust dispatch/workflow; no Python implementation | `crates/sc-lint/src/workflow.rs`, `crates/sc-lint/src/tests.rs` | Keep in the sc-lint binary; capability-gate xwin |
| L.13 `lint fast` | Rust profile workflow plus `.just/run_lint.py` semantics and common checks | `crates/sc-lint/src/workflow.rs`, `.just/run_lint.py`, `.just/tests/test_run_lint.py` | Package the profile runner and shared utilities; do not copy scripts |
| L.14 `lint full` | Rust profile workflow plus Python-backed deny/shear/version/manifest/spell/pytest and target adapters | `crates/sc-lint/src/workflow.rs`, `.just/run_lint.py`, `.just/tests/test_run_lint.py` | Package common utilities; retain Rust-owned checks in the binary |
| L.15 `lint ci` | Rust profile workflow plus Python-backed deny/shear/version/manifest/spell/pytest and portability checks | `crates/sc-lint/src/workflow.rs`, `.just/run_lint.py`, `.just/tests/test_run_lint.py` | Package common utilities; preserve CI profile semantics |
| L.16 top-level `ci` | Rust/Just composition of workspace CI and the sc-lint profile | `Justfile`, `.github/workflows/ci.yml`, `crates/sc-lint/src/workflow.rs` | No separate Python package entry point |

## Reusable script inventory

The rows below are the implementation files that should be considered for a
shared package or a stable package resource. Every path is from the pinned
sc-lint v0.4.0 source tree at `../sc-lint`.

| Script/module | Role and current invocation | Supporting tests/contracts | Reuse recommendation |
| --- | --- | --- | --- |
| `.just/python_adapter.py` | Defines the `sc-lint-python-v1` success/error JSON envelope and stdout serializer | `.just/tests/test_python_adapter.py`, `crates/sc-lint/src/python_adapter.rs` | Package as the stable shared protocol module |
| `.just/lint_common.py` | Shared root discovery, config, Rust-file, report, directive, and log helpers | `.just/tests/test_lint_common.py`, imported by line-counts/identity/common checks | Package as an internal shared module, not a public copied script |
| `.just/view_common.py` | Artifact/view directory and JSON/text materialization helpers | `.just/tests/test_view_common.py`, imported by view/line-counts/identity | Package as an internal resource helper |
| `.just/lint_line_counts.py` | Python-backed line-count analysis and adapter payload | `.just/tests/test_lint_line_counts.py`, `sc_lint_line_counts.rs` in sc-compose | First-class module and console entry point |
| `.just/lint_identity_literals.py` | Python-backed identity-literal analysis and directive policy | `.just/tests/test_lint_identity_literals.py`, `sc_lint_identity_literals.rs` | First-class module and console entry point |
| `.just/view_findings.py` | Collates findings artifacts into stable JSON/text views | `.just/tests/test_view_findings.py`, `sc_lint_view_findings.rs` | First-class module and console entry point |
| `.just/lint_sc_portability.py` | Legacy Python wrapper around the Rust portability backend | `.just/tests/test_lint_sc_portability.py`, `sc_lint_sc_portability.rs` | Ship only as compatibility support; prefer Rust CLI/module dispatch |
| `.just/lint_sc_boundary.py` | Legacy Python wrapper around the Rust boundary backend | `.just/tests/test_lint_sc_boundary.py`, `sc_lint_sc_boundary.rs` | Ship only as compatibility support; do not make it a consumer copy requirement |
| `.just/run_lint.py` | Profile orchestration, ordering, parallelism, logs, and result aggregation | `.just/tests/test_run_lint.py`, Rust `workflow.rs` | Package profile semantics or retire the duplicate in favor of Rust workflow; do not maintain two divergent runners |
| `.just/check_version_sync.py` | Validates workspace/package version consistency | `.just/tests/test_check_version_sync.py` | Keep sc-lint-owned; expose only if clean consumers need the same generic check |
| `.just/lint_manifests.py` | Validates manifest completeness and publish metadata | `.just/tests/test_lint_manifests.py` | Keep sc-lint-owned; package if it remains a supported profile step |
| `.just/lint_codespell.py` | Runs the configured codespell check and report | `.just/tests/test_lint_external_tools.py` | Keep sc-lint-owned; package only with an explicit external-tool/version contract |
| `.just/run_pytests.py` | Discovers/runs sc-lint Python tests and summarizes fixtures | `.just/tests/test_run_pytests.py` | Test/developer utility, not a consumer-facing package entry point |
| `.just/lint_cargo_deny.py` | Runs cargo-deny and normalizes its report | `.just/tests/test_lint_external_tools.py` | Keep as sc-lint-owned CI utility; no consumer copy |
| `.just/lint_cargo_shear.py` | Runs cargo-shear and normalizes its report | `.just/tests/test_lint_external_tools.py` | Keep as sc-lint-owned CI utility; no consumer copy |
| `.just/lint_cargo_modules.py` | Checks Cargo module/dependency organization | `.just/tests/test_lint_cargo_modules.py` | Keep as sc-lint-owned CI utility; no consumer copy |
| `.just/run_version.py`, `.just/run_fmt.py`, `.just/print_help.py` | Just-facing helper/report adapters | `.just/tests/test_run_version.py`, `.just/tests/test_run_fmt.py`, `.just/tests/test_print_help.py` | Internal sc-lint tooling; do not expose as duplicated consumer scripts |
| `.just/fixture_constants.py` | Fixture/test support only | Imported by sc-lint `.just/tests` | Keep test-only; never package as a consumer utility |

## The current failure mode

At v0.4.0, `crates/sc-lint/src/python_adapter.rs` maps the three Python
tools to `.just/lint_line_counts.py`, `.just/lint_identity_literals.py`, and
`.just/view_findings.py`, then executes `python3`/`python` with the analyzed
repository as the current directory. `crates/sc-lint/src/workflow.rs` builds
the same consumer-relative `.just` paths for profile steps.

Therefore a clean consumer without `.just` receives a configuration/backend
failure such as “file not found” before the Python utility can run. The
sc-compose CI action currently avoids that failure only by downloading the
sc-lint source archive and copying its `.just/*.py` files into the checkout.
That workaround is intentionally rejected as a long-term distribution model:
it duplicates ownership, couples consumers to repository layout, and makes
version/schema compatibility implicit.

## Exact GitHub issue body

### Problem

sc-lint 0.4.0 owns several Python-backed lint/report utilities, but its Rust
adapter resolves them from the analyzed repository's `.just/` directory.
sc-compose does not track copies; its CI downloads the sc-lint source archive
and materializes `.just/*.py` at runtime. A clean consumer therefore needs an
undocumented source-copy workaround instead of a supported installable tool.

### Evidence from sc-compose Phase L

- sc-compose tracks no `.just/*.py` lint implementation files.
- `.github/actions/setup-sc-lint/action.yml` downloads v0.4.0 source and copies
  every `*.py` file into `$GITHUB_WORKSPACE/.just/`.
- `crates/sc-lint/src/python_adapter.rs` hard-codes the consumer-relative
  paths `.just/lint_line_counts.py`, `.just/lint_identity_literals.py`, and
  `.just/view_findings.py`.
- `crates/sc-lint/src/workflow.rs` hard-codes consumer-relative profile paths
  such as `.just/lint_sc_portability.py`, `.just/lint_line_counts.py`, and
  `.just/run_pytests.py`.
- The sc-compose integration tests exercise the stable `sc-lint --json
  --root .` contract, but do not and should not vendor the Python scripts.

### Requested outcome

Create a sc-lint-owned pip-installable package for the commonly reused Python
utilities, with a stable API that works from a clean consumer repository and
does not require copied `.just` files.

### Recommended package design

1. Add a package such as `sc_lint_tools/` to the sc-lint distribution. Put the
   adapter protocol, common helpers, artifact helpers, and the supported
   line-counts, identity-literals, and findings implementations in modules
   under that package.
2. Provide a stable dispatcher such as `python -m sc_lint_tools <tool>` and
   console entry points for the supported utilities. The Rust adapter should
   invoke the module/entry point with an explicit `--root`, rather than
   resolving a source file under `<consumer>/.just/`.
3. Ship non-code templates/resources through package resources
   (`importlib.resources` or the equivalent packaging mechanism), not by
   requiring a consumer checkout to contain sc-lint's source tree.
4. Preserve the existing `sc-lint-python-v1` JSON envelope, diagnostics,
   target identity, and exit-status semantics. Version the package contract
   with the sc-lint CLI and reject incompatible schema versions explicitly.
5. Pin and document supported Python versions, sc-lint version coupling, and
   Windows/macOS/Linux behavior. The entry points must use the same explicit
   consumer root and config paths on all platforms.
6. Add wheel and sdist tests that install into a clean temporary consumer with
   no `.just` directory, run each supported utility through the Rust adapter,
   and compare the JSON envelope and representative reports with the source
   checkout behavior. Add an upgrade/migration test for the version contract.
7. Keep Rust-owned targets (`sc-boundary`, `sc-runtime`, check, clippy, and
   xwin variants) in the sc-lint binaries; packaging Python utilities must not
   duplicate those analyzers.

### Relationship to issue #83

This issue is the packaging/distribution and Rust-adapter migration follow-up.
It is related to [#83](https://github.com/randlee/sc-lint/issues/83), which is
the maturin/PyO3 bindings prerequisite and implementation/design home for
native Python bindings. #83 should define the native binding/build boundary;
this issue should define the installable utility package, resource layout,
entry points, and clean-consumer acceptance tests. The two issues should share
the JSON/schema and version-compatibility contract without duplicating work.

Neither issue is implemented by creating this sc-compose inventory. The
packaging and bindings work remains to be designed and implemented in
sc-lint.

## Inventory evidence

Exactly one external issue was created: [#86](https://github.com/randlee/sc-lint/issues/86).
Its body is the exact text in the section above; `gh issue view` was used to
verify the recorded title, URL, related #83 link, evidence, and requested
clean-consumer package design.

Validation commands:

```text
sc-lint version --json
gh issue view <recorded-number> --repo randlee/sc-lint --json number,url,title,body
git diff --check
```
