---
id: R.3
title: Beads Python Bindings
status: complete
branch: sprint/r-3-beads-python-bindings
target: integrate/phase-r
depends_on: [R.1, R.2]
---

# Sprint R.3 — Beads Python Bindings

## Goal

Deliver a dedicated Maturin/PyO3 package so Python extensions call the same
`sc-composer-beads` operations, request validation, authorization guard, and
receipts as Rust and the CLI.

R.3 may be developed in parallel with R.2 after R.1, but it cannot close until
R.2's CLI JSON contract is available for the required cross-surface conformance
fixture.

## Exact targets

- `Cargo.toml`
- `bindings/sc-composer-beads-python/Cargo.toml`
- `bindings/sc-composer-beads-python/pyproject.toml`
- `bindings/sc-composer-beads-python/src/lib.rs`
- `bindings/sc-composer-beads-python/python/sc_composer_beads/{__init__.py,_native.pyi,py.typed}`
- `bindings/sc-composer-beads-python/tests/{test_smoke,test_contract}.py`
- `crates/sc-composer-beads/tests/fixtures/beads/`
- `.github/workflows/ci.yml`
- `.github/scripts/release_artifacts.py` and its tests
- `release/publish-artifacts.toml` (including matching `crates`,
  `python_packages`, and `[[python_distributions]]` entries for the new
  package)
- `release/sc-publish-install.json` (including matching `crates`,
  `python_packages`, and `python_distributions` entries for the new package)

## Public Python contract

The distribution is `sc-composer-beads`; its import package is
`sc_composer_beads`. It offers a faithful, typed representation of the R.1
request/receipt contract plus the library operation and convenience methods:

```python
def execute(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def render(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def validate(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def preview_pour(request: BeadComposeRequest) -> BeadComposeReceipt: ...
def pour(request: BeadComposeRequest) -> BeadComposeReceipt: ...
```

`pour()` requires the same explicit enum/sentinel in the request. The adapter
does not shell out to `sc-compose`, does not accept arbitrary commands, and
does not expose an authorization bypass. It releases the Python GIL while the
Rust crate waits for `bd`.

## Deliverables

1. Add the independent workspace/member package with `cdylib` and `rlib`,
   PyO3 `0.29`, Maturin `>=1.9.4,<2.0`, Python `>=3.11`, typed package files,
   and a dependency only on `sc-composer-beads` plus adapter dependencies.
2. Convert Python dictionaries/lists/scalars to and from the versioned Rust
   request/receipt types without reimplementing rendering or process logic.
   Conversion failures map to a crate-owned Python exception with the stable
   Rust error code and stage, never a raw Rust panic.
3. Add installed-wheel smoke tests and contract tests. They load the canonical
   `crates/sc-composer-beads/tests/fixtures/beads/` fixture directly and prove
   it yields equivalent Rust, CLI JSON, and Python receipts; normalize only
   documented absolute-path differences. R.1 owns updates to that fixture when
   the shared contract changes.
4. Extend CI to build and install wheels on Linux, macOS, and Windows, execute
   the pinned-Beads integration fixture through the wheel, and run ordinary
   workspace tests without requiring an extension-module feature for `cargo
   test`.
5. Add this wheel as a separately named release artifact in both
   `release/publish-artifacts.toml` and `release/sc-publish-install.json`.
   Each manifest must contain a `crates` entry for `sc-composer-beads`, a
   `python_packages` entry for `sc-composer-beads-python`, and a matching
   `[[python_distributions]]`/`python_distributions` entry consumed by the
   release wheel and sdist matrices. The TOML entry must use
   `name = "sc-composer-beads"`, `source = "bindings/sc-composer-beads-python"`,
   `cargo_manifest = "bindings/sc-composer-beads-python/Cargo.toml"`,
   `module_path = "bindings/sc-composer-beads-python/python/sc_composer_beads"`,
   `sdist = true`, and `wheels = ["ubuntu-latest", "macos-latest",
   "windows-latest"]`; the JSON entry must carry the matching field values.
   Wire all three entries into the existing version verification path;
   `verify-version-lockstep` alone is not a substitute for matrix coverage.
   Package publication itself remains subject to the existing explicit release
   authorization workflow.

## Acceptance criteria

- [x] `import sc_composer_beads` works from an installed wheel on all three CI
      platforms, and `.pyi`/`py.typed` ship in the wheel and sdist.
- [x] Python `validate` and `preview_pour` produce the same stage outcomes and
      Beads argv evidence as the Rust library/CLI fixture, using the
      `BeadStageReceipt`, `BeadOutcome`, and `BeadComposeError` definitions from
      ADR-0021 without Python-local variants.
- [x] Python cannot bypass `PourAuthorization::CreatePersistentBeads`; tests
      prove refusal occurs before subprocess execution.
- [x] The binding package has no dependency on `sc-compose`, the existing
      `bindings/python` package, ATM, or Beads source/database code.
- [x] Release metadata validates the new package's version lockstep without
      changing the existing `sc-compose` Python package identity. The R.3
      closeout must run `verify-version-lockstep` against the release manifest
      and workspace manifest.
- [x] Both release manifests contain a matching
      `[[python_distributions]]`/`python_distributions` entry named
      `sc-composer-beads`; the Python wheel and sdist matrix commands each
      emit that distribution name.

## Required validation

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
maturin build --manifest-path bindings/sc-composer-beads-python/Cargo.toml --out dist
python3 -m pytest -q bindings/sc-composer-beads-python/tests
python3 .github/scripts/release_artifacts.py validate-manifest --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
python3 .github/scripts/release_artifacts.py verify-version-lockstep --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
python3 .github/scripts/release_artifacts.py python-wheel-matrix --manifest release/publish-artifacts.toml | python3 -c 'import json,sys; names={entry["name"] for entry in json.load(sys.stdin)["include"]}; raise SystemExit("sc-composer-beads missing from wheel matrix") if "sc-composer-beads" not in names else None'
python3 .github/scripts/release_artifacts.py python-sdist-matrix --manifest release/publish-artifacts.toml | python3 -c 'import json,sys; names={entry["name"] for entry in json.load(sys.stdin)["include"]}; raise SystemExit("sc-composer-beads missing from sdist matrix") if "sc-composer-beads" not in names else None'
```

Also require `git diff --check`.

## Validation evidence

Validated on 2026-08-26 across the R.3 closing commit chain:

- `285c23c` closed the initial QA findings; `0fc1d2b` restored typed JSON
  conversion errors; and `9e1bfa2` completed the schema-valid boundary record.
- CI run [32947283597](https://github.com/randlee/sc-compose/actions/runs/32947283597)
  passed all 17 checks for `9e1bfa2`, including the installed wheel matrix on
  Linux, macOS, and Windows.
- Installed-wheel smoke and contract tests cover the canonical R.1/R.2
  fixture, Python-to-CLI stage receipts, authorization refusal before process
  execution, the pinned-`bd` fixture, and wheel/sdist typing-marker contents.
- `cargo fmt --all --check`, workspace clippy, full workspace tests, Maturin
  wheel/sdist build/install tests, release-manifest validation, version
  lockstep, wheel and sdist matrix checks, and `git diff --check` passed
  locally.

## Out of scope

Combining this package with `sc-compose`, adding a Go binding, publishing a
wheel without the normal release gate, or a non-dry-run Beads creation test is
not part of R.3.
