# Anchors - phase-u

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-u/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | values stay data | 2026-11-03 |
| ANC-002 | locked | tight values before bd | 2026-11-03 |
| ANC-003 | locked | padded values before bd | 2026-11-03 |
| ANC-004 | locked | loop-local values before bd | 2026-11-03 |
| ANC-005 | proposed | lowercase sees literal value | 2026-11-03 |
| ANC-006 | proposed | length sees literal value | 2026-11-03 |
| ANC-007 | proposed | included default supplies missing value | 2026-11-03 |
| ANC-008 | proposed | bead inputs on graph pour | 2026-11-03 |
| ANC-009 | proposed | bead inputs on attach pour | 2026-11-03 |
| ANC-010 | proposed | explicit bd-owned mode | 2026-11-03 |
| ANC-011 | proposed | blocked graph path before any bd | 2026-11-03 |
| ANC-012 | proposed | human graph-plan path | 2026-11-03 |
| ANC-013 | proposed | JSON graph-plan field | 2026-11-03 |
| ANC-014 | locked | docli generates CLI documentation | 2026-11-03 |
| ANC-015 | locked | docli runs every publish | 2026-11-03 |
| ANC-016 | locked | stale CLI docs block preflight | 2026-11-03 |
| ANC-017 | proposed | six platforms for Rust CLI archive | 2026-11-03 |
| ANC-018 | proposed | six platforms for sc-compose Python wheel | 2026-11-03 |
| ANC-019 | proposed | six platforms for sc-sha Python wheel | 2026-11-03 |
| ANC-020 | proposed | six platforms for sc-composer-beads Python wheel | 2026-11-03 |
| ANC-021 | proposed | six platforms for sc-sha-go Go native module | 2026-11-03 |
| ANC-022 | proposed | applicable artifact channels | 2026-11-03 |
| ANC-023 | locked | full matrix only in integration test | 2026-11-03 |
| ANC-024 | proposed | no full matrix in sprint CI | 2026-11-03 |
| ANC-025 | proposed | previous fuzz cases at end | 2026-11-03 |
| ANC-026 | proposed | pouring documented | 2026-11-03 |
| ANC-027 | proposed | publish after Phase U | 2026-11-03 |
| ANC-028 | proposed | next release 1.8.0 | 2026-11-03 |
| ANC-029 | proposed | deterministic tests | 2026-11-03 |
| ANC-030 | proposed | Windows cross-build | 2026-11-03 |
| ANC-031 | locked | lint passes | 2026-11-03 |
| ANC-032 | retired | Windows integration runs on GitHub | 2026-11-03 |
| ANC-033 | locked | integration runs outside normal CI | 2026-11-03 |
| ANC-034 | proposed | ABI3 Python floor3.11 | 2026-11-03 |
| ANC-035 | proposed | schema-less byte regression | 2026-11-03 |
| ANC-036 | proposed | S01 must pass | 2026-11-03 |
| ANC-037 | proposed | S02 must pass | 2026-11-03 |
| ANC-038 | proposed | S03 must pass | 2026-11-03 |
| ANC-039 | proposed | S04 invalid anyway | 2026-11-03 |
| ANC-040 | proposed | S05 invalid anyway | 2026-11-03 |
| ANC-041 | proposed | S06 invalid anyway | 2026-11-03 |
| ANC-042 | proposed | S07 invalid anyway | 2026-11-03 |
| ANC-043 | proposed | S09 invalid anyway: missing file; non-UTF8 alone not invalid | 2026-11-03 |
| ANC-044 | proposed | S10 must pass: diagnostic transport | 2026-11-03 |
| ANC-045 | proposed | S12a invalid anyway | 2026-11-03 |
| ANC-046 | proposed | S12b invalid anyway | 2026-11-03 |
| ANC-047 | proposed | S13 invalid anyway: external read result, only knowable after bd read | 2026-11-03 |
| ANC-048 | proposed | S14 invalid anyway | 2026-11-03 |
| ANC-049 | proposed | all30 artifact/platform builds | 2026-11-03 |

## ANC-001

Anchor: Values containing {{ }}, {{{ }}} or {% %} must stay data and must never be re-parsed.

> Values containing {{ }}, {{{ }}} or {% %} stay data and are never re-parsed.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 1.

Summary: values stay data

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: supply values containing each delimiter and read back identical literal text from the text passed to bd

