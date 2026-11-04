# Phase U proposed anchors — source findings and unresolved wording

Draft only. ANC-001 is the unchanged locked develop record; ANC-002..033 are proposed. No lock, push, merge, question override or tracker import performed. The current branch is based on origin/develop@9a39bc1b. The twenty-anchor524772f4 draft is provenance only: its locked labels were not copied as approval.

## Boundaries from Rand's later corrections

Transcript8766 rejects the sc-publish extension-point anchor as another repo, rejects original017 sequencing and019 pair-programming anchors, and questions016 (bug observation). Those are omitted. Original014 safety is held separately until the lead confirms Rand's clarification. Original008 is the current locked ANC-001 and is not duplicated. Cross-platform Python test-tooling is a stated preference, but this brief did not authorize expanding it into an extra anchor.

## Recovered definitions; no new behavior invented

- **Include defaults:** `bdb3bdae:crates/sc-composer-beads/tests/single_pass_render.rs:fuzz_171_11_include_defaults_reach_the_render_context` proves an included fragment's `defaults.g=G` supplies missing g. `d754230a:render.rs:frontmatter_defaults_apply_and_caller_values_win` proves caller owner=ada overrides fallback owner=team. Library `validation/state.rs:collect_validation_state_characterizes_default_merge_precedence` and `validation/mod.rs:include_derived_defaults_and_required_variables_merge` pin context precedence/merged include defaults. The single quote “Include defaults.” does not itself state those policies: recovered behavior is cited as the meaning source per lead01M6J9Y67Y1KHHMQJ85HDSCGAZ, not substituted for a new Rand quote. No new root-vs-child collision policy invented.
- **Blocked plan timing:** `44bcea50:crates/sc-compose/tests/cli/fuzz_regressions.rs:fuzz_171_21_preview_attach_refuses_blocked_graph_plan_before_bd` creates a directory at `out.formula.toml.graph.json`, then asserts bd trace absent. This local failure is before ANY bd call, not merely before mutation. Generic output readiness-before-create tests are distinct.
- **Path naming:** the same test pins human “invalid graph plan output path” plus actual path and JSON `payload.error.details.field=graph_plan`; `cbd26d57:error_json.rs` selects graph_plan for `.graph.json` output paths, with details.value/rule and graph-plan recovery text. Other paths use rendered_formula. No arbitrary new field or code invented.
- **bd-owned mode:** `bdb3bdae:single_pass_render.rs:bd_mode_keeps_ordinary_expressions_byte_exact` pins ordinary expressions reaching bd unchanged while triple expressions render; the source raw wrapper is consumed. This is rendering-mode meaning, not permission for d754230a's required public request field or changed default, both excluded by compatibility requirements.
- **Graph/attach inputs remain a real gap:** the recovered `d754230a:tests/graph.rs:graph_pour_forbids_runtime_variables_and_registry_pour_forbids_relations` explicitly refuses graph bead_variables. Current attach forbids them too. New quote requires support; do not claim old code already meets it or auto-adopt a replacement public API.

## Artifact by channel (v1.7.0 source, not newly run publication)

Source: `v1.7.0:release/publish-artifacts.toml`, `release/go-native-module.toml`, `.github/workflows/release.yml`, `.github/scripts/go_native_module.py`; platform-matrix-anchor memory note supplies the known shipped-cell inventory.

| Artifact | Existing applicable channels | Baseline platform gaps against six platforms |
|---|---|---|
| Rust CLI archive | GitHub Release; Homebrew; winget; Scoop | Archive: Windows arm64 absent. Homebrew: Linux arm64 absent (macOS x64/arm64 and Linux x64 present). winget/Scoop: Windows arm64 absent (x64 present). |
| sc-compose Python wheel | PyPI; TestPyPI rehearsal; GitHub release assets | macOS x64 and Windows arm64 absent; four platforms present. |
| sc-sha Python wheel | PyPI; TestPyPI rehearsal; GitHub release assets | macOS x64, Linux arm64, Windows arm64 absent; three platforms present. |
| sc-composer-beads Python wheel | PyPI; TestPyPI rehearsal; GitHub release assets | macOS x64, Linux arm64, Windows arm64 absent; three platforms present. |
| sc-sha-go native module bundle | GitHub Release target-specific ZIP module bundles | macOS x64, Linux arm64, Windows arm64 absent; macOS arm64/Linux x64/Windows x64-gnu present. No new Go registry channel inferred. |

