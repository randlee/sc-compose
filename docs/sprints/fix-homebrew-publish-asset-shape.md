---
status: complete
branch: fix/homebrew-publish-asset-shape
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/homebrew-publish-asset-shape
pr: main
---

# FIX: homebrew-publish.yml asset-shape mismatch breaks formula rendering

## Source

`publisher`'s PUBLISH-V1.5.0-CHANNEL-RETRY-MAIN-BBBDE7E completion report
(channel_retry against v1.5.0, commit bbbde7e0ba4184748305ed5e5b12e93e0ed5e3b5):
homebrew-publish.yml run 32521934942 failed in `update-tap`'s "Render
manifest-selected formulas with the published renderer" step with
`TypeError: string indices must be integers, not 'str'`. `verify-release`
passed; no tap commit landed; no credential/token issue.

## Root cause (confirmed by direct read)

`.github/workflows/homebrew-publish.yml`:
- Line 132 (asset-collection step, "Download and hash release archives" or
  equivalent): writes `homebrew-release-assets.json` as
  `json.dumps(assets)` where `assets` is already a **dict**
  `{"macos_arm": {"url": ..., "sha256": ...}, "macos_intel": {...}, "linux": {...}}`.
- Line 162 ("Render manifest-selected formulas" step): re-reads that same
  file and does
  `assets = {asset["key"]: asset for asset in json.loads(Path("homebrew-release-assets.json").read_text())}`
  — this assumes the file is a **list** of `{"key": ..., "url": ..., "sha256": ...}`
  objects. Iterating a dict yields its string keys, so `asset["key"]`
  indexes a string, raising the observed `TypeError`.

## Required fix

Line 162 only needs to load the dict as-is; it is already keyed by asset
key from the writer step:

```python
assets = json.loads(Path("homebrew-release-assets.json").read_text())
```

Do not change the writer step (line ~95-132) — its dict shape is correct
and simpler; fix the one incorrect consumer. Apply identically to the
`plugins/sc-publish` mirror if `homebrew-publish.yml` is vendored there
(confirm via `diff` — the sprint doc's mirror-identity requirement from
FIX-549 applies here too if so).

## Acceptance Criteria

- `homebrew-publish.yml`'s render-formulas step loads
  `homebrew-release-assets.json` using the shape the writer step actually
  produces (dict), with no re-listification.
- Add/adjust a regression test asserting the writer and reader steps agree
  on JSON shape (e.g. a script-level test that round-trips a representative
  `assets` payload through both steps' Python snippets, or an equivalent
  assertion against the workflow YAML text if no existing test harness
  covers workflow-embedded Python).
- Root and `plugins/sc-publish` mirrors remain byte-identical if the file
  is vendored there.
- No other steps' behavior changes.
- Full validation sweep passes (workflow YAML lint, existing
  `.github/scripts/tests/` suite).

## Non-blocking note

`winget` also failed in the same retry (`WINGET_GITHUB_TOKEN` lacks
permission to open PRs against `microsoft/winget-pkgs`) — that is a
token-scope issue requiring the repo owner to reconfigure the PAT's
permissions, not a code fix, and is explicitly out of scope for this task.