## ANC-002

Anchor: When sc-compose renders a Beads formula template, a tight variable expression with no spaces inside its braces, such as {{key}}, must be replaced by its supplied value in the output text.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 3.

Summary: tight values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: render.rs:bead_variables_render_padded_and_tight_placeholders

## ANC-003

Anchor: When sc-compose renders a Beads formula template, a padded variable expression such as {{ key }} must be replaced by its supplied value in the output text.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 3.

Summary: padded values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: render.rs:bead_variables_render_padded_and_tight_placeholders

## ANC-004

Anchor: When sc-compose renders a Beads formula template, a variable expression that refers to a loop-local value must be replaced by that value in the output text.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 3.

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

Anchor: The docli tool must be added to sc-compose as the generator of command-line documentation.

> we need to add docli for cli documentation generation

Said by randlee, Proposed from524772f4:ANC-003; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 3.

Summary: docli generates CLI documentation

Scope: sc-compose command-line documentation generation

Deterministic check to pair with the development gate: inspect generator invocation and generated reference source

## ANC-015

Anchor: The docli command-line documentation generator must run each time sc-compose is published.

> docli must be run every publish (stale documentation blocks preflight)

Said by randlee, Proposed from524772f4:ANC-004; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 3.

Summary: docli runs every publish

Scope: sc-compose publishing preparation and release workflow

Deterministic check to pair with the development gate: inspect publish/preflight docli invocation

## ANC-016

Anchor: Release preflight for sc-compose must fail if the docli tool reports that the command-line documentation is stale.

> docli must be run every publish (stale documentation blocks preflight)

Said by randlee, Proposed from524772f4:ANC-004; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 3.

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

Anchor: Each sc-compose release artifact must be distributed through every channel assigned to that artifact in the approved release channel contract.

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Proposed from524772f4:ANC-001; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: proposed. Version 2.

Summary: applicable artifact channels

Scope: Phase U release artifact distribution. Rand approved via ATM01M6JJ70WRQYV1KCEDZZK6WGJW that applicable channels mean exactly the per-artifact assignments in the approved release channel contract; this clarification is normative alongside the unchanged original quote.

Deterministic check to pair with the development gate: artifact/channel table from v1.7.0 release/publish-artifacts.toml; scope table pins artifact identities without inventing channels

## ANC-023

Anchor: The macOS/Windows/Linux x64/arm64 Rust/wheel/Go matrix, including supported wheel Python versions with pip and uv, must run only in preflight integration tests, excluded from routine sprint continuous integration.

> I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint

Said by randlee, Proposed from524772f4:ANC-002; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 3.

Summary: full matrix only in integration test

Scope: sc-compose integration tests and preflight; Rand approved clarification YES via relay ATM01M6JJ70WRQYV1KCEDZZK6WGJW: full matrix includes all supported artifact/platform cells plus wheel Python-version and pip/uv install combinations; restricted to integration tests required by preflight, excluded from normal sprint continuous integration.

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

Anchor: Phase U Windows-target cross-build and cross-check commands must use the cargo xwin command prefix.

> use xwin

Said by randlee, Proposed; source memory/windows-cross-check-before-ci.md quoted Rand words; scope supplied by that note, 2026-11-03. Status: proposed. Version 2.

Summary: Windows cross-build

Scope: Every Phase U Windows cross-build/check. Rand approved via ATM01M6JJ70WRQYV1KCEDZZK6WGJW that EVERY such build/check uses cargo-xwin; this clarification is normative alongside the unchanged original quote.

Deterministic check to pair with the development gate: cargo xwin test --target x86_64-pc-windows-msvc --no-run

## ANC-031

Anchor: The codespell spelling check for sc-compose must pass.

> you need to attribute or add the words so lint passes

Said by randlee, Proposed; source memory/lint-must-pass-allowlist-words.md quoted Rand words; narrow original codespell context retained for review, 2026-11-03. Status: locked. Version 3.

Summary: lint passes

Scope: The spelling lint check for project-specific words in sc-compose.

Deterministic check to pair with the development gate: just lint on exact candidate; no disabling rules

## ANC-032

Anchor: Windows amd64 integration tests must run on GitHub instead of unavailable local Windows amd64 hardware.

> you can run win amd64 and manylinux on github, but do not add them to the normal ci pipeline runs.

