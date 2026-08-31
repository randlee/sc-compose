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
  consumer-local mirror. It received the same stopgap correction in this PR.
  The canonical package correction is in flight on
  `randlee/sc-publish:fix/homebrew-formula-bundled-paths-tojson`; a subsequent
  package sync should take that canonical copy rather than independently
  re-deciding this behavior.
- **Prior test gap:** the earlier regression only asserted that the rendered
  text contained `("pkgshare"/"examples").install ...` and ran `ruby -c`.
  Quoted and bare helper forms are both valid Ruby syntax, so neither check
  exercised Homebrew's runtime path-helper API. The regression now executes
  the rendered `install` block against a minimal Homebrew-compatible helper
  harness; the old quoted receiver fails because a Ruby `String` has no
  `install` method.
- **Validation evidence for `d4ada7a`:**
  [`PR #603 checks`](https://github.com/randlee/sc-compose/pull/603/checks)
  include the CI record for the code commit. The required commands also passed
  locally at that commit: `cargo test --workspace` and
  `python3 -m pytest -q .github/scripts/tests/test_release_artifacts.py`.

