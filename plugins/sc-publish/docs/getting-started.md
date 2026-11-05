# Getting started

Four steps take a repository from nothing to a published release:

1. Read this guide.
2. Write the pin and the JSON, then install the kit.
3. Add one block per destination, plus its secrets.
4. Publish.

Most destinations are one JSON block plus secrets. Two need more than the JSON:

- crates.io also needs the Cargo layout below and a committed `Cargo.lock`.
- PyPI also needs a static version in each `pyproject.toml`, and each binding crate listed as a workspace member.

Field lists: [`publish-kit-requirements.md`](../.claude/skills/publishing/ref/publish-kit-requirements.md)
and [`channel-contracts.md`](../.claude/skills/publishing/ref/channel-contracts.md).

## Files you write

1. `release/sc-publish-pin.toml`: copy `release/sc-publish-pin.toml.example` and set the 40-character kit SHA.
2. `release/sc-publish-consumer-input.json`: the only release manifest. Start from the minimum JSON below.
3. `scripts/prerelease_tag.py`: only if you opt into GitHub prereleases.

Never edit a file that `install.py` writes. That covers every copied kit file (`.github/`, `.claude/`, `.codex/`, `.cursor/`, `docs/`, `README.sc-publish.md`, the starter templates under `release/homebrew/` and `release/scoop/`) and the three rendered files (`release/publish-artifacts.toml`, `release/publish-channel-contracts.toml`, `release/publish-preflight.toml`). To change anything, edit the JSON or the pin and re-run install. A repeat `--dry-run` must exit 0.

## Install

Clone sc-publish at the pin SHA into `.sc-publish-kit/`. Add both `.sc-publish-kit/` and `.sc-publish-venv/` to `.gitignore`.

`install.py` overwrites any file of yours at a kit path without warning, for example `.github/workflows/release.yml`, `docs/getting-started.md` or anything under `docs/sprints/`. Only `README.md` is installed under another name (`README.sc-publish.md`). Move conflicting files first.

```bash
publish_python="$(python3 .sc-publish-kit/plugins/sc-publish/.github/scripts/bootstrap_sc_compose.py --venv .sc-publish-venv)"
"$publish_python" .sc-publish-kit/plugins/sc-publish/install.py \
  --input release/sc-publish-consumer-input.json .
"$publish_python" .sc-publish-kit/plugins/sc-publish/install.py \
  --dry-run --input release/sc-publish-consumer-input.json .
```

Check the manifest and the Cargo versions, then list the secrets and environments the manifest needs. `validate-manifest` alone does not check versions; Release Preflight runs `verify-version-lockstep` and fails on it.

```bash
python3 .github/scripts/release_artifacts.py validate-manifest \
  --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
python3 .github/scripts/release_artifacts.py verify-version-lockstep \
  --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
python3 .github/scripts/release_artifacts.py preflight-secret-plan \
  --manifest release/publish-artifacts.toml
```

Commit the installed files. Release workflows run the kit from the release tag, so install before you tag.

## Cargo layout

- `workspace_toml` names the root `Cargo.toml`. It needs a `[workspace]` table and `[workspace.package].version`. That is the release version. A single-crate `Cargo.toml` without `[workspace]` fails.
- List every `cargo_toml` directory literally in `[workspace].members`. A glob such as `"crates/*"` fails `validate-manifest`.
- Every `"publish": true` crate sets `version.workspace = true`. A literal version passes `validate-manifest` but fails `verify-version-lockstep`.
- Commit `Cargo.lock`. Preflight runs `cargo package --locked` and releases run `cargo publish --locked`.

```toml
[workspace]
members = ["crates/example"]

[workspace.package]
version = "0.1.0"
```

## Minimum JSON

This JSON publishes crates.io and a GitHub Release carrying the `example` binary archive. `"channels": {}` adds nothing else.

- `release_targets` must be non-empty.
- Each `release_binaries` name must be a Cargo `[[bin]]` name.
- The GitHub Release needs at least one asset: a binary, a wheel or sdist, or an npm package. A crates-only manifest (`"release_binaries": []` with no Python or npm packages) fails at `release.yml` "Collect archives", because nothing was uploaded and `artifacts/` does not exist. This is a kit gap.
- Crates publish in `publish_order`. A crate with `"publish": false` takes `"publish_order": 0`.

```json
{
  "schema_version": 1,
  "project": {
    "name": "example",
    "archive_prefix": "example",
    "description": "Example library",
    "homepage": "https://github.com/example/example",
    "license": "MIT",
    "workspace_toml": "Cargo.toml",
    "rust_toolchain": "1.94.1"
  },
  "release_targets": [
    {"target": "x86_64-unknown-linux-gnu", "os": "ubuntu-latest", "archive": "tar.gz"}
  ],
  "crates": [
    {
      "artifact": "example",
      "package": "example",
      "cargo_toml": "crates/example/Cargo.toml",
      "publish": true,
      "publish_order": 1,
      "wait_after_publish_seconds": 0
    }
  ],
  "release_binaries": [{"name": "example"}],
  "python_packages": [],
  "python_distributions": [],
  "npm_packages": [],
  "channels": {}
}
```