Said by randlee, Proposed; source user transcript5764;5756 states local hardware unavailable, 2026-11-03. Status: retired. Version 2.

Summary: Windows integration runs on GitHub

Scope: u-8/u-9 full integration test platform execution

Deterministic check to pair with the development gate: inspect integration runner selection

## ANC-033

Anchor: Normal CI pipeline runs must exclude the Windows amd64 and manylinux integration runs.

> you can run win amd64 and manylinux on github, but do not add them to the normal ci pipeline runs.

Said by randlee, Proposed; source user transcript5764, 2026-11-03. Status: locked. Version 2.

Summary: integration runs outside normal CI

Scope: u-8/u-9 full integration workflow

Deterministic check to pair with the development gate: inspect CI invocation paths; does not ban ordinary Windows unit CI

## ANC-034

Anchor: The sc-compose ABI3 wheels must support CPython 3.11 and newer.

> I would limit python to 3.11 if macos didn't ship 3.9... it is so out of date

Said by randlee, Lead01M6JAJWE373EJC3EV62AZHE14 explicitly conveys Rand ruling CPython>=3.11; hedge remains verbatim provenance, no lock, 2026-11-03. Status: proposed. Version 1.

Summary: ABI3 Python floor3.11

Scope: u-7/u-8/u-9 wheel ABI and install checks

Deterministic check to pair with the development gate: abi3-py311 tags and requires-python>=3.11; install same wheel on3.12+

## ANC-035

Anchor: Template output without schemas must match sc-compose v1.7.0 byte for byte, allowing only differences recorded in approved-exceptions.json as genuine bug fixes with issue links.

> schema-less templates render byte-identical to v1.7.0 except genuine bugs

Said by randlee, Lead01M6JAKMVZ5RXHGS45A59P45G6 exact new quote; bug exception transcript6183; execution detail01M6JAMQ7ZCF4C8C8K63N01S62/01M6JAN3316GWY4V43QZFK00Z2/01M6JANVK3N4Q4K535KCK9QPG3, 2026-11-03. Status: proposed. Version 2.

Summary: schema-less byte regression

Scope: sc-compose schema-less template byte regression checks; Rand approved clarification YES via relay ATM01M6JJ70WRQYV1KCEDZZK6WGJW: permitted byte differences are issue-linked genuine-bug fixes recorded in the byte-regression approved-exceptions.json list.

Deterministic check to pair with the development gate: one Python script compares fixed corpus stdout bytes against committed v1.7.0 goldens; unlisted difference nonzero; approved exception per genuine bug with issue; no baseline executable in CI/preflight

## ANC-036

Anchor: sc-compose must pass option-like bead identifiers to bd as identifier arguments rather than options.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S01 classification=must pass, 2026-11-03. Status: proposed. Version 1.

Summary: S01 must pass

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_016_option_like_bead_ids_are_never_parsed_as_bd_options; fuzz_016_option_like_relation_sources_are_not_dep_list_options

## ANC-037

Anchor: Printed Beads recovery commands must shell-quote their arguments so literal values survive shell parsing.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S02 classification=must pass, 2026-11-03. Status: proposed. Version 1.

Summary: S02 must pass

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_040_recovery_arguments_are_shell_quoted_and_control_escaped; shell_quote_control_characters_round_trip_under_posix_shells

## ANC-038

Anchor: Printed Beads recovery commands must escape control, zero-width and bidi characters rather than emit them verbatim.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S03 classification=must pass, 2026-11-03. Status: proposed. Version 1.

Summary: S03 must pass

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_040_recovery_arguments_are_shell_quoted_and_control_escaped; character set pinned by this named test, no broader rejection

## ANC-039

Anchor: sc-compose must refuse attach graph references outside [A-Za-z0-9_-]{1,32} before invoking bd.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S04 classification=invalid anyway, 2026-11-03. Status: proposed. Version 1.

Summary: S04 invalid anyway

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_021_invalid_refs_and_relation_steps_keep_typed_errors; fuzz_021_invalid_attach_ref_is_typed_validation_error

## ANC-040

Anchor: sc-compose must refuse relation step identifiers outside [A-Za-z0-9_]{1,64} before invoking bd.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S05 classification=invalid anyway, 2026-11-03. Status: proposed. Version 1.

Summary: S05 invalid anyway

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_021_invalid_refs_and_relation_steps_keep_typed_errors; cooked-step output case is separately discovered after cook

