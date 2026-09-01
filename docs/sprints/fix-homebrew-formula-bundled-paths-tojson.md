---
status: pending-upstream
branch: fix/homebrew-formula-bundled-paths-tojson
pr: 603
upstream_pr: https://github.com/randlee/sc-publish/pull/85
---

# Fix: render Homebrew bundled-path helpers as Ruby methods

## Scope

Render the first `bundled_paths.destination_components` entry as a Homebrew
path helper and quote only later path segments. This produces, for example,
`(pkgshare/"examples").install ...`, not `("pkgshare"/"examples").install ...`.

## Closeout

- **Load-bearing template:** `release/homebrew/formula.rb.j2`. The release
  manifest selects that exact path at
  `release/publish-artifacts.toml` → `channels.homebrew.formulas[0].template`;
  `.github/workflows/homebrew-publish.yml` reads the manifest and passes the
  selected template to the published renderer.
- **Vendored copy:** `plugins/sc-publish/release/homebrew/formula.rb.j2` is a
  consumer-local mirror and must not be hand-edited. It temporarily matches the
  validated fix in [sc-publish PR #85](https://github.com/randlee/sc-publish/pull/85)
  at `8f858ed`, while upstream `develop` remains unfixed. CI fetches both
  upstream `develop` and PR #85: it accepts byte parity with either source and
  rejects every other divergence. Once PR #85 merges, `develop` becomes the
  required byte-parity source automatically.
- **Prior test gap:** the earlier regression only asserted that the rendered
  text contained `("pkgshare"/"examples").install ...` and ran `ruby -c`.
  Quoted and bare helper forms are both valid Ruby syntax, so neither check
  exercised Homebrew's runtime path-helper API. The regression now executes
  a multi-component rendered `install` block against a minimal
  Homebrew-compatible helper harness; the old quoted receiver fails because a
  Ruby `String` has no `install` method. The Rust CLI test keeps its
  cross-platform rendering assertion only: GitHub's `windows-latest` image
  currently includes Ruby, but the Rust test does not declare Ruby as a
  toolchain dependency. Ruby execution is therefore intentionally owned by
  this Ubuntu Python regression rather than a runner-provided executable.
- **Validation evidence (QA6 correction after `ed6471c`):** both the
  load-bearing top-level `.github/scripts/tests` suite and the vendored
  `plugins/sc-publish/.github/scripts/tests` suite pass genuinely with the
  fixed template; no xfail or skip is used. The CI guard permits this single,
  tracked, exact PR #85 divergence while that PR is open, while still failing
  any divergence from both upstream sources.

## QA7 coverage and follow-up

At `d942f7b`, the CI guard began accepting only byte parity with upstream
`develop` or the exact [sc-publish PR #85](https://github.com/randlee/sc-publish/pull/85)
head; every other vendored divergence fails. The vendored three-component
fixture was added later, at `c48b046`.

The top-level and vendored Ruby-execution fixtures both use
`["pkgshare", "examples", "nested"]`. They therefore execute the bare helper
and the subsequently JSON-quoted path components, and fail against the
pre-fix template. QA7 re-runs both focused tests as genuine passes.

- **Tracked follow-up:** when PR #85 merges upstream and this repository
  synchronizes its vendored copy, remove the PR #85 fetch-and-diff branch from
  `.github/workflows/ci.yml` and require `sc-publish/develop` byte parity only.