## One-time repository setup

- Enable immutable releases: Settings → General → Releases. `release.yml` fails closed without it, and every downstream channel is denied. Enable it before the first release. A release created mutable cannot be fixed; publish a new version instead.
- Create each GitHub environment a destination below names (Settings → Environments). Release Preflight fails when one is missing.
- Use a `develop` and `main` branch model. `release-candidate.yml` tags `origin/develop` as `release-candidate-vX.Y.Z`. `release.yml` requires that tag to be an ancestor of `origin/main`, and builds and tags `vX.Y.Z` from `origin/main`. Merge the version bump to `main` before you publish. See [`release-state-strategy.md`](../.claude/skills/publishing/ref/release-state-strategy.md).

## Destinations

Each block merges into the minimum JSON. `project` and `channels` blocks add keys. `release_targets` blocks add targets; keep the ones you already have. Every other key replaces its list.

### crates.io (always on)

- **JSON:** the `crates` list in the minimum JSON. Nothing else.
- **Templates:** none.
- **Setup:** create environment `crates-io`. Add repository secret `CARGO_REGISTRY_TOKEN`: a crates.io API token with the publish-new and publish-update scopes.
- **Confirm:** `curl -sf https://crates.io/api/v1/crates/example/0.1.0` succeeds.

### GitHub Release (always on)

- **JSON:** none. Binary archives come from `release_binaries` × `release_targets`. Wheels, sdists and npm tarballs come from their own blocks.
- **Templates:** none.
- **Setup:** immutable releases. No secret: `release.yml` uses `GITHUB_TOKEN` with `contents: write`.
- **Confirm:** `gh api repos/OWNER/REPO/releases/tags/v0.1.0 --jq '.immutable, [.assets[].name]'` prints `true` and lists every asset plus `checksums.txt`.

### PyPI and TestPyPI

One channel covers both. The `testpypi` release target uploads to TestPyPI as a rehearsal. Production PyPI publishes from `pypi-publish.yml` after the immutable Release exists.

**JSON:** each distribution needs a `python_packages` entry with the same `package` name. Use `cargo_manifest` for maturin, or `"build_system": "setuptools"` instead. `wheels` lists GitHub runner labels. A wheels-only project sets `"release_binaries": []`.

```json
{
  "python_packages": [
    {"artifact": "example-py-wheel", "package": "example-py", "manifest": "crates/example-py/pyproject.toml", "module": "example_py", "publish": "pypi"}
  ],
  "python_distributions": [
    {"name": "example-py", "source": "crates/example-py", "cargo_manifest": "crates/example-py/Cargo.toml", "module_path": "crates/example-py", "sdist": true, "wheels": ["ubuntu-latest", "macos-latest", "windows-latest"]}
  ],
  "channels": {
    "pypi": {
      "workflow": "pypi-publish.yml",
      "dispatch_inputs": {"target": "production"},
      "credential_rehearsal_inputs": {"target": "testpypi"},
      "test_repository": "testpypi",
      "production_repository": "pypi"
    }
  }
}
```

- **Templates:** none. Edit each `pyproject.toml`: `[project].name` must equal `package`, and `[project].version` must be a static `version = "X.Y.Z"` line equal to the workspace version. `dynamic = ["version"]` fails `sync-python-version` in the wheel build. List a maturin crate in `[workspace].members`.
- **Setup:** create environments `pypi` and `testpypi`. Add environment secret `PYPI_API_TOKEN` to `pypi` and `TEST_PYPI_API_TOKEN` to `testpypi`. A new project needs an account-scoped token for its first upload.
- **Confirm:** `curl -sf https://pypi.org/pypi/example-py/0.1.0/json` succeeds (`test.pypi.org` for the rehearsal); `pip install example-py==0.1.0`.

### Homebrew

**JSON:** `release_targets` must include both macOS targets and the Linux target. `assets` maps exactly `macos_arm`, `macos_intel` and `linux`. `renderer_target` must be an `ubuntu-latest` `tar.gz` target. `test_output` must appear in the output of `<test_binary> <test_command>`.

