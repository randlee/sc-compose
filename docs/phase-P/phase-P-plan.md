---
phase: P
title: Released sc-lint 0.5 Consumer Upgrade and Dual-Repository Acceptance
status: planned
target: develop
---

# Phase P — Released sc-lint 0.5 Consumer Upgrade and Dual-Repository Acceptance

## Plan status

- Type: cross-repository consumer-upgrade plan; no product implementation is
  claimed by this document.
- Planning branch: `feature/sc-lint-upgrade-planning`
- Planning worktree: `../sc-compose-worktrees/feature/sc-lint-upgrade-planning`
- Implementation target: `develop`; no direct change is made to `main`.
- sc-compose baseline reviewed: `38cf63a5e1fe68f93be39fbed30315de4e3b620f`
- atm-core baseline reviewed: `b3475b397c544bd43a43fb97f855b6ddb68f01b1`
- sc-lint baseline reviewed: `96f25a9ea7bdd913b996886c1a8d2da784bcb407`
- Product dependency: a released, checksum-verified sc-lint 0.5.0 artifact
  whose public consumer configuration contract is accepted and documented.

This is the authoritative Phase P plan. It does not assume that a planning
branch, a source checkout, or an ambient `PATH` binary proves a consumer setup
tool works. A release artifact must prove the same workflow against both real
reference consumers before either repository adopts it.

## Requirements, ADR, and NFR traceability

| ID | Requirement or constraint | Evidence / authority | Closure |
| --- | --- | --- | --- |
| P-R1 | One released sc-lint artifact must qualify both sc-compose and atm-core before either production-repository conversion starts. | Product direction; P.1 dual-consumer matrix | P.1 recorded PASS matrix |
| P-R2 | No fallback source archive, copied `.just` utility, `cargo run` integration, or consumer-local installer remains in an adopted setup path. | Phase E consumer contract; current sc-compose setup action evidence | P.2/P.3 path and source scans |
| P-R3 | Existing reserved `Justfile` recipes are never overwritten based on a heuristic. A supported explicit migration must be proven on disposable copies or the conversion is blocked. | ADR-0016, ADR-0017, sc-lint Phase F conflict policy | P.1 preflight and reapply evidence |
| P-R4 | A configuration must have one compatible-version authority: `[tool.sc-lint].minimum_version`. CI and local setup must derive the same artifact choice from it. | sc-lint Phase E requirements and Phase F plan | P.2/P.3 config and CI assertions |
| P-R5 | `just lint` and `just test` are complete repository gates after conversion; repository-specific checks may be retained only through an explicit reviewed composition. | Product direction; current Justfiles | P.2/P.3 functional evidence |
| P-R6 | sc-compose remains a consumer; no sc-lint Cargo dependency, analyzer reimplementation, or new process runner enters `sc-composer` or `bindings/python`. | CLAUDE.md Rules 1–5; ADR-0016 | P.2 boundary and dependency checks |
| P-R7 | sc-compose preserves the established allowlisted JSON/reporting boundary until an approved ADR amendment replaces it. | ADR-0017 | P.1 ADR decision; P.2 evidence |
| P-N1 | Qualification is deterministic, offline after verified artifact installation, and portable across Linux, macOS, and Windows. | Phase E distribution contract | P.1 all-platform matrix |
| P-N2 | Every actual repository write is previewed, carried out in a dedicated consumer worktree, and idempotently re-applied before a PR is proposed. | Product direction; P.1–P.3 gates | retained preview/apply/reapply evidence |

## Goal

Replace the two current, divergent consumer integrations with one released
sc-lint 0.5.0 setup contract that is demonstrably usable in **both** current
reference consumers:

1. sc-compose, which currently pins `0.4.0`, downloads a sc-lint source archive
   in its setup action, copies `.just/*.py` into its checkout, and runs a
   temporary `lint-ci-consumer` profile; and
2. atm-core, which currently has no `sc-lint.toml`, has a large user-owned
   `Justfile` with existing `lint`, `test`, and `ci` recipes, and uses many
   repository-owned `.just` Python helpers.

Phase P does not call those different layouts “compatible” because both contain
a Cargo workspace and a Justfile. The exact product artifact must successfully
discover, preview, apply in disposable copies, and reapply idempotently to both
layouts. If it cannot, the missing behavior is an sc-lint product finding and
Phase P remains blocked; no consumer workaround, copied script, or manual
unreviewed rewrite is authorized.

## Current evidence and non-assumptions

