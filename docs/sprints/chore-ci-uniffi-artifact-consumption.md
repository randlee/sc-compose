---
sprint: CHORE.CI-UNIFFI-CONSUME
phase: Phase P follow-up (post-release CI cost reduction)
status: assigned
branch: chore/ci-uniffi-artifact-consumption
worktree: ../sc-compose-worktrees/chore/ci-uniffi-artifact-consumption
target: integrate/phase-p
owner: comp
---

# Consume the published uniffi-bindgen-go generated-source artifact in CI

## Problem

`sc-compose` CI currently installs and compiles the pinned `uniffi-bindgen-go`
generator independently in **every** `sc-sha-go` matrix job
(`.github/workflows/ci.yml`, job `sc-sha-go`, matrix
`linux/amd64`, `darwin/arm64`, `windows/amd64`):

- `.github/scripts/install_uniffi_bindgen_go.py` runs a `cargo install
  uniffi-bindgen-go --git ... --tag v0.7.1+v0.31.0 --locked` per job
  (ci.yml:292, mirrored in `.github/workflows/release.yml`).
- The generator binary itself is intentionally Linux-only (see
  `sc-publish`'s `plugins/sc-publish/packages/uniffi-bindgen-go/manifest.toml`:
  "The published generator is intentionally Linux-only. Consumers that need
  generated Go sources on another runner use the accompanying workflow's
  generated-source artifact instead of executing a Linux binary there.").
- Despite that, macOS and Windows runners currently pay the full Rust
  compile cost of building `uniffi-bindgen-go` from source on every run,
  even with the existing `actions/cache` step (ci.yml:279-289) — a cache
  miss (dependency bump, cold cache, or the darwin/arm64 retry case fixed
  in PR #520) means a 20-35+ minute Rust compile on that platform, per job,
  just to produce a generator binary that then emits **host-neutral Go
  source text**.
- `uniffi-bindgen-go generate` (invoked via `just generate-sc-sha-go check`,
  ci.yml:299) is fast; the cost is entirely in building the generator
  binary, not in running it.

`sc-publish` already ships a reusable workflow
(`plugins/sc-publish/packages/uniffi-bindgen-go/release-workflow.yml`,
merged at `randlee/sc-publish` PR #26) that builds the pinned generator on
Ubuntu once and can emit the generated Go source as a build artifact
(`uniffi-bindgen-go-generated`). That workflow is not yet wired to run
automatically or to publish the generated source as a **durable, keyed**
artifact that `sc-compose`'s later CI runs can fetch — a separate item
(`SCPUB.2-FIX-WIRE-UNIFFI-CALLER`, already assigned) covers wiring a caller
onto that workflow on the `sc-publish` side. This sprint is the
**`sc-compose`-side consumption half** of the same architecture change.

## Required change (user-specified)

1. Cache/download the published generated Go artifact keyed by
   **generator version + a hash of the UDL/API surface**
   (`bindings/sc-sha-go/src/sc_sha_go.udl` and `bindings/sc-sha-go/uniffi.toml`
   — anything that changes the generated output). Do not key on
   `Cargo.lock` alone (that's the current generator-binary cache key,
   ci.yml:286/288, and is orthogonal to this).
2. Do **not** run `cargo install` (i.e. do not build the generator binary)
   in every OS matrix job. Only the platform that can run the generator
   (Linux) should ever build/execute it.
3. Regenerate and publish the generated Go source **only when the key
   changes** — a cache/artifact hit must skip generation entirely.
4. Platform jobs (darwin/arm64, windows/amd64, and linux/amd64 itself for
   consistency) consume the already-generated, host-neutral Go source and
   only build/test the **native** `sc-sha-go` static library and Go module
   for their own target. They must not invoke the generator.

## Suggested shape (adjust as the implementation requires — this is not a
rigid contract, the four numbered requirements above are)

- Add a `sc-sha-go-generate` job (parallel to or preceding the existing
  `sc-sha-go-plan` job, ci.yml:240) that runs on `ubuntu-latest`:
  - Computes `key = "${generator_version}-${sha256(udl + uniffi.toml)}"`.
  - Restores from `actions/cache` (or fetches a durable `sc-publish`
    release/artifact keyed the same way, if `SCPUB.2` lands a durable
    publish path first — coordinate with that item's actual output rather
    than assuming its exact mechanism).
  - On a cache miss only: builds the generator (existing
    `install_uniffi_bindgen_go.py` logic, unchanged), runs
    `uniffi-bindgen-go generate`, saves the generated source to the cache
    under that key, and uploads it as a build artifact for the matrix jobs
    in this same workflow run to consume via `actions/download-artifact`.
- The `sc-sha-go` matrix job (ci.yml:258) drops its
  "Install pinned UniFFI Go generator" and "Test pinned UniFFI Go generator
  installer" steps (ci.yml:291-296) entirely — those move to the new
  `sc-sha-go-generate` job (Linux-only, so the existing installer script and
  its unit tests still run, just once instead of per-matrix-entry).
  Matrix jobs instead download the generated-source artifact and run
  `just generate-sc-sha-go check` in verify-only mode against the
  downloaded source (or an equivalent "place these files, don't regenerate"
  step — check what `just generate-sc-sha-go check` actually does before
  assuming it can run without the generator binary present; it may need a
  new `just` target that just verifies file placement).
- Apply the same restructuring to `.github/workflows/release.yml`, which
  mirrors this per-job install pattern.
- Keep `.github/scripts/install_uniffi_bindgen_go.py` and its test file —
  they're still needed by the single Linux generation job, and PR #520's
  CI-wiring fix (ci.yml:294-296) should move with the step, not be deleted.

## Out of scope

- The `sc-publish`-side durable-publish mechanism itself
  (`SCPUB.2-FIX-WIRE-UNIFFI-CALLER`) — this sprint should consume whatever
  contract that item produces, or use `actions/cache` alone if a
  cross-repo durable artifact isn't ready yet. Do not block this sprint on
  `SCPUB.2`; an intra-workflow `actions/cache` keyed the same way satisfies
  requirements 1-4 on its own and can be pointed at a durable `sc-publish`
  artifact later without changing the matrix-job contract.
- Any change to the generated Go source's actual content, the native
  `sc-sha-go` build/test steps after generation, or the release-layout
  bundling logic.

## Acceptance criteria

- `cargo install uniffi-bindgen-go` (or equivalent generator build) runs at
  most once per CI run, on `ubuntu-latest` only.
- `darwin/arm64` and `windows/amd64` matrix jobs contain no step that
  builds or installs the generator binary.
- A no-op CI run (UDL/uniffi.toml unchanged, generator version unchanged)
  demonstrably skips generation (cache hit) and still produces a correct
  generated-source checkout for every matrix job.
- A UDL-content-changing commit demonstrably regenerates (cache miss) and
  the new source propagates to all matrix jobs.
- Full CI suite still passes end-to-end (all existing `sc-sha-go` matrix
  job steps — build, conformance tests, consumer-module test,
  release-layout check — unchanged in behavior).
- Report the measured wall-clock change for `darwin/arm64` and
  `windows/amd64` `sc-sha-go` jobs before/after (this is the 30+ minute
  cost the user is trying to eliminate).

## Verification evidence

- Cache miss and generation: [CI run 32221174807, generation job 95972903359](https://github.com/randlee/sc-compose/actions/runs/32221174807/job/95972903359).
  The generated-source cache was absent, the pinned installer built the
  generator once on Ubuntu, and the cache was saved. The generation job ran
  from 06:00:10Z to 06:07:20Z (7m10s).
- Cache hit and generation skip: [CI run 32223576326, generation job 95979491029](https://github.com/randlee/sc-compose/actions/runs/32223576326/job/95979491029).
  With the same UDL/config and generator version, the keyed cache was reused;
  the generation job completed from 06:32:47Z to 06:33:04Z (17s), with no
  installer or generator build.
- Wall-clock comparison: before this change, [run 32211066138](https://github.com/randlee/sc-compose/actions/runs/32211066138)
  took approximately 34m06s for darwin/arm64 and 19m48s for windows/amd64.
  After artifact generation was centralized, [run 32221174807](https://github.com/randlee/sc-compose/actions/runs/32221174807)
  took approximately 1m37s for darwin/arm64 and 2m35s for windows/amd64.