The v1.7.0 Rust crates also publish to crates.io, a source-package channel independent of platform binary cells. No Homebrew/winget/Scoop delivery inferred for Python or Go. TestPyPI is rehearsal, not a consumer production channel. Five distinct Rust CLI OS/architecture cells plus10 wheel cells plus3 Go cells =18/30 baseline cells; Windows GNU/MSVC variants do not count as different architectures.12 platform/artifact cells remain missing. Approved CPython>=3.9 ABI3 floor excludes Python<=3.8; baseline cp311-cp311 wheels also exclude3.9/3.10/3.12+ binary install compatibility. The intent says remove Python-version/platform limits but does not itself supersede the stated>=3.9 floor; report the exclusion rather than promise universal support.

## Sprint coverage / quote gaps

| Sprint with no prior anchor | Proposed applicability | Source / limit |
|---|---|---|
| u-1 | no Phase U publication; next version1.8.0 | transcript5233, exact quote retained |
| u-2 | lint passes | memory literal “you need to attribute or add the words so lint passes”; originally codespell words, so the broader scope is still a proposed reading |
| u-3 | deterministic test outcomes | ~/.claude/CLAUDE.md exact standing instruction; verification does not recreate the completed refactors |
| u-4 | Windows xwin build | memory exact quote “use xwin”, original surrounding requirement is in that note |
| u-9 | no publishing; full-matrix integration/preflight boundary | transcript5233/8644 |
| u-10 | deterministic tests; rejected strict-only defaults remain prohibited by retained compatibility authority | no separately recovered verbatim schema-less-byte-identity quote; memory note contains the policy as prose, not a Rand quotation |
| u-12 | deterministic tests; v1.7.0 byte differential governed by existing D7 | **no quote** found for the exact schema-less-byte-identity wording in the named transcript; memory prose and FR24 remain referenced authority, not fabricated quote |

Windows hardware fact is transcript5756; hosting/normal-CI limit is transcript5764. A literal “workflow_dispatch only” Rand quote was not found in the named transcript; existing plan and platform note specify it, so no new direct-quote anchor claims that phrase. Genuine-bug exception is exact transcript6183; it is an exception to comparison, not permission for arbitrary backward incompatibility.

## Beads intent trace

| Proposed anchors | Trace to Rand's intent |
|---|---|
| ANC-001..007 (literals, variables, filters, defaults) | avoid bd rendering bugs without consumer repair scripts |
| ANC-008..010 (graph/attach inputs and explicit native mode) | consumer pouring without custom scripts; choose native ownership explicitly |
| ANC-011..013 (local path refusal/diagnostics) | failures reported by the pouring API instead of requiring consumer diagnosis scripts; safety complements intent rather than fixing bd rendering itself |
| ANC-025 previous fuzz regression | avoid returning render defects to consumers |
| ANC-026 pouring docs | consumers can use the API instead of maintaining scripts |
| ANC-017..022 artifact/channel coverage | phase-wide Python/platform intent |

No runtime capability or successful install/pour is claimed by this draft.

## Initial live rubric and wording options for Rand

Exactly one statement submission per anchor; each submission uses the instrument's built-in two confirmations (third only for disagreement). No retry/rewording to force a PASS; maximum3 statement tries remains unspent. Results:3 testable including unchanged ANC-001,30 inconclusive,0 needs_work/unstable. Low confidence never counts as a pass. Raw stdout/stderr lives in the ordinary local report directory `/Users/randlee/.codex/reports/phase-u-anchor-candidates-20261104/`; not a new evidence service. Independent plan-free rubric calibration is separate from approving these statements.

The rubric API supplies choices/confidence/criteria, not a free-text narrative rationale. Below, “rationale” reports those actual roles/choices and the sealed criterion; it does not invent an explanation. Options are for Rand to clarify wording, not applied edits or requests to get a passing verdict.

### ANC-002 — tight values before bd

Quote: `Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.`

Rationale: self_contained: no@0.7, no@0.68; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record. observable: unclear@0.1, unclear@0.06; criterion: Some evidence is named, but the observation needed to settle compliance remains ambiguous. faithful: yes@0.5, yes@0.64; criterion: Those normalization rules are sufficient and no substantive requirement was invented.

Option A (retain the source-defined interpretation): In advanced Beads rendering, tight {{key}} variable expressions must be rendered before bd receives the formula.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Before invoking bd, sc-compose must replace source {{key}} expressions with their bound values.

### ANC-003 — padded values before bd

Quote: `Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.`

