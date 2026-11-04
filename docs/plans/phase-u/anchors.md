# Anchors - phase-u

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-u/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | platform matrix | 2026-11-03 |
| ANC-002 | locked | full matrix only in integration test | 2026-11-03 |
| ANC-003 | locked | docli generates CLI docs | 2026-11-03 |
| ANC-004 | locked | docli every publish | 2026-11-03 |
| ANC-005 | locked | kit preflight extension point | 2026-11-03 |
| ANC-006 | locked | docli optional kit check | 2026-11-03 |
| ANC-007 | locked | variables rendered before bd | 2026-11-03 |
| ANC-008 | locked | values stay data | 2026-11-03 |
| ANC-009 | locked | filters on real value | 2026-11-03 |
| ANC-010 | locked | include defaults | 2026-11-03 |
| ANC-011 | locked | bead inputs on graph and attach | 2026-11-03 |
| ANC-012 | locked | bd-owned explicit mode | 2026-11-03 |
| ANC-013 | locked | refuse blocked plans | 2026-11-03 |
| ANC-014 | locked | reject unsafe inputs | 2026-11-03 |
| ANC-015 | locked | name plan path failures | 2026-11-03 |
| ANC-016 | locked | bd substitution is buggy | 2026-11-03 |
| ANC-017 | locked | bead port at end of Phase U | 2026-11-03 |
| ANC-018 | locked | closing fuzz sprint | 2026-11-03 |
| ANC-019 | locked | no undocumented decisions | 2026-11-03 |
| ANC-020 | locked | pouring documented | 2026-11-03 |

## ANC-001

Anchor: The sc-compose release must deliver the entire platform matrix: the Rust CLI archive, the sc-compose, sc-sha and sc-composer-beads Python wheels, and the sc-sha-go Go module, each for macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64, distributed through all applicable channels.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: platform matrix

Scope: release/publish-artifacts.toml, release/go-native-module.toml, release workflows, wheel builds; 6 platforms x 5 artifacts

Deterministic check to pair with the development gate: a manifest-vs-matrix check that every one of the 30 cells has a built artifact and channel entry

## ANC-002

Anchor: CI must run the full install matrix, meaning every platform, Python version and installer cell, only in the integration test workflow that release preflight requires.

> I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: full matrix only in integration test

Scope: GitHub workflows: sprint and pull-request CI (ci.yml) must not run the full matrix; the workflow_dispatch-only integration test workflow runs it

Deterministic check to pair with the development gate: grep the workflows: the full-matrix workflow has only workflow_dispatch triggers and ci.yml never invokes it

## ANC-003

Anchor: docli must be added and used for CLI documentation generation.

> we need to add docli for cli documentation generation

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: docli generates CLI docs

Scope: sc-compose CLI reference generation (Phase U u-6, #648)

Deterministic check to pair with the development gate: the CLI reference files are produced by docli generate from the dumped Clap model

## ANC-004

Anchor: docli must be run on every publish, and stale documentation must block preflight.

> docli must be run every publish (stale documentation blocks preflight)

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: docli every publish

Scope: release preflight of sc-compose

Deterministic check to pair with the development gate: preflight regenerates the CLI reference with docli and fails when it differs from the committed reference

## ANC-005

Anchor: An extension point for repos to add custom requirements to preflight must be added as a pull request to the develop branch of sc-publish.

> there is not an extension point that repos can add custom requirements.  this is something that should be added as a pr to develop on sc-publish

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: kit preflight extension point

Scope: sc-publish kit, repository preflight

Deterministic check to pair with the development gate: a merged PR on sc-publish develop adds the extension point

## ANC-006

Anchor: docli must be offered as an optional preflight check.

> docli should be offered as an optional preflight check ( i want all repos that have cli to use docli)

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: docli optional kit check

Scope: sc-publish kit, repositories with a CLI

Deterministic check to pair with the development gate: the sc-publish kit manifest documents and implements an optional docli preflight check

## ANC-007

Anchor: Variables, both tight {{key}} and padded {{ key }}, including loop-local values, must be rendered before bd.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: variables rendered before bd

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: render a loop with tight and padded variables and read back the text bd receives: no unrendered variable remains

## ANC-008

Anchor: Values containing {{ }}, {{{ }}} or {% %} must stay data and must never be re-parsed.

> Values containing {{ }}, {{{ }}} or {% %} stay data and are never re-parsed.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: values stay data

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: supply values containing each delimiter and read back identical literal text

## ANC-009

Anchor: Filters must act on the real value, so A{BC lowercases to a{bc and has length 4.

> Filters act on the real value, so A{BC lowercases to a{bc and has length 4.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: filters on real value

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: render lower and length filters on A{BC and compare output

## ANC-010

Anchor: Include defaults must be applied.

> Include defaults.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: include defaults

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: an included template default reaches the rendered output when the caller omits it

## ANC-011

Anchor: Bead inputs must be supported on graph and attach pour.

> Bead inputs on graph and attach pour

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: bead inputs on graph and attach

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: graph-mode pour and attach accept bead input values under the advanced mode

## ANC-012

Anchor: bd-owned expressions must be an explicit mode.

> bd-owned expressions as an explicit mode

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: bd-owned explicit mode

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: bd-owned expression handling is selected only by an explicit option; the default is unchanged

## ANC-013

Anchor: Blocked graph plans must be refused before bd.

> refusing blocked graph plans before bd

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: refuse blocked plans

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: a blocked graph plan produces a refusal and zero bd create calls

## ANC-014

Anchor: Unsafe bead inputs must be rejected.

> rejecting unsafe bead inputs

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: reject unsafe inputs

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) (definition of unsafe input pending from the user)

Deterministic check to pair with the development gate: unsafe inputs produce a typed error and zero bd calls

## ANC-015

Anchor: Plan path failures must be named.

> naming plan path failures

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: name plan path failures

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: a failing graph plan path appears by name in the error

## ANC-016

Anchor: sc-compose must not rely on bd variable substitution, which is buggy.

> beads variable substitution is buggy

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: bd substitution is buggy

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: the text passed to bd is correct with no bd --var substitution

## ANC-017

Anchor: The bead pour work must be added to the end of Phase U, after the upgrades.

> let's add it to the end of phase-u (after the upgrades)

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: bead port at end of Phase U

Scope: Phase U plan sequencing

Deterministic check to pair with the development gate: the bead sprints depend on every upgrade sprint in the plan

## ANC-018

Anchor: The bead pour work must have fuzz tests at the end that confirm everything works and verify that previous fuzz test runs show no regression.

> it should have fuzz tests at the end to confirm everything works (and verify previous fuzz tests run have no regression).

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: closing fuzz sprint

Scope: Phase U plan, final sprint

Deterministic check to pair with the development gate: the final sprint runs the earlier fuzz cases and they all pass

## ANC-019

Anchor: The bead pour work must not be done by pair programming or by making undocumented decisions.

> I do not like pair programming and making undocumented decisions.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: no undocumented decisions

Scope: bead pour work planning

Deterministic check to pair with the development gate: an ADR or requirement exists before each implementation sprint starts

## ANC-020

Anchor: Pouring must be clearly documented, in a requirement or an ADR.

> pouring clearly documented (or ADR)

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: pouring documented

Scope: advanced bead pour: sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach); documentation

Deterministic check to pair with the development gate: ADR-0024/FR-24 exist and describe each mode with a worked example