```json
{
  "project": {"renderer_archive_path": "bin/example"},
  "release_targets": [
    {"target": "x86_64-unknown-linux-gnu", "os": "ubuntu-latest", "archive": "tar.gz"},
    {"target": "aarch64-apple-darwin", "os": "macos-latest", "archive": "tar.gz"},
    {"target": "x86_64-apple-darwin", "os": "macos-latest", "archive": "tar.gz"}
  ],
  "release_binaries": [{"name": "example"}],
  "channels": {
    "homebrew": {
      "workflow": "homebrew-publish.yml",
      "dispatch_inputs": {},
      "tap_repository": "example/homebrew-tap",
      "renderer_target": "x86_64-unknown-linux-gnu",
      "formulas": [
        {"path": "Formula/example.rb", "template": "release/homebrew/formula.rb.j2", "class": "Example", "binaries": ["example"], "test_binary": "example", "test_command": "--version", "test_output": "example", "release_track": "stable"}
      ],
      "assets": [
        {"key": "macos_arm", "target": "aarch64-apple-darwin"},
        {"key": "macos_intel", "target": "x86_64-apple-darwin"},
        {"key": "linux", "target": "x86_64-unknown-linux-gnu"}
      ]
    }
  }
}
```

- **Templates:** `release/homebrew/formula.rb.j2` is a shipped starter and works unedited. To customize, copy it to a new path such as `release/homebrew/example.rb.j2`, edit the copy, and point `template` at it.
- **Setup:** create the tap repository `example/homebrew-tap` with at least one commit on its default branch. Add repository secret `HOMEBREW_TAP_TOKEN`: a token with contents write access to the tap.
- **Confirm:** the tap gets a commit `chore: bump example to v0.1.0`; `brew install example/tap/example && example --version`.

### Scoop

**JSON:** `release_targets` must include the Windows `zip` target and the Linux `tar.gz` renderer target. `binary` is the path inside the archive: `bin/<name>.exe`.

```json
{
  "project": {"renderer_archive_path": "bin/example"},
  "release_targets": [
    {"target": "x86_64-unknown-linux-gnu", "os": "ubuntu-latest", "archive": "tar.gz"},
    {"target": "x86_64-pc-windows-msvc", "os": "windows-latest", "archive": "zip"}
  ],
  "release_binaries": [{"name": "example"}],
  "channels": {
    "scoop": {
      "workflow": "scoop-publish.yml",
      "dispatch_inputs": {},
      "bucket_repository": "example/scoop-bucket",
      "manifest_path": "bucket/example.json",
      "manifest_template": "release/scoop/manifest.json.j2",
      "installer_target": "x86_64-pc-windows-msvc",
      "binary": "bin/example.exe",
      "renderer_target": "x86_64-unknown-linux-gnu"
    }
  }
}
```

- **Templates:** `release/scoop/manifest.json.j2` is a shipped starter and works unedited. To customize, use a copy at a new path, as for Homebrew.
- **Setup:** create the bucket repository `example/scoop-bucket` with at least one commit. Add repository secret `SCOOP_BUCKET_TOKEN`: a token with contents write access to the bucket.
- **Confirm:** the bucket's `bucket/example.json` shows the new version; `scoop bucket add example https://github.com/example/scoop-bucket`, then `scoop install example/example`.

### WinGet

**JSON:** `release_targets` must include the Windows `zip` target. `identifier` must match the package already in winget-pkgs.

```json
{
  "release_targets": [
    {"target": "x86_64-pc-windows-msvc", "os": "windows-latest", "archive": "zip"}
  ],
  "release_binaries": [{"name": "example"}],
  "channels": {
    "winget": {
      "workflow": "winget-publish.yml",
      "dispatch_inputs": {},
      "identifier": "Example.Example",
      "installer_target": "x86_64-pc-windows-msvc"
    }
  }
}
```

- **Templates:** none. `winget-publish.yml` derives the new manifest from the previous winget-pkgs version.
- **Setup:**
  - Fork `microsoft/winget-pkgs` into the account that owns this repository.
  - Submit the first version by hand, for example with `wingetcreate new`, and wait for it to merge. The workflow only updates an existing package.
  - Add repository secret `WINGET_GITHUB_TOKEN`: a classic token with `public_repo` scope from the fork owner.
- **Confirm:** a pull request for `Example.Example 0.1.0` opens on `microsoft/winget-pkgs`. After it merges, `winget install Example.Example`.

### npm

**JSON:** declare `npm_packages` and `channels.npm` together. `source` is a directory with a committed `package.json` and `package-lock.json`.

```json
{
  "npm_packages": [{"name": "@example/client", "source": "bindings/js"}],
  "channels": {
    "npm": {"workflow": "npm-publish.yml", "dispatch_inputs": {"dry_run": "false"}}
  }
}
```

- **Templates:** none. In `package.json`, `name` must equal the declared name, `version` must equal the release version, and the package must be public.
- **Setup:** the npm scope (user or organization) must already exist. Create environment `npm` and add environment secret `NPM_TOKEN`: an npm token allowed to publish new packages in that scope. The workflow publishes with `--access public`. Immutable releases are required.
- **Confirm:** `npm view @example/client@0.1.0 version` prints `0.1.0`.

