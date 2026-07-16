---
id: C4
title: Python Release Train And Packaging Hardening
status: planned
branch: plan/maturin-bindings-implementation-plan
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/maturin-bindings-implementation-plan
---

# Sprint C4 — Python Release Train And Packaging Hardening

## Goal

Extend the main `sc-compose` release train so the Python adapter ships as a
first-class release channel after the adapter surface and cross-platform wheel
builds already exist.

This sprint is intentionally separate from C1 because it depends on live
release credentials and release-pipeline ownership that are not required to
prove the binding scaffold itself.

## Hard Dependencies

- [docs/phase-C/sprint-C1-maturin-bindings.md](./sprint-C1-maturin-bindings.md)
- [docs/architecture.md](../architecture.md)
- [docs/publishing.md](../publishing.md)
- [docs/publishing-agent.md](../publishing-agent.md)

## Exact Targets

- `.github/workflows/release.yml`
- `release/publish-artifacts.toml`
- `docs/publishing.md`
- `docs/publishing-agent.md`
- `docs/phase-C/sprint-C4-python-release-train.md`

## Deliverables

- `D1`
  - add Python wheel and sdist build steps to the main release workflow
- `D2`
  - add PyPI publish wiring to the main release workflow
- `D3`
  - add Python artifact metadata to `release/publish-artifacts.toml`
- `D4`
  - document the new `PYPI_API_TOKEN` requirement
- `D5`
  - update release operator docs so PyPI is a required verification channel
- `D6`
  - attach built wheels and sdist artifacts to GitHub Releases

## Release Workflow Sample

The release workflow additions should follow this shape:

```yaml
  build-python-wheels:
    needs: gate-and-tag
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        python-version: ["3.11"]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ needs.gate-and-tag.outputs.release_tag }}
      - uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python-version }}
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.94.1
      - name: Install maturin
        run: python -m pip install maturin==1.9.4
      - name: Build wheels
        run: maturin build --release --manifest-path bindings/python/Cargo.toml --out dist
      - name: Build sdist
        run: maturin sdist --manifest-path bindings/python/Cargo.toml --out dist
      - uses: actions/upload-artifact@v4
        with:
          name: python-wheels-${{ matrix.os }}
          path: dist/*

  publish-pypi:
    needs: [gate-and-tag, build-python-wheels]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: python-wheels-*
          merge-multiple: true
          path: dist
      - uses: actions/setup-python@v5
        with:
          python-version: "3.11"
      - name: Install maturin
        run: python -m pip install maturin==1.9.4
      - name: Publish wheels and sdist to PyPI
        env:
          MATURIN_PYPI_TOKEN: ${{ secrets.PYPI_API_TOKEN }}
          MATURIN_NON_INTERACTIVE: "1"
        run: maturin upload --non-interactive dist/*.whl dist/*.tar.gz
```

This sample is normative for C4 in four ways:

- PyPI credentials enter the publish step only through
  `MATURIN_PYPI_TOKEN=${{ secrets.PYPI_API_TOKEN }}`
- release-tagged wheel builds stay on the main release workflow rather than a
  separate ad hoc workflow
- wheels and the sdist are both produced into the same `dist/` artifact set
  before upload
- the PyPI publish job downloads only the `python-wheels-*` artifacts and
  uploads only `dist/*.whl` and `dist/*.tar.gz`

## GitHub Release Attachment Sample

The existing `release` job also needs an explicit update so wheel and sdist
artifacts are present before the GitHub Release is created and are not dropped
by the collection filter:

```diff
-  release:
-    needs: [gate-and-tag, build, publish]
+  release:
+    needs: [gate-and-tag, build, publish, build-python-wheels, publish-pypi]
     runs-on: ubuntu-latest
     steps:
       - uses: actions/checkout@v4
         with:
           ref: ${{ needs.gate-and-tag.outputs.release_tag }}

       - name: Download all artifacts
         uses: actions/download-artifact@v4
         with:
           path: artifacts

       - name: Collect archives
         run: |
           mkdir -p release
-          find artifacts -type f \( -name '*.tar.gz' -o -name '*.zip' \) -exec mv {} release/ \;
+          find artifacts -type f \( -name '*.tar.gz' -o -name '*.zip' -o -name '*.whl' \) -exec mv {} release/ \;
           ls -la release/

       - name: Generate checksums
         working-directory: release
         run: |
           files=()
           for pattern in *.tar.gz *.zip *.whl; do
             for file in $pattern; do
               [[ -e "$file" ]] || continue
               files+=("$file")
             done
           done
           sha256sum "${files[@]}" > checksums.txt
```

This sample is normative for `D6` in three ways:

- the `release` job must wait on `build-python-wheels` and `publish-pypi`
  before collecting artifacts and creating the GitHub Release
- the collection filter must include `*.whl` so wheel files land in `release/`
  beside the existing binary archives and the Python sdist `*.tar.gz`
- the checksum generation step must include `*.whl` so every wheel attached to
  the GitHub Release also appears in `checksums.txt`

