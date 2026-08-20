---
phase: Q
title: Adopt the canonical sc-publish workflow
status: in_qa
branch: integrate/phase-q
target: develop
related_package: ../sc-publish/plugins/sc-publish
---

# Phase Q — Adopt the Canonical `sc-publish` Workflow

## Goal

Replace sc-compose's independently maintained publishing agents, scripts,
templates, and release workflows with the canonical `plugins/sc-publish`
package, while retaining only sc-compose's repository-specific release
manifest input and non-publishing CI/workflows.

The installed package is the source of truth for publishing behavior. The
consumer repository supplies a complete JSON manifest that is rendered into
`release/publish-artifacts.toml` and `release/publish-channel-contracts.toml`.

## Non-goals

- Do not change sc-compose runtime crates or release versions.
- Do not copy or modify unrelated CI, Pages, or documentation workflows.
- Do not weaken the package's ATM-core-derived preflight, authorization,
  credential, channel-result, retry, or release-state contracts.
- Do not publish production artifacts until the installed preflight and
  test-PyPI rehearsal gates pass.

## Sprint sequence and parallelism

| Sprint | Scope | Dependency | Parallelism |
| --- | --- | --- | --- |
| Q.1 | Complete canonical package parity and installer contract | Phase P complete; sc-publish develop baseline | May run in parallel with unrelated sc-compose work; blocks Q.2 |
| Q.2 | Install and cut over sc-compose, then prove publish readiness | Q.1 merged in sc-publish | May run in parallel with unrelated work that does not modify release assets or publishing workflows |
| Q.3 | Consume the current sc-publish develop update (re-vendor, verify, no internal sc-publish fixes) | Q.2 merged in sc-compose; sc-publish develop update merged | May run in parallel with unrelated work that does not modify release assets or publishing workflows |

Q.1 is primarily a cross-repository package sprint. Q.2 is the sc-compose
consumer migration sprint. Q.3 is a consumer-side update sprint: it re-vendors
an upstream sc-publish update and revalidates sc-compose's own surface: it
does not implement or fix sc-publish's internal probe, workflow, or install
logic — those defects are filed as sc-publish issues instead. Neither sprint
may silently maintain a second publishing implementation.

## Shared contracts

### Canonical package

The authoritative source is `sc-publish/plugins/sc-publish` at its reviewed
`develop` commit. The installer is:

```text
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json .
```

The install input is complete and caller-owned. It must declare project
metadata, release targets, crates and publish order, release binaries, Python
packages/distributions, and all four channels (`pypi`, `homebrew`, `scoop`,
`winget`). The installer must render both repository-specific TOML manifests
and copy the shared publishing assets without source-specific edits.

### Consumer-owned files

After installation, the only publishing-specific sc-compose input is:

```text
release/sc-publish-install.json
```

The generated files are outputs and must not be hand-edited:

```text
release/publish-artifacts.toml
release/publish-channel-contracts.toml
```

Unrelated `.github/workflows/ci.yml`, `.github/workflows/pages.yml`, and
non-publishing documentation remain consumer-owned.

## Required cross-repository gate

Before Q.2 begins, sc-publish must contain the complete Homebrew and Scoop
channel implementations. Each mandatory channel must include its named agent,
credential/environment-variable checks, publish workflow, required actions and
scripts, and all templates consumed by that workflow. These are package-owned
assets, not consumer-owned exceptions.

The sc-publish package must also have passing installer, manifest-rendering,
workflow-contract, and script tests on the exact commit used by Q.2.

## Phase acceptance criteria

- [ ] The reviewed sc-publish package contains every shared publishing agent,
      skill, script, workflow, action, channel template, and test required by
      its own workflows.
- [ ] A complete sc-compose install JSON is committed and validates against
      the package installer without inferred targets or channels.
- [ ] Running the installer against sc-compose produces the expected generated
      manifests and installs the canonical publishing assets.
- [ ] A second `--dry-run` reports no drift.
- [ ] No publishing workflow, agent, or helper remains as an independently
      maintained duplicate outside the installed package; unrelated CI and
      Pages workflows remain intact.
- [ ] Installed Release Preflight passes its manifest, version, package,
      registry, credential, and ATM-core-derived channel gates for a selected
      rehearsal version.
- [ ] Test-PyPI rehearsal and per-channel result/retry behavior are proven
      without production publication.
- [ ] Production publication is explicitly authorized only after the exact
      `main` commit passes final preflight and the release gate.
- [ ] The installed package tree is re-vendored to match sc-publish develop
      whenever an upstream update is consumed, with byte-for-byte diff proof
      and no sc-publish internal logic modified in the consuming sprint.

## Required validation

Each sprint must run its own validation and record exact output. Phase Q
closeout reruns:

```text
python3 -m pytest -q <sc-publish package tests>
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
python3 .github/scripts/release_artifacts.py validate-manifest \
  --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
git diff --check
```

The first dry run is expected to report drift before installation. The second
dry run must return success with no drift. Workflow YAML must parse, and the
installed preflight must be run through GitHub Actions before any production
publish attempt.

## QA and follow-on fix routing

Any sc-lint or package-parity finding that remains after Q.1 or Q.2 gets a
dedicated fix worktree from the sprint commit, grouped by independent issue
class. Send the fix commit and evidence to team-lead for PR creation; team-lead
routes it to quality-mgr. A sprint is not complete until required fix worktrees
are QA-approved, merged, and the merged parent is revalidated.

## Sprint index

- [Sprint Q.1 — sc-publish package parity](sprint-q-1-sc-publish-package-parity.md)
- [Sprint Q.2 — sc-compose install and publish cutover](sprint-q-2-sc-compose-publish-cutover.md)
- [Sprint Q.3 — consume sc-publish develop update](sprint-q-3-sc-publish-consume-update.md)