### GitHub prerelease (`/prerelease`)

A prerelease creates a GitHub prerelease only. It never touches crates.io, PyPI, Homebrew, Scoop, WinGet or npm. It needs `release_binaries` or Python wheels.

**JSON:** keep `tag_prefix` as `prerelease/v`, because `prerelease-archive.yml` triggers only on `prerelease/v*.*.*` tags. Every name in `binaries` must be a release binary. `verify` must print the installed version. Placeholders: `{version}`, `{stage_dir}`, `{python}`, `{selector_dir}`, `{wheels}`.

```json
{
  "release_binaries": [{"name": "example"}],
  "prerelease": {
    "tag_prefix": "prerelease/v",
    "tag_script": "scripts/prerelease_tag.py",
    "install_root": "~/.example-builds",
    "binaries": ["example"],
    "selector_dir": {
      "darwin": "~/.example-builds/selectors/darwin",
      "linux": "~/.example-builds/selectors/linux",
      "windows": "%LOCALAPPDATA%\\example\\prerelease-selectors"
    },
    "post_install": "{selector_dir}/example --version",
    "verify": "{selector_dir}/example --version",
    "protected_branches": ["develop", "main"]
  }
}
```

A wheels-only project sets `"release_binaries": []` and `"binaries": []`, and installs its wheels instead:

```json
{
  "post_install": "{python} -m pip install --force-reinstall {wheels}",
  "verify": "{python} -m pip show example-py"
}
```

**Tag script:** you write `scripts/prerelease_tag.py`. The kit does not choose the version or create the tag. With `--dry-run`, the script prints its plan and changes nothing. A real run:

1. picks a version never published, patch-bumping the workspace version if needed;
2. commits the bump, so the tagged commit's workspace version equals the tag version;
3. pushes the branch, then creates and pushes `prerelease/vX.Y.Z`;
4. prints this line, which the kit parses:

```text
created and pushed prerelease/v0.1.1 from <commit>
```

- **Templates:** none.
- **Setup:** no secret. Run from a clean, non-protected branch with an authenticated `gh`. Do not pass a version to `/prerelease`.
- **Confirm:** `/prerelease --publish` prints the Release URL once every asset and checksum verifies. `/prerelease --list` shows it, and `/prerelease --install` installs it.

## Publish

Publish with `/cursor-publish` (`.cursor/agents/publisher.md`) or the ATM `publisher` teammate. In order, it:

1. cuts `release-candidate-vX.Y.Z` from `develop` with `release-candidate.yml`;
2. merges the release, including the version bump, to `main`;
3. runs Release Preflight on that `main` commit;
4. runs `release.yml`: the TestPyPI rehearsal, then production (crates.io, the `vX.Y.Z` tag and the immutable GitHub Release);
5. dispatches each declared channel workflow.

Each leg retries by tag on its own. Branch rules: [`release-state-strategy.md`](../.claude/skills/publishing/ref/release-state-strategy.md).

## Checklist

Every destination is a block in `release/sc-publish-consumer-input.json`. Re-run install and `--dry-run` after each change. crates.io and PyPI also need the files in the last column.

| Destination | JSON keys you add | Secrets | Environments | Other repositories or files |
| --- | --- | --- | --- | --- |
| crates.io | `crates` | `CARGO_REGISTRY_TOKEN` (repository) | `crates-io` | root `Cargo.toml` (members, `[workspace.package].version`), each crate `Cargo.toml` (`version.workspace = true`), committed `Cargo.lock` |
| GitHub Release | none | none (`GITHUB_TOKEN`) | none | none |
| PyPI and TestPyPI | `python_packages`, `python_distributions`, `channels.pypi` | `PYPI_API_TOKEN`, `TEST_PYPI_API_TOKEN` (environment) | `pypi`, `testpypi` | each `pyproject.toml` (static `version`); binding crate in `[workspace].members` |
| Homebrew | `release_targets`, `release_binaries`, `project.renderer_archive_path`, `channels.homebrew` | `HOMEBREW_TAP_TOKEN` (repository) | none | tap |
| Scoop | `release_targets`, `release_binaries`, `project.renderer_archive_path`, `channels.scoop` | `SCOOP_BUCKET_TOKEN` (repository) | none | bucket |
| WinGet | `release_targets`, `release_binaries`, `channels.winget` | `WINGET_GITHUB_TOKEN` (repository) | none | `winget-pkgs` fork; first version merged by hand |
| npm | `npm_packages`, `channels.npm` | `NPM_TOKEN` (environment) | `npm` | existing npm scope; `package.json` and `package-lock.json` |
| `/prerelease` | `prerelease`, plus `scripts/prerelease_tag.py` | none | none | none |

Every destination except `/prerelease` needs immutable releases enabled and the `develop`/`main` release-candidate flow. Edit a template only as a copy at a new path.