## Release Artifact Manifest Sample

The `release/publish-artifacts.toml` additions should follow this shape:

```toml
schema_version = 1

[[python_packages]]
artifact = "sc-compose-python"
package = "sc-compose"
manifest = "bindings/python/pyproject.toml"
module = "sc_compose"
publish = "pypi"

[[python_distributions]]
name = "sc-compose"
source = "bindings/python"
sdist = true
wheels = ["ubuntu-latest", "macos-latest", "windows-latest"]
```

This sample is normative for C4 in two ways:

- the Python package entry is a first-class release artifact beside the
  existing crates and release binaries
- the manifest points at `bindings/python/pyproject.toml`, not any crate under
  `crates/`

## Acceptance Criteria

- `AC1` for `D1`
  - `.github/workflows/release.yml` defines a named Python build path for
    tagged releases that builds wheel artifacts on macOS, Linux, and Windows
  - `.github/workflows/release.yml` defines one Python source distribution path
- `AC2` for `D2`
  - `.github/workflows/release.yml` defines a named `publish-pypi` job
  - PyPI publication uses `MATURIN_PYPI_TOKEN` sourced from
    `PYPI_API_TOKEN`
- `AC3` for `D3`
  - `release/publish-artifacts.toml` documents Python release artifacts with a
    concrete entry for the `sc-compose` Python package
- `AC4` for `D4`
  - `docs/publishing-agent.md` names `PYPI_API_TOKEN` as a required secret
- `AC5` for `D5`
  - `docs/publishing.md` and `docs/publishing-agent.md` include PyPI
    verification and release-operator steps
- `AC6` for `D6`
  - GitHub Releases attach Python wheels and the sdist beside existing release
    archives

## Required Validation

- `cargo test --workspace`
- `python3 -c "import pathlib, yaml; yaml.safe_load(pathlib.Path('.github/workflows/release.yml').read_text())"`
- `python3 - <<'PY'
import pathlib
import tomllib

tomllib.loads(pathlib.Path('release/publish-artifacts.toml').read_text())
PY`
- `python3 - <<'PY'
import pathlib
import tomllib

data = tomllib.loads(pathlib.Path('release/publish-artifacts.toml').read_text())
packages = data.get('python_packages', [])
distributions = data.get('python_distributions', [])

assert any(
    entry.get('package') == 'sc-compose'
    and entry.get('manifest') == 'bindings/python/pyproject.toml'
    and entry.get('module') == 'sc_compose'
    and entry.get('publish') == 'pypi'
    for entry in packages
), 'missing or invalid [[python_packages]] entry for sc-compose'

assert any(
    entry.get('name') == 'sc-compose'
    and entry.get('source') == 'bindings/python'
    and entry.get('sdist') is True
    and entry.get('wheels') == ['ubuntu-latest', 'macos-latest', 'windows-latest']
    for entry in distributions
), 'missing or invalid [[python_distributions]] entry for sc-compose'
PY`
- `python3 - <<'PY'
import pathlib

text = pathlib.Path('.github/workflows/release.yml').read_text()

assert 'needs: [gate-and-tag, build, publish, build-python-wheels, publish-pypi]' in text, \
    'release job must depend on build-python-wheels and publish-pypi'
assert \"-name '*.whl'\" in text, \
    'release artifact collection must include *.whl files'
assert 'pattern: python-wheels-*' in text, \
    'publish-pypi artifact download must be scoped to python-wheels-*'
assert 'merge-multiple: true' in text, \
    'publish-pypi artifact download must flatten scoped Python artifacts into dist/'
assert 'maturin upload --non-interactive dist/*.whl dist/*.tar.gz' in text, \
    'publish-pypi must upload only wheel and sdist files'
assert 'for pattern in *.tar.gz *.zip *.whl; do' in text, \
    'release checksum generation must include wheel files'
PY`
- `gh workflow view Release --yaml >/dev/null`

Validation-to-AC mapping:

- `cargo test --workspace`
  - verifies `AC1` through `AC6` do not regress the workspace after the Python
    release-train wiring lands
- `python3 -c "import pathlib, yaml; ..."`
  - verifies `AC1`, `AC2`, and `AC6` by ensuring the edited workflow YAML still
    parses
- `python3 - <<'PY' ...`
  - verifies the base TOML syntax needed for `AC3`
- the second `python3 - <<'PY' ...`
  - verifies `AC3` by asserting the concrete `[[python_packages]]` and
    `[[python_distributions]]` entries exist with the expected keys and values
- the third `python3 - <<'PY' ...`
  - verifies `AC2` and `AC6` by asserting the PyPI publish job downloads only
    scoped Python artifacts, uploads only wheel and sdist files, the `release`
    job depends on the Python artifact jobs, the artifact-collection filter
    includes `*.whl`, and checksum generation includes wheel files
- `gh workflow view Release --yaml >/dev/null`
  - verifies the named workflow remains addressable as `Release`, which is the
    workflow C4 edits directly
