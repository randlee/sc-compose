---
status: complete
branch: fix/homebrew-formula-bundled-paths-tojson
pr: 603
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
  consumer-local mirror and must not be hand-edited. It is restored byte-for-byte
  to its pre-`85f7dd7` upstream state. The canonical package correction is
  `randlee/sc-publish` PR #85 at `8f858ed`; that PR remains open and upstream
  `develop` still has the pre-fix template. CI fetches that actual upstream
  `develop` branch and diffs this specific vendored template against it. The
  guard therefore passes while PR #85 remains unmerged and fails after it
  merges until this consumer synchronizes the kit.
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
- **Validation evidence for `d4ada7a`:**
  [`PR #603 checks`](https://github.com/randlee/sc-compose/pull/603/checks)
  include the CI record for the code commit. The required commands also passed
  locally at that commit: `cargo test --workspace` and
  `python3 -m pytest -q .github/scripts/tests/test_release_artifacts.py`.