Rationale: observable: yes@0.18, yes@0.13; criterion: It names evidence or behavior that can be inspected with a determinate outcome. self_contained: no@0.5, no@0.58; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): In advanced Beads rendering, padded {{ key }} variable expressions must be rendered before bd receives the formula.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Before invoking bd, sc-compose must replace source {{key}} expressions with their bound values.

### ANC-004 — loop-local values before bd

Quote: `Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.`

Rationale: self_contained: no@0.68, no@0.64; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record. observable: unclear@0.04, no@0.11; criterion: Some evidence is named, but the observation needed to settle compliance remains ambiguous. faithful: yes@0.4, yes@0.37; criterion: Those normalization rules are sufficient and no substantive requirement was invented.

Option A (retain the source-defined interpretation): In advanced Beads rendering, variable expressions using the current loop item must be rendered before bd receives the formula.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Before invoking bd, sc-compose must replace source {{key}} expressions with their bound values.

### ANC-005 — lowercase sees literal value

Quote: `Filters act on the real value, so A{BC lowercases to a{bc and has length 4.`

Rationale: self_contained: yes@0.7, yes@0.55; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): In advanced Beads rendering, applying the lowercase filter to the value A{BC must produce a{bc.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Beads lowercase filter must map the input string A{BC to a{bc.

### ANC-007 — included default supplies missing value

Quote: `Include defaults.`

Rationale: observable: yes@0.55, yes@0.64; criterion: It names evidence or behavior that can be inspected with a determinate outcome. faithful: no@0.33, no@0.28; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. self_contained: yes@0.11, yes@0.09; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): In advanced Beads rendering, an included fragment's frontmatter default must supply an otherwise missing variable value.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Beads render context must include a fragment's declared default when the caller supplies no value for that name.

### ANC-008 — bead inputs on graph pour

Quote: `Bead inputs on graph and attach pour`

Rationale: faithful: yes@0.26, no@0.3; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: yes@0.61, yes@0.58; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Graph-based Beads pour must accept caller-supplied bead variable inputs.

Option B (alternate wording for Rand approval; not applied or rubric-tested): A caller of graph-based Beads pour must be able to supply bead variable values to sc-compose.

### ANC-009 — bead inputs on attach pour

Quote: `Bead inputs on graph and attach pour`

Rationale: faithful: no@0.48, no@0.38; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. self_contained: yes@0.4, yes@0.31; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Beads attach pour must accept caller-supplied bead variable inputs.

Option B (alternate wording for Rand approval; not applied or rubric-tested): A caller of graph-based Beads pour must be able to supply bead variable values to sc-compose.

### ANC-010 — explicit bd-owned mode

Quote: `bd-owned expressions as an explicit mode`

Rationale: faithful: yes@0.37, yes@0.37; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: no@0.37, no@0.38; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): Beads rendering must offer an explicit bd-owned mode that leaves ordinary {{ expression }} text for bd to evaluate.

Option B (alternate wording for Rand approval; not applied or rubric-tested): sc-compose must expose a selectable rendering mode in which ordinary double-brace expressions remain bd-owned.

### ANC-011 — blocked graph path before any bd

Quote: `refusing blocked graph plans before bd`

Rationale: faithful: no@0.3, no@0.24; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. self_contained: yes@0.31, yes@0.42; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Beads preview-attach must refuse a graph-plan output path occupied by a directory before invoking bd.

Option B (alternate wording for Rand approval; not applied or rubric-tested): If the preview-attach graph-plan destination is a directory, sc-compose must reject it without starting bd.

### ANC-012 — human graph-plan path

Quote: `naming plan path failures`

Rationale: observable: yes@0.48, yes@0.46; criterion: It names evidence or behavior that can be inspected with a determinate outcome. faithful: no@0.48, no@0.46; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. self_contained: no@0.35, no@0.26; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): A Beads graph-plan output-path failure must identify the failing graph-plan path in the human diagnostic.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The human preview-attach refusal must print the path of a blocked graph-plan output.

### ANC-013 — JSON graph-plan field

Quote: `naming plan path failures`

Rationale: faithful: no@0.74, no@0.73; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. self_contained: yes@0.43, yes@0.46; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): A Beads graph-plan output-path failure must name graph_plan as its JSON error details.field.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The human preview-attach refusal must print the path of a blocked graph-plan output.

### ANC-014 — docli generates CLI documentation

Quote: `we need to add docli for cli documentation generation`