| Consumer | Observed current fact | Consequence for qualification |
| --- | --- | --- |
| sc-compose | `sc-lint.toml` uses `version = "0.4.0"`; `.github/actions/setup-sc-lint` downloads a release archive **and** a source archive, then copies `.just/*.py`; `Justfile` runs `cargo run --bin sc-compose` and has a temporary `lint-ci-consumer`. | P.2 must remove the source/copy workaround only after the released artifact performs the equivalent setup/lint/test contract. |
| atm-core | No `sc-lint.toml` exists. Its root `Justfile` owns many `.just` helpers and already defines `lint`, `test`, and `ci`. | P.3 cannot silently replace or rename any reserved recipe. The product must show an approved explicit composition/migration plan on a disposable copy first. |
| both | Cargo workspace and existing CI workflow exist. | Presence is not compatibility. The product must report exact planned operations and no-write conflicts. |

The plan does **not** assume that every current `.just` helper belongs to
sc-lint. A helper is removed only when the released product’s reviewed plan
identifies it as an exact replaced sc-lint integration asset and the post-apply
gate proves its behavior is covered. atm-core-specific operational, test,
release, and reporting helpers remain consumer-owned unless independently
classified and approved.

## Product qualification contract

### Required artifact and command evidence

Before P.2 or P.3 begins, P.1 records:

- the exact released sc-lint version, archive/checksum identity, and installed
  executable path for each operating-system matrix entry;
- the accepted public configuration command syntax and JSON schema version;
- the `sc-lint --json version` output and minimum-version preflight result;
- one sanitized request/selection document per consumer, with no shell command
  strings or source-checkout paths;
- the exact preview, apply, and reapply transcripts and their JSON results.

The artifact under test must be installed through the released installer or
release archive verification path. `cargo run`, a sibling checkout, copied
`.just` files, and an ambient developer installation are invalid qualification
evidence.

### Dual-repository test matrix

P.1 runs the following matrix against disposable worktrees/copies created from
the recorded sc-compose and atm-core baseline commits. It is the **same product
version** in every cell.

| Stage | sc-compose | atm-core | Required result |
| --- | --- | --- | --- |
| Preflight | inspect/version/compatibility | inspect/version/compatibility | structured success or an explicit supported conflict; never a crash, source fallback, or write |
| Preview | produce a complete JSON plan | produce a complete JSON plan | each planned file, retained file, conflict, and recovery action is reviewable |
| Apply | apply only the approved preview | apply only the approved preview | no README change, source archive, copied utility, arbitrary recipe replacement, or unlisted write |
| Reapply | rerun against the applied copy | rerun against the applied copy | zero planned content changes; stable no-op/idempotent result |
| Functional gate | `just setup`, `just lint`, `just test` | `just setup`, `just lint`, `just test` | complete configured gates pass or return the documented product failure class |
| CI gate | generated/adapted workflow executes the same contract | generated/adapted workflow executes the same contract | Linux, macOS, Windows use the one config version authority |

A conflict is acceptable only when it identifies a real unsupported shape and
causes no writes. It is not acceptable to declare Phase P ready after a
conflict: the sc-lint team must either implement the missing explicitly safe
behavior and rerun the entire matrix, or reject the conversion scope.

## Design and ownership boundary

The product owns discovery, configuration schema, recommended setup, artifact
selection, compatibility preflight, and any approved transaction. Each consumer
owns its own repository policy, non-sc-lint recipes, test/report semantics, and
PR review. sc-compose does not embed a sc-lint runner or add a sc-lint Cargo
dependency. atm-core does not gain a sc-compose dependency or a copied setup
implementation.

The required post-conversion public developer interface is:

```text
just setup
just lint
just test
just upgrade
```

“Complete” means the resulting `lint`/`test` commands run the configured
product profile plus any explicitly retained consumer checks. The wizard/JSON
request must show that composition before apply; it may not hide it behind a
generated shell string.

### Reserved-recipe migration is a hard product gate

Both reference consumers make an unproven generic “replace the Justfile” plan
unsafe. The relevant capability is not simple file presence; it is an explicit,
reviewable migration for an existing `lint`, `test`, or `ci` recipe that:

1. identifies every byte-range it will change and every consumer-owned recipe
   it retains;
2. has an approved user/agent selection for how the existing check composes
   with the configured sc-lint profile;
3. validates the resulting Justfile and preserves all content outside the
   reviewed transformation; and
4. passes preview → apply → reapply and the functional matrix above on **both**
   reference copies.

Until the released product provides and proves that capability, P-R3 is a
blocking product gap. Neither P.2 nor P.3 may emulate it with a custom Python
script, ad-hoc `sed`/YAML change, or manual recipe rewrite.

## Required ADR treatment

ADR-0016 currently records a pinned 0.4.0 release and permits a temporary
consumer-relative Python utility materialization. ADR-0017 records the 0.4
target registry and allowlisted sc-compose runner/report path. Phase P may not
silently contradict either decision.

