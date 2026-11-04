# Anchors - phase-u

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-u/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | values stay data | 2026-11-03 |
| ANC-002 | proposed | tight values before bd | 2026-11-03 |
| ANC-003 | proposed | padded values before bd | 2026-11-03 |
| ANC-004 | proposed | loop-local values before bd | 2026-11-03 |
| ANC-005 | proposed | lowercase sees literal value | 2026-11-03 |
| ANC-006 | proposed | length sees literal value | 2026-11-03 |
| ANC-007 | proposed | included default supplies missing value | 2026-11-03 |
| ANC-008 | proposed | bead inputs on graph pour | 2026-11-03 |
| ANC-009 | proposed | bead inputs on attach pour | 2026-11-03 |
| ANC-010 | proposed | explicit bd-owned mode | 2026-11-03 |
| ANC-011 | proposed | blocked graph path before any bd | 2026-11-03 |
| ANC-012 | proposed | human graph-plan path | 2026-11-03 |
| ANC-013 | proposed | JSON graph-plan field | 2026-11-03 |
| ANC-014 | proposed | docli generates CLI documentation | 2026-11-03 |
| ANC-015 | proposed | docli runs every publish | 2026-11-03 |
| ANC-016 | proposed | stale CLI docs block preflight | 2026-11-03 |
| ANC-017 | proposed | six platforms for Rust CLI archive | 2026-11-03 |
| ANC-018 | proposed | six platforms for sc-compose Python wheel | 2026-11-03 |
| ANC-019 | proposed | six platforms for sc-sha Python wheel | 2026-11-03 |
| ANC-020 | proposed | six platforms for sc-composer-beads Python wheel | 2026-11-03 |
| ANC-021 | proposed | six platforms for sc-sha-go Go native module | 2026-11-03 |
| ANC-022 | proposed | applicable artifact channels | 2026-11-03 |
| ANC-023 | proposed | full matrix only in integration test | 2026-11-03 |
| ANC-024 | proposed | no full matrix in sprint CI | 2026-11-03 |
| ANC-025 | proposed | previous fuzz cases at end | 2026-11-03 |
| ANC-026 | proposed | pouring documented | 2026-11-03 |
| ANC-027 | proposed | publish after Phase U | 2026-11-03 |
| ANC-028 | proposed | next release 1.8.0 | 2026-11-03 |
| ANC-029 | proposed | deterministic tests | 2026-11-03 |
| ANC-030 | proposed | Windows cross-build | 2026-11-03 |
| ANC-031 | proposed | lint passes | 2026-11-03 |
| ANC-032 | proposed | Windows integration runs on GitHub | 2026-11-03 |
| ANC-033 | proposed | integration runs outside normal CI | 2026-11-03 |

## ANC-001

Anchor: Values containing {{ }}, {{{ }}} or {% %} must stay data and must never be re-parsed.

> Values containing {{ }}, {{{ }}} or {% %} stay data and are never re-parsed.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: values stay data

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: supply values containing each delimiter and read back identical literal text from the text passed to bd

## ANC-002

Anchor: In advanced Beads rendering, tight {{key}} variable expressions must be rendered before bd receives the formula.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: tight values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: render.rs:bead_variables_render_padded_and_tight_placeholders

## ANC-003

Anchor: In advanced Beads rendering, padded {{ key }} variable expressions must be rendered before bd receives the formula.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: padded values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: render.rs:bead_variables_render_padded_and_tight_placeholders

## ANC-004

Anchor: In advanced Beads rendering, variable expressions using the current loop item must be rendered before bd receives the formula.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: loop-local values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: bdb3bdae single_pass_render.rs FUZZ-171-03

## ANC-005