Rationale: observable: yes@0.64, yes@0.55; criterion: It names evidence or behavior that can be inspected with a determinate outcome. self_contained: yes@0.28, yes@0.28; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): sc-compose must use docli to generate its CLI documentation.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose CLI reference generator must invoke docli.

### ANC-015 — docli runs every publish

Quote: `docli must be run every publish (stale documentation blocks preflight)`

Rationale: faithful: yes@0.58, yes@0.55; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: no@0.52, no@0.58; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): Every sc-compose publish must run docli.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Each sc-compose publication must execute docli.

### ANC-016 — stale CLI docs block preflight

Quote: `docli must be run every publish (stale documentation blocks preflight)`

Rationale: observable: no@0.28, no@0.3; criterion: It supplies only a subjective quality, undefined shorthand, or an approval with no observable record. self_contained: no@0.62, no@0.56; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): Stale sc-compose CLI documentation must block release preflight.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Each sc-compose publication must execute docli.

### ANC-017 — six platforms for Rust CLI archive

Quote: `support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels`

Rationale: faithful: yes@0.38, yes@0.31; criterion: Those normalization rules are sufficient and no substantive requirement was invented.

Option A (retain the source-defined interpretation): The Rust CLI archive must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Rust CLI archive must cover both x64 and arm64 on each of macOS, Windows and Linux.

### ANC-018 — six platforms for sc-compose Python wheel

Quote: `support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels`

Rationale: faithful: yes@0.44, yes@0.61; criterion: Those normalization rules are sufficient and no substantive requirement was invented.

Option A (retain the source-defined interpretation): The sc-compose Python wheel must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Rust CLI archive must cover both x64 and arm64 on each of macOS, Windows and Linux.

### ANC-019 — six platforms for sc-sha Python wheel

Quote: `support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels`

Rationale: faithful: yes@0.56, yes@0.55; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: no@0.26, no@0.19; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): The sc-sha Python wheel must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Rust CLI archive must cover both x64 and arm64 on each of macOS, Windows and Linux.

### ANC-020 — six platforms for sc-composer-beads Python wheel

Quote: `support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels`

Rationale: faithful: yes@0.61, yes@0.55; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: yes@0.42, yes@0.55; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): The sc-composer-beads Python wheel must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Rust CLI archive must cover both x64 and arm64 on each of macOS, Windows and Linux.

### ANC-021 — six platforms for sc-sha-go Go native module

Quote: `support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels`

Rationale: faithful: yes@0.56, yes@0.48; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: no@0.25, no@0.26; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): The sc-sha-go Go native module must support macOS x64, macOS arm64, Windows x64, Windows arm64, Linux x64 and Linux arm64.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Rust CLI archive must cover both x64 and arm64 on each of macOS, Windows and Linux.

### ANC-022 — applicable artifact channels

Quote: `support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels`

Rationale: observable: unclear@0.26, unclear@0.35; criterion: Some evidence is named, but the observation needed to settle compliance remains ambiguous. faithful: yes@0.58, yes@0.68; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: yes@0.16, yes@0.09; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Each sc-compose release artifact must be distributed through every channel applicable to that artifact.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The sc-compose Rust CLI archive must cover both x64 and arm64 on each of macOS, Windows and Linux.

### ANC-023 — full matrix only in integration test

Quote: `I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint`

Rationale: observable: yes@0.66, yes@0.5; criterion: It names evidence or behavior that can be inspected with a determinate outcome. self_contained: yes@0.34, yes@0.4; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): CI must run the full install matrix only in the integration test required by release preflight.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Only the preflight-required integration test may execute all declared platform, Python-version and installer combinations.

### ANC-024 — no full matrix in sprint CI

Quote: `I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint`

Rationale: self_contained: yes@0.43, yes@0.46; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Sprint CI must not run the full install matrix.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Only the preflight-required integration test may execute all declared platform, Python-version and installer combinations.

### ANC-025 — previous fuzz cases at end

Quote: `it should have fuzz tests at the end to confirm everything works (and verify previous fuzz tests run have no regression).`

Rationale: single_constraint: yes@0.44, yes@0.3; criterion: All details enforce the same invariant and would be evaluated together. observable: yes@0.43, yes@0.42; criterion: It names evidence or behavior that can be inspected with a determinate outcome. self_contained: yes@0.44, yes@0.37; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): The final Beads regression exercise must verify that previous fuzz test cases have no regression.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The closing Beads fuzz exercise must replay the earlier fuzz regression cases without regressions.

### ANC-026 — pouring documented

Quote: `pouring clearly documented (or ADR)`