P.1 must create an ADR amendment decision record (or formally amend ADR-0016
and ADR-0017 after team-lead approves the record number/scope) that states:

- the released 0.5.0 consumer contract and one-version-authority rule;
- removal of the source-archive/copy utility fallback only after dual-consumer
  evidence passes;
- the replacement or retirement status of the 0.4 target registry/runner and
  report-artifact contract;
- the safe reserved-recipe composition contract and its no-write failure mode;
- the explicit prohibition on a sc-compose-only or atm-core-only fork.

No implementation sprint starts before this ADR decision is accepted. If the
product’s released contract preserves the old runner unchanged, the amendment
must say so and must not falsely claim it was retired.

## Sequencing recommendation

```text
accepted sc-lint 0.5.0 consumer contract + verified release artifact
  -> P.1 dual-reference qualification on disposable copies + ADR amendment
      -> P.2 sc-compose consumer PR                   ┐
      -> P.3 atm-core consumer PR (separate repo)     ├-> dual CI/QA/reapply gate
                                                       -> Phase P close
```

P.2 and P.3 are independent only after P.1 passes. They must use the exact
same release version and recorded request schema. A product fix invalidates
both consumer qualifications and requires P.1 to rerun before either consumer
PR merges.

## Sprint records

- [Sprint P.1 — dual-reference product qualification](sprint-p-1-dual-reference-qualification.md)
- [Sprint P.2 — sc-compose released-consumer conversion](sprint-p-2-sc-compose-conversion.md)
- [Sprint P.3 — atm-core released-consumer acceptance](sprint-p-3-atm-core-acceptance.md)

## Boundary Rules compliance

### Rule 1 — `sc-composer` remains pure

No setup product, process invocation, release installer, or consumer migration
code is added to `sc-composer`.

### Rules 2–5 — dependency direction and Python adapter boundary

P.2 adds no Cargo dependency on sc-lint. `sc-compose` continues to depend only
on `sc-composer` and approved standalone observability crates; `bindings/python`
continues to depend only on `sc-composer`. No setup logic is introduced into
either library boundary.

### Rules 6–7 — no ATM runtime coupling

No `ATM_HOME`, ATM crate, or `agent-team-mail` dependency is introduced. The
atm-core qualification is external consumer evidence run in its own worktree,
not a sc-compose runtime dependency.

### Rules 8–9 — sc-sha boundaries remain unchanged

Phase P adds no hash, PyO3, or maturin dependency to the sc-sha core or its
adapter. Its consumer setup work is unrelated to those approved boundaries.

### ADR-0016 and ADR-0017

Their accepted ownership/security boundaries remain in force until P.1's ADR
amendment is accepted. In particular, this plan does not authorize arbitrary
descriptor commands, human-output parsing, copied analyzer code, or a
repository-specific sc-lint runner.

## Phase-level gates and external handoff

### Before implementation

- [ ] Record the accepted sc-lint release version, checksums, command/schema
      contract, and the actual artifact installation transcript.
- [ ] Record the two consumer baseline commits and create disposable copies;
      do not use dirty primary worktrees as qualification input.
- [ ] Review and approve the ADR-0016/0017 amendment or record why no change
      is needed.
- [ ] Confirm the exact generated file set and reserved-recipe composition
      behavior in both preview JSON documents.

### Phase-level gates

- [ ] P.1 matrix passes on Linux, macOS, and Windows using one release artifact
      version; preview, apply, and reapply evidence is retained for both repos.
- [ ] P.2 and P.3 each have an independently reviewed consumer PR, QA result,
      and post-merge revalidation on their own `develop` branch.
- [ ] `just setup`, `just lint`, `just test`, and `just upgrade` have the
      documented result in each clean consumer checkout.
- [ ] sc-compose has no active source archive, copied `.just` utility, or
      temporary 0.4 `lint-ci-consumer` workaround in setup/lint/CI paths.
- [ ] atm-core retains every explicitly consumer-owned helper; no deletion is
      justified merely by a filename match.
- [ ] Both CI workflows derive the selected sc-lint artifact from the same
      configuration version authority and pass on Linux, macOS, and Windows.

## Explicit non-goals

- Do not create a sc-compose-specific configuration wizard, setup Python
  package, Cargo integration, or analyzer runner.
- Do not make direct writes to atm-core from this repository/worktree.
- Do not replace a whole Justfile, delete a helper by name, or treat a source
  archive copy as a valid permanent fallback.
- Do not close Phase P based on a documentation review, an unexecuted request,
  a single consumer, a source checkout, or a green sc-compose-only CI run.
