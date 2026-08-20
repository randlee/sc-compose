---
id: Q.4
title: consume sc-publish release-candidate provenance update
status: complete
branch: sprint/q-4-sc-publish-rc-provenance-consume
worktree: ../sc-compose-worktrees/sprint/q-4-sc-publish-rc-provenance-consume
target: integrate/phase-q
depends_on: Q.3 merged (sc-compose develop); sc-publish PR #48 merged (sc-publish develop 2c91d7b)
parallel_with: unrelated work outside release assets and publishing workflows
---

# Sprint Q.4 — Consume `sc-publish` Release-Candidate Provenance Update

## Scope

sc-compose is a **consumer** of the `sc-publish` package, not its owner. This
sprint installs the canonical `sc-publish` package byte-for-byte from the
merged upstream release-candidate-provenance work (sc-publish PR #48,
`develop` commit `2c91d7b`), replacing the Q.3-era vendored copy, and renders
the result only against sc-compose's own existing manifests.

This sprint does **not** modify `sc-publish`'s internal workflow, gate, or
policy-doc logic. sc-publish PR #48 replaces the prior main/develop
content-equality release gate with publisher-managed
`release-candidate-vX.Y.Z` provenance (`release-candidate.yml` as sole
creator/reuser of the tag, ancestry validation against the readiness ref and
final `origin/main`, post-cut drift recording/escalation, and updated
Claude/Codex and Cursor publisher policy docs); this consumer sprint only
re-vendors that upstream result.

## Exact targets

- `plugins/sc-publish/` (full re-vendor from sc-publish develop `2c91d7b`)
- `.claude/agents/publisher.md`, `.cursor/agents/publisher.md`,
  `.claude/skills/publishing/SKILL.md`,
  `.claude/skills/publishing/ref/release-state-strategy.md` (installed
  publisher policy docs)
- `.github/workflows/release-candidate.yml` (new),
  `.github/workflows/release-preflight.yml`,
  `.github/scripts/release_gate.sh`
- `.github/scripts/tests/test_install.py`,
  `.github/scripts/tests/test_publish_kit_assets.py`,
  `.github/scripts/tests/test_publish_kit_scripts.py`,
  `.github/scripts/tests/test_release_artifacts.py`
- `README.sc-publish.md`
- Superseded publish-kit files: remove only if installer ownership requires
  it (i.e. the installer's own manifest declares a file removed/renamed
  upstream); do not hand-prune anything the installer did not itself retire.

## Deliverables

1. Re-run the installer against the current `sc-publish` develop commit
   (`2c91d7b`) using the existing `release/sc-publish-install.json`, updating
   only what the new package version requires.
2. Confirm the resulting `plugins/sc-publish/` tree matches upstream
   `sc-publish` develop byte-for-byte (diff against the sc-publish repo
   checkout, not just "installer exited zero").
3. Render only the existing sc-compose manifests
   (`release/sc-publish-install.json` and its generated
   `release/publish-artifacts.toml` / `release/publish-channel-contracts.toml`
   outputs) against the new package version; do not author new manifest
   surface for this sprint.
4. Remove any publish-kit files the installer itself marks superseded; leave
   everything else untouched.
5. Prove sc-compose's own dry-run parity holds: a second installer dry run is
   clean, and the installed `release-candidate.yml` /
   `release-preflight.yml` workflows reference only existing local assets.
6. Open the PR to `integrate/phase-q` (per the now-merged integration-branch
   policy — not `develop` directly).

## Acceptance criteria

- [x] `plugins/sc-publish/` matches sc-publish develop `2c91d7b` exactly
      (no drift, no local patches).
- [x] A second installer dry run is clean with exit code zero.
- [x] No sc-publish internal logic (`release-candidate.yml`'s gate
      conditions, `release_gate.sh`'s ancestry checks, publisher policy
      content) is modified by this sprint's diff beyond the installer's own
      re-vendor.
- [x] No files outside the installer's declared ownership are deleted.
- [x] `python3 .github/scripts/tests/test_install.py`,
      `test_publish_kit_assets.py`, `test_publish_kit_scripts.py`, and
      `test_release_artifacts.py` pass against the updated install.
- [x] Sprint PR targets `integrate/phase-q`, not `develop`.

## Required validation

```text
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
diff -rq plugins/sc-publish/ <sc-publish-repo-checkout>/plugins/sc-publish/
python3 -m pytest .github/scripts/tests -q
git diff --check
```

## Validation evidence

- Byte-diff check: sc-publish checked out at `2c91d7b` in a worktree, `diff
  -rq` against `plugins/sc-publish/` in this sprint's tree returns no
  differences other than gitignored `__pycache__` dirs — content is
  byte-for-byte identical.
- `python3 -m pytest .github/scripts/tests -q`, run through the Q.3-pinned
  bootstrap venv (`/private/tmp/sc-compose-q3-venv-1.4.1`,
  `sc-compose==1.4.1`): 80 passed, 7 skipped, 3 subtests passed, 0 failed.
  (A stale ambient global `sc-compose==1.2.0` wheel on the local machine
  otherwise produces one unrelated `tojson`-filter failure
  (`test_release_channel_templates_render_to_valid_ruby_and_json`); this is a
  pre-existing local-environment gap, not a regression from this diff, and is
  tracked separately.)
- Second installer dry run through the same pinned venv: "Publish-kit assets
  are in sync.", exit 0.
- `git diff origin/integrate/phase-q...HEAD --diff-filter=D --name-status`
  returns zero files: nothing was deleted outside installer ownership.
- Independently re-verified by `quality-mgr` during Sprint Q.4 QA
  (`Q4-SC-PUBLISH-RC-PROVENANCE-QA1`), which reproduced all of the above and
  refuted `req-qa`'s evidence-gap findings (SCQ-QA-001/002/003/005). QA
  verdict: PASS, 6/6 deliverables, CI 14/14 green.

## Handoff and fix routing

Send `team-lead` the sc-publish source commit, dry-run proof, upstream-diff
proof, and test-suite output. `team-lead` opens the PR to `integrate/phase-q`
and routes it to `quality-mgr`. Any finding that is actually an sc-publish
defect (not an install/consumption problem) gets filed as an sc-publish
issue, not fixed in this sprint's worktree.
