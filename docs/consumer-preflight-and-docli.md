# Consumer publish preflight and optional sc-docli

Release Preflight (`release-preflight.yml`) is the kit's fail-closed gate before
any release dispatch. Kit-owned checks cover credentials, registry state, manifest
validation, and (when declared) Cargo packaging.

Repositories also need **repository-specific** checks — for example crates.io README
metadata, checksum contracts, or generated CLI documentation. Those must not be
hand-wired into vendored workflow YAML (sync would erase them). Instead, declare
them in the consumer input JSON and let the kit render and execute them through a
single extension point.

## Extension point (required pattern)

1. **`preflight_checks`** (optional array in consumer input JSON) — each entry:
   `id`, `description`, and `run` (shell command, usually `bash scripts/...` under
   the repository root).
2. **`docli`** (optional object) — see below.
3. **Rendered manifest** — `release/publish-preflight.toml` (third rendered file
   alongside `publish-artifacts.toml` and `publish-channel-contracts.toml`).
4. **Kit runner** — `.github/scripts/run-consumer-preflight.sh`, invoked from
   `release-preflight.yml` as step `consumer_preflight`, recorded in the deny
   summary as `consumer-preflight`.

The runner is a no-op when the manifest is missing or declares no `[[checks]]`.

## Optional sc-docli for customer-facing CLIs

[sc-docli](https://crates.io/crates/sc-docli) generates committed CLI reference
HTML from a JSON model (typically produced by a small `gen-cli-model` binary).
That documentation ships with the product; it must not drift at release time.

**Strong recommendation (not mandatory):** every consumer that ships a
**customer-facing CLI** SHOULD enable docli in consumer input JSON, pin sc-docli in
a repository-owned file such as `release/docli-pin.toml`, and implement
`scripts/generate-cli-docs.sh` to regenerate the model and HTML.

When `"docli": { "enabled": true, ... }` is set, the kit:

- Adds a `[docli]` table and a `docli-cli-html` `[[checks]]` entry to
  `release/publish-preflight.toml`.
- Installs `.github/scripts/release-preflight-docli.sh`, which runs your
  `generate_script` and fails on `git diff` / dirty paths in `drift_paths`.

**Do not run docli on every pull-request CI job** unless you have a separate reason
to do so. Regenerating CLI HTML is release-shaped work: run it in **Release
Preflight** so sprint CI stays fast and release still fails closed when docs drift.
Remove or avoid a standing PR `cli-docs` job when you adopt this pattern.

Example consumer input fragment:

```json
"docli": {
  "enabled": true,
  "generate_script": "scripts/generate-cli-docs.sh",
  "drift_paths": [
    "fixtures/cli/my-cli-model.json",
    "site/cli/"
  ],
  "description": "CLI reference HTML matches usage_message() and shipped extensions"
},
"preflight_checks": [
  {
    "id": "crates-readme-metadata",
    "description": "Published crates include README and docs.rs metadata",
    "run": "bash scripts/check-crates-publish-manifest.sh"
  }
]
```

Pin sc-docli separately (not rendered by the kit), for example:

```toml
# release/docli-pin.toml
sc_docli_version = "0.2.0"
```

## Consumer-owned evidence checks

A repository can declare a verifier using the existing fields:

```json
{"preflight_checks": [{"id": "verify-matrix", "description": "Verify release matrix evidence", "run": "python3 scripts/verify-matrix-evidence.py release/evidence.json"}]}
```

The verifier should read the candidate commit with `git rev-parse HEAD` and
compare it and the SHA-256 hashes of the exact artifacts to be published
against its evidence and trusted release inputs. File existence alone is not
verification. The consumer owns obtaining the artifact hashes and the verifier;
preflight should not rerun the matrix.

## Consumer-owned reusable workflows (optional)

A shell check runs inside the single preflight job, so it cannot schedule jobs
on other runners. To run an existing reusable workflow of your own (for
example an install matrix across hardware runners) as part of Release
Preflight, declare it in consumer input JSON:

```json
"preflight_workflows": [
  {
    "id": "install-matrix",
    "description": "Install matrix on every release platform",
    "uses": "./.github/workflows/install-matrix.yml",
    "with": {"profile": "release"}
  }
]
```

- `id`: lowercase letters, digits, `_` or `-`; unique. The call job is
  `consumer_workflow_<id>`.
- `uses`: any reusable-workflow reference GitHub accepts in `jobs.<id>.uses`.
- `with` (optional): extra inputs, each a string, boolean or integer.

The installer appends to the installed `release-preflight.yml`, after the kit's
`preflight` job:

- one job per entry that calls the workflow with `version` (the dispatched
  version input) and `candidate_sha` (`github.sha`, the commit preflight
  checks out) plus your `with` inputs. The called workflow must declare
  `version` and `candidate_sha` as `workflow_call` string inputs;
- a `consumer_workflows` job that writes each call's result to the run summary
  and fails unless every call succeeded, so the preflight run fails.

The calls run alongside the kit's `preflight` job, inherit the workflow's
`read-all` token and receive no secrets. Re-run the installer after changing
the declaration; `--dry-run` reports drift as for any kit file. Without
`preflight_workflows` (or with an empty list) the installed workflow is the kit
file byte for byte.

## Consumer-declared docli freshness check

The existing extension point can also run a repository-owned script to check
CLI reference freshness. Install and pin [docli](https://github.com/randlee/docli)
(the published crate is `sc-docli`) in the consumer environment. For example:

```sh
#!/usr/bin/env bash
set -euo pipefail
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
./your-cli dump-neutral-model > "$tmp_dir/model.json"
docli generate --input "$tmp_dir/model.json" --html "$tmp_dir/html"
cmp "$tmp_dir/html/index.html" docs/cli/index.html
```

Replace the model-dump command and reference path with your own. Declare the
script using the existing `id`, `description`, and `run` fields. This compares
generated bytes without changing the committed reference. The existing
optional `docli` configuration described above also remains available.
