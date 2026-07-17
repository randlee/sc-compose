---
id: C.3
title: Python Release Train And Packaging Hardening
status: complete
branch: sprint/c-3-python-release-train
worktree: ../sc-compose-worktrees/sprint/c-3-python-release-train
---

# Sprint C.3 — Python Release Train And Packaging Hardening

## Goal

Extend the main `sc-compose` release train so the Python adapter ships as a
first-class release channel after the adapter surface, the full Python API
surface, and cross-platform wheel builds already exist.

This sprint is intentionally separate from C.1 and C.2 because it depends on
live release credentials and release-pipeline ownership that are not required
to prove the binding scaffold or wrapper surface themselves.

This sprint plans and wires the production-ready release path, but it edits
the live `.github/workflows/release.yml` and is gated by the
[Explicit Non-Closure](#explicit-non-closure) rule below rather than being
non-executable: the workflow changes are real and CI-parseable, but the
release path is not treated as closed for production until it has been
exercised through a staged execution.

## Hard Dependencies

- [docs/phase-C/README.md](./README.md)
- [docs/phase-C/maturin-bindings-investigation.md](./maturin-bindings-investigation.md)
- [docs/phase-C/sprint-c-2-python-api-surface.md](./sprint-c-2-python-api-surface.md)
- [docs/architecture.md](../architecture.md)
- [docs/publishing.md](../publishing.md)
- [docs/publishing-agent.md](../publishing-agent.md)

## Exact Targets

- `.github/workflows/release.yml`
- `release/publish-artifacts.toml`
- `scripts/release_artifacts.py`
- `docs/publishing.md`
- `docs/publishing-agent.md`
- `docs/phase-C/sprint-c-3-python-release-train.md`

## Deliverables

- `D1`
  - add Python wheel and sdist build steps to the main release workflow
- `D2`
  - add PyPI publish wiring to the main release workflow
- `D3`
  - add Python artifact metadata to `release/publish-artifacts.toml`
- `D4`
  - document the new `PYPI_API_TOKEN` requirement and protected environment
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
    runs-on: ${{ matrix.os }}
    steps:
      - uses: ./.github/actions/setup-python-release-build
        with:
          release_tag: ${{ needs.gate-and-tag.outputs.release_tag }}
          release_version: ${{ needs.gate-and-tag.outputs.release_version }}
      - name: Build wheels
        run: maturin build --release --manifest-path bindings/python/Cargo.toml --out dist
      - uses: actions/upload-artifact@v4
        with:
          name: python-wheels-${{ matrix.os }}
          path: dist/*.whl

  build-python-sdist:
    needs: gate-and-tag
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/actions/setup-python-release-build
        with:
          release_tag: ${{ needs.gate-and-tag.outputs.release_tag }}
          release_version: ${{ needs.gate-and-tag.outputs.release_version }}
      - name: Build sdist
        run: maturin sdist --manifest-path bindings/python/Cargo.toml --out dist
      - uses: actions/upload-artifact@v4
        with:
          name: python-sdist
          path: dist/*.tar.gz

  publish-pypi:
    needs: [gate-and-tag, build-python-wheels, build-python-sdist]
    runs-on: ubuntu-latest
    environment: pypi
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: python-wheels-*
          merge-multiple: true
          path: dist
      - uses: actions/download-artifact@v4
        with:
          name: python-sdist
          path: dist
      - uses: actions/setup-python@v5
        with:
          python-version: "3.11"
      - name: Install maturin
        run: python -m pip install maturin==1.9.4
      - name: Assert unique sdist
        shell: bash
        run: |
          set -euo pipefail
          shopt -s nullglob
          sdists=(dist/*.tar.gz)
          if [[ "${#sdists[@]}" -ne 1 ]]; then
            echo "expected exactly one sdist, found ${#sdists[@]}" >&2
            exit 1
          fi
      - name: Publish wheels and sdist to PyPI
        env:
          MATURIN_PYPI_TOKEN: ${{ secrets.PYPI_API_TOKEN }}
          MATURIN_NON_INTERACTIVE: "1"
        run: maturin upload --non-interactive dist/*.whl dist/*.tar.gz
```

This sample is normative for C.3 in six ways:

- PyPI credentials enter the publish step only through
  `MATURIN_PYPI_TOKEN=${{ secrets.PYPI_API_TOKEN }}`
- the Python package version is synchronized from `Cargo.toml` immediately
  before both wheel and sdist builds, then re-verified against the release tag
- release-tagged wheel builds stay on the main release workflow rather than a
  separate ad hoc workflow
- wheel builds produce only wheel artifacts, while the sdist is built exactly
  once in a dedicated `build-python-sdist` job
- the PyPI publish job must run inside the protected GitHub Actions `pypi`
  environment
- the PyPI publish job downloads only the `python-wheels-*` artifacts plus the
  single `python-sdist` artifact, asserts there is exactly one `dist/*.tar.gz`,
  and uploads only `dist/*.whl` and `dist/*.tar.gz`

## GitHub Release Attachment Sample

The existing `release` job also needs an explicit update so wheel and sdist
artifacts are present before the GitHub Release is created and are not dropped
by the collection filter:

```diff
-  release:
-    needs: [gate-and-tag, build, publish]
+  release:
+    needs: [gate-and-tag, build, publish, build-python-wheels, build-python-sdist, publish-pypi]
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
+          find artifacts -type f \( -name '*.tar.gz' -o -name '*.zip' \) -exec mv {} release/ \;
+          find artifacts -type f -name '*.whl' -exec mv {} release/ \;
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

This sample is normative for `D6` in four ways:

- the `release` job must wait on `build-python-wheels`, `build-python-sdist`,
  and `publish-pypi` before collecting artifacts and creating the GitHub
  Release
- the release collector must preserve the pre-existing binary-archive sweep for
  `*.tar.gz` and `*.zip`, and add `*.whl` without regressing the tarball path
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

For C.3 planning, these `[[python_packages]]` and `[[python_distributions]]`
entries are descriptive release-manifest records, not yet a live workflow
input. Their authority comes from cross-check validation enforced by
`scripts/release_artifacts.py validate-manifest` plus the explicit workflow
assertions below, so the manifest and YAML cannot silently drift while the
release workflow still uses direct YAML wiring.

This sample is normative for C.3 in three ways:

- the Python package entry is a first-class release artifact beside the
  existing crates and release binaries
- the manifest points at `bindings/python/pyproject.toml`, not any crate under
  `crates/`
- the Python release manifest remains descriptive-only until a later sprint
  promotes it into a direct workflow input, and the validation suite must
  cross-check both sources until that promotion happens

## Acceptance Criteria

- `AC1` for `D1`
  - `.github/workflows/release.yml` defines a named Python build path for
    tagged releases that builds wheel artifacts on macOS, Linux, and Windows
  - `.github/workflows/release.yml` defines one Python source-distribution path
    that builds exactly one sdist
- `AC2` for `D2`
  - `.github/workflows/release.yml` defines a named `publish-pypi` job
  - PyPI publication uses `MATURIN_PYPI_TOKEN` sourced from
    `PYPI_API_TOKEN`
  - `.github/workflows/release.yml` defines a dedicated `build-python-sdist`
    job and `publish-pypi` asserts exactly one sdist exists before upload
  - the release workflow syncs and verifies the Python package version before
    wheel and sdist builds
- `AC3` for `D3`
  - `release/publish-artifacts.toml` documents Python release artifacts with a
    concrete entry for the `sc-compose` Python package
  - `scripts/release_artifacts.py validate-manifest` cross-checks the Python
    manifest entries against the planned package paths and wheel metadata
- `AC4` for `D4`
  - `docs/publishing-agent.md` names `PYPI_API_TOKEN` as a required secret in
    the protected GitHub Actions `pypi` environment
- `AC5` for `D5`
  - `docs/publishing.md` and `docs/publishing-agent.md` include PyPI
    verification, TestPyPI or workflow rehearsal, and release-operator steps
- `AC6` for `D6`
  - GitHub Releases attach Python wheels and the sdist beside existing release
    archives

## Explicit Non-Closure

Static YAML, TOML, and grep-style validation does not prove end-to-end Python
release correctness. Until the planned workflow is exercised through a staged
release, these acceptance criteria remain design-closed only.

Before C.3 may be treated as closed for a real production release, one staged
execution must pass on either TestPyPI or a `workflow_dispatch` rehearsal path
that builds wheels, builds exactly one sdist, publishes to a non-production
destination, and confirms the GitHub Release attachment set.

## Required Validation

- `cargo test --workspace`
- `python3 scripts/release_artifacts.py validate-manifest --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
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

assert 'needs: [gate-and-tag, build, publish, build-python-wheels, build-python-sdist, publish-pypi]' in text, \
    'release job must depend on build-python-wheels, build-python-sdist, and publish-pypi'
assert 'name: python-sdist' in text, \
    'release workflow must define a dedicated python-sdist artifact'
assert 'environment: pypi' in text, \
    'publish-pypi must run in the protected pypi environment'
assert "find artifacts -type f \\( -name '*.tar.gz' -o -name '*.zip' \\) -exec mv {} release/ \\;" in text, \
    'release artifact collection must preserve the pre-existing tar.gz and zip sweep'
assert "find artifacts -type f -name '*.whl' -exec mv {} release/ \\;" in text, \
    'release artifact collection must include a dedicated wheel sweep without a redundant zip match'
assert 'pattern: python-wheels-*' in text, \
    'publish-pypi artifact download must be scoped to python-wheels-*'
assert 'expected exactly one sdist' in text, \
    'publish-pypi must assert that exactly one sdist exists before upload'
assert 'maturin upload --non-interactive dist/*.whl dist/*.tar.gz' in text, \
    'publish-pypi must upload only wheel and sdist files'
assert 'for pattern in *.tar.gz *.zip *.whl; do' in text, \
    'release checksum generation must include wheel files'
assert 'uses: ./.github/actions/setup-python-release-build' in text, \
    'release workflow must invoke the shared Python release-build composite action'
action_text = pathlib.Path('.github/actions/setup-python-release-build/action.yml').read_text()
assert 'verify-python-version' in action_text and 'sync-python-version' in action_text, \
    'shared Python release-build action must sync and verify the Python package version before wheel and sdist builds'
PY`
- `gh workflow view Release --yaml >/dev/null`

Validation-to-AC mapping:

- `cargo test --workspace`
  - verifies `AC1` through `AC6` do not regress the workspace after the Python
    release-train wiring lands
- `python3 scripts/release_artifacts.py validate-manifest ...`
  - verifies `AC3` by checking the Python manifest records structurally and
    cross-checking them against the planned package paths
- `python3 -c "import pathlib, yaml; ..."`
  - verifies `AC1`, `AC2`, and `AC6` by ensuring the edited workflow YAML still
    parses
- `python3 - <<'PY' ...`
  - verifies the base TOML syntax needed for `AC3`
- the second `python3 - <<'PY' ...`
  - verifies `AC3` by asserting the concrete `[[python_packages]]` and
    `[[python_distributions]]` entries exist with the expected keys and values
- the third `python3 - <<'PY' ...`
  - verifies `AC1`, `AC2`, and `AC6` by asserting the PyPI publish job
    downloads only scoped Python artifacts plus the dedicated sdist artifact,
    asserts exactly one sdist exists, uploads only wheel and sdist files, the
    `release` job depends on the Python artifact jobs, the artifact-collection
    filter includes `*.whl`, the sdist collector is isolated, and checksum
    generation includes wheel files
- `gh workflow view Release --yaml >/dev/null`
  - verifies the named workflow remains addressable as `Release`, which is the
    workflow C.3 edits directly
