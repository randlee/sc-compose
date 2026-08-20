---
id: Q.1
title: sc-publish package parity
status: complete
branch: sprint/q-1-sc-publish-package-parity
worktree: ../sc-compose-worktrees/sprint/q-1-sc-publish-package-parity
target: sc-publish develop
parallel_with: unrelated sc-compose work that does not modify publishing assets
---

# Sprint Q.1 — `sc-publish` Package Parity

## Scope

Make the canonical `sc-publish` package complete enough to install into
sc-compose without leaving stale publishing templates or requiring consumer
source edits. This sprint changes the sc-publish repository, not sc-compose
runtime code.

## Exact targets

- `sc-publish/plugins/sc-publish/install.py`
- `sc-publish/plugins/sc-publish/release/*`
- `sc-publish/plugins/sc-publish/.github/workflows/*`
- `sc-publish/plugins/sc-publish/.github/scripts/*`
- `sc-publish/plugins/sc-publish/.claude/*`
- `sc-publish/plugins/sc-publish/.cursor/*`
- `sc-publish/plugins/sc-publish/.github/scripts/tests/*`

## Deliverables

1. Package the complete mandatory Homebrew and Scoop channels: named agents,
   credential/environment-variable checks, publish workflows, required
   actions/scripts, and formula/manifest templates.
2. Verify the installer copies all shared publishing assets and renders both
   TOML outputs from complete JSON without inferring channels or targets.
3. Add regression tests proving a fresh consumer receives all workflow/action/
   script/agent/template files and that package tests do not depend on the
   package source checkout layout.
4. Verify the ATM-core-derived preflight and retry contracts remain intact.

## Acceptance criteria

- [ ] A clean temporary consumer installed from the package has every file
      referenced by its publishing workflows.
- [ ] Homebrew and Scoop each have a named agent, credential/environment
      check, publish workflow, required actions/scripts, and templates, and
      both workflows resolve their templates after installation.
- [ ] Complete JSON renders valid TOML with all sc-compose-relevant sections.
- [ ] Installer repeat-run returns zero and reports no drift.
- [ ] Package unit/integration tests pass on the exact commit handed to Q.2.
- [ ] Workflow YAML parses and all referenced local actions/scripts exist.
- [ ] No credential value is committed; channel secret names and locations
      remain those defined by the canonical channel contract.

## Required validation

```text
python3 -m pytest -q plugins/sc-publish/.github/scripts/tests
python3 .integration/manifest_examples.py
python3 -m py_compile plugins/sc-publish/install.py
git diff --check
```

## Handoff and fix routing

Send team-lead the sc-publish commit, package test output, installed-file
inventory, and template-resolution proof. Team-lead opens the PR and routes it
to quality-mgr. Remaining independent findings use fix worktrees from this
sprint commit and must be QA-approved before Q.1 closes.