## ANC-041

Anchor: sc-compose must refuse empty parent or relation bead identifiers, or those containing whitespace or control characters, before invoking bd.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S06 classification=invalid anyway, 2026-11-03. Status: proposed. Version 1.

Summary: S06 invalid anyway

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: invalid_parent_and_relation_bead_ids_keep_native_typed_errors;9311ea46 contract.rs;44bcea50 fuzz_171_17_control_char_graph_ids_are_refused_before_bd

## ANC-042

Anchor: sc-compose must refuse a rendered-formula path whose parent is missing or is a file before invoking bd.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S07 classification=invalid anyway, 2026-11-03. Status: proposed. Version 1.

Summary: S07 invalid anyway

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_042_missing_rendered_formula_directory_is_typed; fuzz_042_parent_file_and_relative_output_are_typed

## ANC-043

Anchor: A missing non-UTF8 Beads request-file path must produce BEADS_REQUEST_READ_FAILED instead of failing to format the diagnostic.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S09 classification=invalid anyway: missing file; non-UTF8 alone not invalid, 2026-11-03. Status: proposed. Version 1.

Summary: S09 invalid anyway: missing file; non-UTF8 alone not invalid

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_038_non_utf8_request_paths_are_typed_errors_in_json_and_human_modes; no blanket non-UTF8 refusal

## ANC-044

Anchor: Human Beads attach-refusal diagnostics must escape identifier control characters.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S10 classification=must pass: diagnostic transport, 2026-11-03. Status: proposed. Version 1.

Summary: S10 must pass: diagnostic transport

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_040_human_attach_refusals_escape_identifier_controls_without_changing_json; preserve raw JSON values under existing oracle

## ANC-045

Anchor: Beads request-shape errors must take precedence over graph-identifier errors.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S12a classification=invalid anyway, 2026-11-03. Status: proposed. Version 1.

Summary: S12a invalid anyway

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_052_request_errors_take_precedence_over_invalid_ids (unknown operation, wrong type, missing field)

## ANC-046

Anchor: Beads pour-authorization errors must take precedence over graph-identifier errors.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S12b classification=invalid anyway, 2026-11-03. Status: proposed. Version 1.

Summary: S12b invalid anyway

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: fuzz_052_request_errors_take_precedence_over_invalid_ids (unauthorized attach)

## ANC-047

Anchor: Dependency types read from bd must match [A-Za-z][A-Za-z0-9_-]* before sc-compose plans graph mutations.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S13 classification=invalid anyway: external read result, only knowable after bd read, 2026-11-03. Status: proposed. Version 1.

Summary: S13 invalid anyway: external read result, only knowable after bd read

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: graph.rs:dependency_read_rejects_unsafe_type_tokens_before_planning; three read calls already occurred, no mutation

## ANC-048

Anchor: sc-compose must refuse a rendered-formula destination equal to the input template before invoking bd.

> invalid graph is fine to fail fast. I do not want inputs overly scrutinized and things that would actually pass to fail fast.

Said by randlee, Lead01M6JAYXV21RZ3X8WSKWJC2RKC supplies Rand quote and existing-tests-only constraint; S14 classification=invalid anyway, 2026-11-03. Status: proposed. Version 1.

Summary: S14 invalid anyway

Scope: Beads request validation, bd argv/recovery output and graph planning

Deterministic check to pair with the development gate: 9311ea46 execute.rs:validate_request rendered_formula==template; no source overwrite

## ANC-049

Anchor: All 30 release artifact/platform combinations must build successfully: five artifacts (Rust CLI, three Python wheels, Go native module) × six targets (macOS, Windows, Linux × x64, arm64).

> support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels

Said by randlee, Meaning approved by ATM01M6JJ70WRQYV1KCEDZZK6WGJW; remains proposed because rubric is inconclusive., 2026-11-03. Status: proposed. Version 1.

Summary: all30 artifact/platform builds

Scope: Phase U release builds. Rand approved via ATM01M6JJ70WRQYV1KCEDZZK6WGJW that support requires actual successful builds of Rust CLI + three Python wheels + Go native module on macOS/Windows/Linux x64/arm64, all 30 cells; this clarification is normative alongside the unchanged original quote.

Deterministic check to pair with the development gate: Compare actual successful build outputs against all30 required artifact/platform cells, no missing cell passes.