Rationale: faithful: yes@0.46, yes@0.52; criterion: Those normalization rules are sufficient and no substantive requirement was invented. self_contained: no@0.43, no@0.48; criterion: At least one decisive term is an undefined quality or private label, or compliance depends on an approval with no identified record.

Option A (retain the source-defined interpretation): Beads pouring must be documented in requirements or an architectural decision record.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The requirements or an ADR must describe how consumers pour beads with sc-compose.

### ANC-027 — publish after Phase U

Quote: `we will publish AFTER phase-u, do not include publishing as part of phase-u.  w/ publish of 1.8.0 assumed, let's continue this discussion`

Rationale: self_contained: yes@0.44, yes@0.6; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Phase U must exclude publishing from its sprint work.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Phase U must finish before publication occurs.

### ANC-028 — next release 1.8.0

Quote: `we will publish AFTER phase-u, do not include publishing as part of phase-u.  w/ publish of 1.8.0 assumed, let's continue this discussion`

Rationale: single_constraint: yes@0.56, yes@0.74; criterion: All details enforce the same invariant and would be evaluated together. faithful: no@0.26, no@0.2; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. self_contained: yes@0.48, yes@0.48; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): The release following Phase U must be planned as version 1.8.0.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Phase U must finish before publication occurs.

### ANC-029 — deterministic tests

Quote: `Flaky tests are prohibited by design. Any flaky test identified (one that relies on a race, timing, sleeps, event-loop ordering or load) must be re-designed to be deterministic by driving the logic with controlled inputs.`

Rationale: observable: yes@0.62, yes@0.5; criterion: It names evidence or behavior that can be inspected with a determinate outcome. self_contained: yes@0.46, yes@0.46; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Phase U tests must use controlled inputs so their results do not depend on races, timing, sleeps, event-loop ordering or load.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Test logic must be driven by controlled inputs rather than depend on races, elapsed time, sleeps, event-loop ordering or load.

### ANC-030 — Windows cross-build

Quote: `use xwin`

Rationale: faithful: no@0.62, no@0.61; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. observable: yes@0.25, yes@0.18; criterion: It names evidence or behavior that can be inspected with a determinate outcome. self_contained: yes@0.37, yes@0.3; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Changes to Windows-specific code must receive a cargo-xwin Windows cross-build check.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The local Windows build check must use cargo xwin.

### ANC-031 — lint passes

Quote: `you need to attribute or add the words so lint passes`

Rationale: observable: yes@0.56, yes@0.55; criterion: It names evidence or behavior that can be inspected with a determinate outcome. self_contained: yes@0.25, yes@0.22; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Phase U lint work must leave lint passing.

Option B (alternate wording for Rand approval; not applied or rubric-tested): Legitimate words flagged by codespell must be attributed or allowlisted so lint passes.

### ANC-032 — Windows integration runs on GitHub

Quote: `you can run win amd64 and manylinux on github, but do not add them to the normal ci pipeline runs.`

Rationale: faithful: no@0.74, no@0.74; criterion: Even after ordinary normalization, the statement requires a new policy, implementation choice, strength, trigger, or outcome. single_constraint: yes@0.66, yes@0.56; criterion: All details enforce the same invariant and would be evaluated together. self_contained: yes@0.5, yes@0.46; criterion: None of those outside-context dependencies is needed; ordinary technical terms, concrete prohibitions, lists, and explicit input/action/outcome conditions are sufficient.

Option A (retain the source-defined interpretation): Windows amd64 integration tests must run on GitHub instead of unavailable local Windows amd64 hardware.

Option B (alternate wording for Rand approval; not applied or rubric-tested): The Windows amd64 integration run must use a GitHub runner.

## Independent calibration result

The isolated anchor-tuning agent evaluated3 complete candidate rubrics (15 role wordings) against the frozen17-case corpus. All85 role cells matched the oracle with stable repeats, but the best candidate had only84/85 cells with both confirmations>=0.6; one faithful cell scored0.52/0.62. **Not accepted; no rubric sealed.** Four dedupe cases and8 deterministic lint cases matched their expected outcomes. Both result schemas, fenced-envelope validation and relational result validation passed; that validates the report contract, not the rubric.110 direct live calls are retained; no server run IDs were returned, so local correlation IDs are explicitly distinguished. No thresholds or production questions were changed. Per-anchor plan/development question tuning cannot use these unapproved proposed anchors as locked authority. Safety S01-S14 remains untuned pending the lead's confirmation. This is an honest nonpassing handoff, not a passing set or product test result.