Anchor: In advanced Beads rendering, applying the lowercase filter to the value A{BC must produce a{bc.

> Filters act on the real value, so A{BC lowercases to a{bc and has length 4.

Said by randlee, Proposed from524772f4:ANC-009; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: lowercase sees literal value

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: bdb3bdae single_pass_render.rs FUZZ-171-09

## ANC-006

Anchor: In advanced Beads rendering, applying the length filter to the value A{BC must produce 4.

> Filters act on the real value, so A{BC lowercases to a{bc and has length 4.

Said by randlee, Proposed from524772f4:ANC-009; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: length sees literal value

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: bdb3bdae single_pass_render.rs FUZZ-171-09

## ANC-007

Anchor: In advanced Beads rendering, an included fragment's frontmatter default must supply an otherwise missing variable value.

> Include defaults.

Said by randlee, Proposed from524772f4:ANC-010; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: included default supplies missing value

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: bdb3bdae:single_pass_render.rs:fuzz_171_11_include_defaults_reach_the_render_context; d754230a:render.rs:frontmatter_default_satisfies_an_otherwise_undefined_variable

## ANC-008

Anchor: Graph-based Beads pour must accept caller-supplied bead variable inputs.

> Bead inputs on graph and attach pour

Said by randlee, Proposed from524772f4:ANC-011; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: bead inputs on graph pour

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: d754230a graph.rs/graph_bd.rs graph input cases

## ANC-009

Anchor: Beads attach pour must accept caller-supplied bead variable inputs.

> Bead inputs on graph and attach pour

Said by randlee, Proposed from524772f4:ANC-011; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: bead inputs on attach pour

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: d754230a graph.rs/graph_bd.rs attach input cases

## ANC-010

Anchor: Beads rendering must offer an explicit bd-owned mode that leaves ordinary {{ expression }} text for bd to evaluate.

> bd-owned expressions as an explicit mode

Said by randlee, Proposed from524772f4:ANC-012; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: explicit bd-owned mode

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: bdb3bdae:single_pass_render.rs:bd_mode_keeps_ordinary_expressions_byte_exact

## ANC-011

Anchor: Beads preview-attach must refuse a graph-plan output path occupied by a directory before invoking bd.

> refusing blocked graph plans before bd

Said by randlee, Proposed from524772f4:ANC-013; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: blocked graph path before any bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: 44bcea50:CLI fuzz_171_21_preview_attach_refuses_blocked_graph_plan_before_bd; trace must not exist

## ANC-012

Anchor: A Beads graph-plan output-path failure must identify the failing graph-plan path in the human diagnostic.

> naming plan path failures

Said by randlee, Proposed from524772f4:ANC-015; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: human graph-plan path

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: 44bcea50:CLI fuzz_171_21_preview_attach_refuses_blocked_graph_plan_before_bd

## ANC-013

Anchor: A Beads graph-plan output-path failure must name graph_plan as its JSON error details.field.

> naming plan path failures

Said by randlee, Proposed from524772f4:ANC-015; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: JSON graph-plan field

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: 44bcea50:CLI fuzz_171_21_preview_attach_refuses_blocked_graph_plan_before_bd; cbd26d57:error_json.rs

## ANC-014

Anchor: sc-compose must use docli to generate its CLI documentation.

> we need to add docli for cli documentation generation

Said by randlee, Proposed from524772f4:ANC-003; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: docli generates CLI documentation

Scope: u-6 CLI documentation generation

Deterministic check to pair with the development gate: inspect generator invocation and generated reference source

## ANC-015

Anchor: Every sc-compose publish must run docli.

> docli must be run every publish (stale documentation blocks preflight)

Said by randlee, Proposed from524772f4:ANC-004; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: docli runs every publish

Scope: u-6/u-9 publishing preparation and later release workflow

Deterministic check to pair with the development gate: inspect publish/preflight docli invocation

## ANC-016

Anchor: Stale sc-compose CLI documentation must block release preflight.

> docli must be run every publish (stale documentation blocks preflight)

Said by randlee, Proposed from524772f4:ANC-004; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: stale CLI docs block preflight

Scope: u-6/u-9 preflight configuration

Deterministic check to pair with the development gate: stale generated reference negative preflight test

## ANC-017

Anchor: The Rust CLI archive must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Proposed from524772f4:ANC-001; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: six platforms for Rust CLI archive

Scope: Phase U release artifact platform configuration

Deterministic check to pair with the development gate: compare six platform targets against v1.7.0 manifest, workflows and Go module artifacts

## ANC-018

Anchor: The sc-compose Python wheel must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Proposed from524772f4:ANC-001; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: six platforms for sc-compose Python wheel

Scope: Phase U release artifact platform configuration

Deterministic check to pair with the development gate: compare six platform targets against v1.7.0 manifest, workflows and Go module artifacts

## ANC-019

Anchor: The sc-sha Python wheel must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Proposed from524772f4:ANC-001; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: six platforms for sc-sha Python wheel

Scope: Phase U release artifact platform configuration

Deterministic check to pair with the development gate: compare six platform targets against v1.7.0 manifest, workflows and Go module artifacts

## ANC-020

Anchor: The sc-composer-beads Python wheel must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Proposed from524772f4:ANC-001; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: six platforms for sc-composer-beads Python wheel

Scope: Phase U release artifact platform configuration

Deterministic check to pair with the development gate: compare six platform targets against v1.7.0 manifest, workflows and Go module artifacts

## ANC-021

Anchor: The sc-sha-go Go native module must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Proposed from524772f4:ANC-001; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: six platforms for sc-sha-go Go native module

Scope: Phase U release artifact platform configuration

Deterministic check to pair with the development gate: compare six platform targets against v1.7.0 manifest, workflows and Go module artifacts

## ANC-022

Anchor: Each sc-compose release artifact must be distributed through every channel applicable to that artifact.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Proposed from524772f4:ANC-001; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: applicable artifact channels

Scope: Phase U release channel configuration

Deterministic check to pair with the development gate: artifact/channel table from v1.7.0 release/publish-artifacts.toml; scope table pins artifact identities without inventing channels

## ANC-023

Anchor: CI must run the full install matrix only in the integration test required by release preflight.

> I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint

Said by randlee, Proposed from524772f4:ANC-002; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: full matrix only in integration test

Scope: u-8/u-9 GitHub integration workflow and preflight

Deterministic check to pair with the development gate: inspect triggers and preflight dependency; full install matrix is platform/Python/installer coverage

## ANC-024

Anchor: Sprint CI must not run the full install matrix.

> I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint

Said by randlee, Proposed from524772f4:ANC-002; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: no full matrix in sprint CI

Scope: u-8/u-9 sprint CI workflows

Deterministic check to pair with the development gate: inspect sprint/PR CI for full install matrix invocation

## ANC-025

Anchor: The final Beads regression exercise must verify that previous fuzz test cases have no regression.

> it should have fuzz tests at the end to confirm everything works (and verify previous fuzz tests run have no regression).

Said by randlee, Proposed from524772f4:ANC-018; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: previous fuzz cases at end

Scope: u-13 closing Beads fuzz/regression sprint

Deterministic check to pair with the development gate: replay the preserved historical inputs on final candidate; missing repros remain missing

## ANC-026

Anchor: Beads pouring must be documented in requirements or an architectural decision record.

> pouring clearly documented (or ADR)

Said by randlee, Proposed from524772f4:ANC-020; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 1.

Summary: pouring documented

Scope: u-10/u-11 consuming planning requirements and ADR

Deterministic check to pair with the development gate: FR24/ADR0024 contain operation inputs/output/pour ownership

## ANC-027

Anchor: Phase U must exclude publishing from its sprint work.

> we will publish AFTER phase-u, do not include publishing as part of phase-u.  w/ publish of 1.8.0 assumed, let's continue this discussion

Said by randlee, Proposed; source user transcript line5233, 2026-11-03. Status: proposed. Version 1.

Summary: publish after Phase U

Scope: all Phase U sprints, especially u-1/u-9

Deterministic check to pair with the development gate: no publishing step in Phase U execution

## ANC-028

Anchor: The release following Phase U must be planned as version 1.8.0.

> we will publish AFTER phase-u, do not include publishing as part of phase-u.  w/ publish of 1.8.0 assumed, let's continue this discussion

Said by randlee, Proposed; source user transcript line5233, 2026-11-03. Status: proposed. Version 1.

Summary: next release 1.8.0

Scope: u-1/u-9 version and release preparation

Deterministic check to pair with the development gate: inspect release version targets without publishing

## ANC-029

Anchor: Phase U tests must use controlled inputs so their results do not depend on races, timing, sleeps, event-loop ordering or load.

> Flaky tests are prohibited by design. Any flaky test identified (one that relies on a race, timing, sleeps, event-loop ordering or load) must be re-designed to be deterministic by driving the logic with controlled inputs.

Said by randlee, Proposed; source /Users/randlee/.claude/CLAUDE.md, 2026-11-03. Status: proposed. Version 1.

Summary: deterministic tests

Scope: all test-writing/verification sprints including u-3/u-10/u-12

Deterministic check to pair with the development gate: review relevant tests for controlled deterministic drivers

## ANC-030

Anchor: Changes to Windows-specific code must receive a cargo-xwin Windows cross-build check.

> use xwin

Said by randlee, Proposed; source memory/windows-cross-check-before-ci.md quoted Rand words; scope supplied by that note, 2026-11-03. Status: proposed. Version 1.

Summary: Windows cross-build

Scope: u-4 Windows cross checks and any sprint touching Windows paths

Deterministic check to pair with the development gate: cargo xwin test --target x86_64-pc-windows-msvc --no-run

## ANC-031

Anchor: Phase U lint work must leave lint passing.

> you need to attribute or add the words so lint passes

Said by randlee, Proposed; source memory/lint-must-pass-allowlist-words.md quoted Rand words; narrow original codespell context retained for review, 2026-11-03. Status: proposed. Version 1.

Summary: lint passes

Scope: u-2 lint policy and u-9 aggregate acceptance

Deterministic check to pair with the development gate: just lint on exact candidate; no disabling rules

## ANC-032

Anchor: Windows amd64 integration tests must run on GitHub instead of unavailable local Windows amd64 hardware.

> you can run win amd64 and manylinux on github, but do not add them to the normal ci pipeline runs.

Said by randlee, Proposed; source user transcript5764;5756 states local hardware unavailable, 2026-11-03. Status: proposed. Version 1.

Summary: Windows integration runs on GitHub

Scope: u-8/u-9 full integration test platform execution

Deterministic check to pair with the development gate: inspect integration runner selection

## ANC-033

Anchor: Normal CI pipeline runs must exclude the Windows amd64 and manylinux integration runs.

> you can run win amd64 and manylinux on github, but do not add them to the normal ci pipeline runs.

Said by randlee, Proposed; source user transcript5764, 2026-11-03. Status: proposed. Version 1.

Summary: integration runs outside normal CI

Scope: u-8/u-9 full integration workflow

Deterministic check to pair with the development gate: inspect CI invocation paths; does not ban ordinary Windows unit CI
