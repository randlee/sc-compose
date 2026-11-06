# Anchors - phase-v

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-v/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | values stay data | 2026-11-03 |
| ANC-002 | locked | tight values before bd | 2026-11-03 |
| ANC-003 | locked | padded values before bd | 2026-11-03 |
| ANC-004 | locked | loop-local values before bd | 2026-11-03 |
| ANC-005 | proposed | lowercase sees literal value | 2026-11-03 |
| ANC-006 | proposed | length sees literal value | 2026-11-03 |
| ANC-007 | proposed | included default supplies missing value | 2026-11-03 |
| ANC-008 | retired | bead inputs on graph pour | 2026-11-03 |
| ANC-009 | retired | bead inputs on attach pour | 2026-11-03 |
| ANC-010 | retired | explicit bd-owned mode | 2026-11-03 |
| ANC-011 | retired | blocked graph path before any bd | 2026-11-03 |
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
| ANC-035 | retired | schema-less byte regression | 2026-11-03 |
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
| ANC-049 | retired | all30 artifact/platform builds | 2026-11-03 |
| FR-24.8 | retired | FR-24.8 approved replacement | 2026-11-03 |
| FR-24.7 | retired | FR-24.7 approved replacement | 2026-11-03 |
| FR-24.10 | retired | FR-24.10 approved replacement | 2026-11-03 |
| FR-24.2 | retired | FR-24.2 approved replacement | 2026-11-03 |
| FR-25.1 | locked | FR-25.1 approved replacement | 2026-11-03 |
| FR-24.13 | retired | Approved Beads requirement 1 | 2026-11-04 |
| FR-24.14 | retired | Approved Beads requirement 2 | 2026-11-04 |
| FR-24.15 | retired | Approved Beads requirement 3 | 2026-11-04 |
| FR-24.19 | retired | Approved Beads requirement 7 | 2026-11-04 |
| FR-24.20 | retired | Approved Beads requirement 8 | 2026-11-04 |
| FR-24.21 | retired | Approved Beads requirement 9 | 2026-11-04 |
| FR-24.22 | retired | Approved Beads requirement 11 | 2026-11-04 |
| FR-24.23 | retired | Approved Beads requirement 12 | 2026-11-04 |
| FR-24.24 | proposed | Approved Beads requirement 13 | 2026-11-04 |
| FR-24.25 | retired | Approved Beads requirement 15 | 2026-11-04 |
| FR-24.26 | retired | Approved Beads requirement 16 | 2026-11-04 |
| FR-24.27 | locked | Approved Beads requirement 19 | 2026-11-04 |
| FR-24.28 | retired | Approved Beads requirement 20 | 2026-11-04 |
| FR-24.29 | retired | Approved Beads requirement 21 | 2026-11-04 |
| FR-24.30 | retired | Approved Beads requirement 22 | 2026-11-04 |
| FR-24.31 | retired | Approved Beads requirement 23 | 2026-11-04 |
| FR-24.32 | proposed | Approved Beads requirement 24 | 2026-11-04 |
| FR-24.13a | locked | Approved Beads requirement 1 | 2026-11-04 |
| FR-24.14a | locked | Approved Beads requirement 2 | 2026-11-04 |
| FR-24.15a | locked | Approved Beads requirement 3 | 2026-11-04 |
| FR-24.19a | locked | Approved Beads requirement 7 | 2026-11-04 |
| FR-24.21b | locked | Approved Beads requirement 9b | 2026-11-04 |
| FR-24.8a | locked | Approved Beads requirement 10a | 2026-11-04 |
| FR-24.8c | locked | Approved Beads requirement 10c | 2026-11-04 |
| FR-24.22a | locked | Approved Beads requirement 11a | 2026-11-04 |
| FR-24.2a | locked | Approved Beads requirement 14a | 2026-11-04 |
| FR-24.2b | retired | Approved Beads requirement 14b | 2026-11-04 |
| FR-24.2d | locked | Approved Beads requirement 14d | 2026-11-04 |
| FR-24.26b | locked | Approved Beads requirement 16b | 2026-11-04 |
| FR-24.7a | locked | Approved Beads requirement 17a | 2026-11-04 |
| FR-24.7b | locked | Approved Beads requirement 17b | 2026-11-04 |
| FR-24.10a | locked | Approved Beads requirement 18 | 2026-11-04 |
| FR-24.28b | locked | Approved Beads requirement 20b | 2026-11-04 |
| FR-24.29b | locked | Approved Beads requirement 21b | 2026-11-04 |
| FR-24.20a | locked | Approved Beads requirement 8 | 2026-11-04 |
| FR-24.20b | locked | Approved Beads requirement 8b | 2026-11-04 |
| FR-24.20c | locked | Approved Beads requirement 8c | 2026-11-04 |
| FR-24.21a | locked | Approved Beads requirement 9a | 2026-11-04 |
| FR-24.22b | locked | Approved Beads requirement 11b | 2026-11-04 |
| FR-24.22c | locked | Approved Beads requirement 11c | 2026-11-04 |
| FR-24.22d | locked | Approved Beads requirement 11d | 2026-11-04 |
| FR-24.23b | locked | Approved Beads requirement 12b | 2026-11-04 |
| FR-24.13b | locked | Approved Beads requirement 1b | 2026-11-04 |
| FR-24.2c | locked | Approved Beads requirement 14c | 2026-11-04 |
| FR-24.2f | locked | Approved Beads requirement 14f | 2026-11-04 |
| FR-24.25a | locked | Approved Beads requirement 15 | 2026-11-04 |
| FR-24.23a | locked | Approved Beads requirement 12a | 2026-11-04 |
| FR-24.2e | locked | Approved Beads requirement 14e | 2026-11-04 |
| FR-24.26a | locked | Approved Beads requirement 16a | 2026-11-04 |
| FR-24.28a | locked | Approved Beads requirement 20a | 2026-11-04 |
| FR-24.8b | locked | Approved Beads requirement 10b | 2026-11-04 |
| FR-24.8d | locked | Approved Beads requirement 10d | 2026-11-04 |
| FR-24.24b | locked | Approved Beads requirement 13b | 2026-11-04 |
| FR-24.30a | locked | Approved Beads requirement 22 | 2026-11-04 |
| FR-24.31a | locked | Approved Beads requirement 23 | 2026-11-04 |
| FR-24.32a | locked | Approved Beads requirement 24a | 2026-11-04 |
| FR-24.24c | locked | Approved Beads requirement 13c | 2026-11-04 |
| FR-24.29a | locked | Approved Beads requirement 21a | 2026-11-04 |

## ANC-001

Anchor: Values containing {{ }}, {{{ }}} or {% %} must stay data and must never be re-parsed.

> Values containing {{ }}, {{{ }}} or {% %} stay data and are never re-parsed.

Said by randlee, Phase U planning, 2026-11-04, 2026-11-03. Status: locked. Version 2.

Summary: values stay data

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Deterministic check to pair with the development gate: supply values containing each delimiter and read back identical literal text from the text passed to bd

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-jvn`, plan questions `comp-jvn.1`, dev questions `comp-jvn.2`

Fixtures:
- pass: Values containing {{ name }}, {{{ name }}} or {% if flag %} are inserted as literal data; the renderer never parses inserted values as template source.
- fail: After inserting supplied values, the renderer parses the resulting output as a new template so {{ name }} inside a value is evaluated.

## ANC-002

Anchor: When sc-compose renders a Beads formula template, a tight variable expression with no spaces inside its braces, such as {{key}}, must be replaced by its supplied value in the output text.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 4.

Summary: tight values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: render.rs:bead_variables_render_padded_and_tight_placeholders

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 builds and releases artifacts and does not implement it.

Beads: anchor `comp-w29`, plan questions `comp-w29.1`, dev questions `comp-w29.2`

Fixtures:
- pass: For a Beads formula containing tight {{key}} and the supplied value key=ALPHA, sc-compose writes ALPHA into the rendered formula text before handing it to bd.
- fail: For tight {{key}} with key=ALPHA, sc-compose leaves {{key}} unchanged in the output formula and delegates its substitution to bd.

## ANC-003

Anchor: When sc-compose renders a Beads formula template, a padded variable expression such as {{ key }} must be replaced by its supplied value in the output text.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 4.

Summary: padded values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: render.rs:bead_variables_render_padded_and_tight_placeholders

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 builds and releases artifacts and does not implement it.

Beads: anchor `comp-i2f`, plan questions `comp-i2f.1`, dev questions `comp-i2f.2`

Fixtures:
- pass: For a Beads formula containing padded {{ key }} and the supplied value key=ALPHA, sc-compose writes ALPHA into the rendered output text.
- fail: The renderer replaces only tight {{key}}; padded {{ key }} remains unchanged even when key=ALPHA is supplied.

## ANC-004

Anchor: When sc-compose renders a Beads formula template, a variable expression that refers to a loop-local value must be replaced by that value in the output text.

> Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.

Said by randlee, Proposed from524772f4:ANC-007; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 4.

Summary: loop-local values before bd

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: bdb3bdae single_pass_render.rs FUZZ-171-03

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 builds and releases artifacts and does not implement it.

Beads: anchor `comp-54p`, plan questions `comp-54p.1`, dev questions `comp-54p.2`

Fixtures:
- pass: In a formula loop over items named Build and Test, sc-compose replaces each expression referring to the current loop-local name with Build or Test respectively in that iteration’s output.
- fail: Formula loops are expanded while loop-local variable expressions are left in the output; the local bindings are discarded before those expressions are rendered.

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

Said by randlee, Proposed from524772f4:ANC-011; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: retired. Version 2.

Summary: bead inputs on graph pour

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: d754230a graph.rs/graph_bd.rs graph input cases

## ANC-009

Anchor: Beads attach pour must accept caller-supplied bead variable inputs.

> Bead inputs on graph and attach pour

Said by randlee, Proposed from524772f4:ANC-011; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: retired. Version 2.

Summary: bead inputs on attach pour

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: d754230a graph.rs/graph_bd.rs attach input cases

## ANC-010

Anchor: Beads rendering must offer an explicit bd-owned mode that leaves ordinary {{ expression }} text for bd to evaluate.

> bd-owned expressions as an explicit mode

Said by randlee, Proposed from524772f4:ANC-012; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: retired. Version 2.

Summary: explicit bd-owned mode

Scope: sc-composer-beads rendering and sc-compose bead render/pour/attach commands

Deterministic check to pair with the development gate: bdb3bdae:single_pass_render.rs:bd_mode_keeps_ordinary_expressions_byte_exact

## ANC-011

Anchor: Beads preview-attach must refuse a graph-plan output path occupied by a directory before invoking bd.

> refusing blocked graph plans before bd

Said by randlee, Proposed from524772f4:ANC-013; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: retired. Version 2.

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

Said by randlee, Proposed from524772f4:ANC-003; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 4.

Summary: docli generates CLI documentation

Scope: sc-compose command-line documentation generation

Deterministic check to pair with the development gate: inspect generator invocation and generated reference source

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-4 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-8 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-3`: Phase V assigns adding docli to comp-v-8 (comp-v-8 satisfies ANC-014 at 0.96); comp-v-3 does not touch command-line documentation generation.
- `comp-v-4`: Phase V assigns adding docli to comp-v-8 (comp-v-8 satisfies ANC-014 at 0.96); comp-v-4 only exports the command model that docli consumes.

Beads: anchor `comp-bw1`, plan questions `comp-bw1.1`, dev questions `comp-bw1.2`

Fixtures:
- pass: Add the docli tool to sc-compose and invoke it as the generator of the command-line documentation.
- fail: Generate all sc-compose command-line documentation using a private generator; do not add or invoke the docli tool.

## ANC-015

Anchor: The docli command-line documentation generator must run each time sc-compose is published.

> docli must be run every publish (stale documentation blocks preflight)

Said by randlee, Proposed from524772f4:ANC-004; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 4.

Summary: docli runs every publish

Scope: sc-compose publishing preparation and release workflow

Deterministic check to pair with the development gate: inspect publish/preflight docli invocation

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-8 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-8 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-8 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-le9`, plan questions `comp-le9.1`, dev questions `comp-le9.2`

Fixtures:
- pass: Every sc-compose publishing path invokes the docli command-line documentation generator each time it publishes.
- fail: Run docli only for occasional documentation updates; routine sc-compose publication skips the documentation generator.

## ANC-016

Anchor: Release preflight for sc-compose must fail if the docli tool reports that the command-line documentation is stale.

> docli must be run every publish (stale documentation blocks preflight)

Said by randlee, Proposed from524772f4:ANC-004; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 5.

Summary: stale CLI docs block preflight

Scope: An item that adds or changes sc-compose release preflight configuration or the checks release preflight runs.

Deterministic check to pair with the development gate: stale generated reference negative preflight test

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-3`: Phase V assigns this requirement to comp-v-8 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-3 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-8 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-8 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-0qp`, plan questions `comp-0qp.1`, dev questions `comp-0qp.2`

Fixtures:
- pass: Release preflight invokes docli and fails before publication whenever docli reports that command-line documentation is stale.
- fail: When docli reports stale command-line documentation, release preflight still reports success and permits sc-compose publication.

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

Anchor: The full build-and-install matrix (every supported artifact on every supported OS and architecture, plus each supported wheel Python version installed with pip and with uv) must run only in the integration tests release preflight requires, not in the CI workflow that runs on every push or pull request.

> I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint

Said by randlee, Proposed from524772f4:ANC-002; exact user quote, not locked; reviewed source definitions are evidence, not approval., 2026-11-03. Status: locked. Version 7.

Summary: full matrix only in integration test

Scope: An item that adds or changes CI workflows, integration tests or release preflight runs that build or install sc-compose artifacts. The full matrix (Rand approved, relay ATM01M6JJ70WRQYV1KCEDZZK6WGJW) is every supported artifact/platform cell (macOS, Windows and Linux on x64 and arm64; Rust CLI archive, Python wheels, Go module) plus wheel Python-version and pip/uv install combinations.

Deterministic check to pair with the development gate: inspect triggers and preflight dependency; full install matrix is platform/Python/installer coverage

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-3`: No Phase V sprint claims this for comp-v-3; Jev leans not relevant and the sprint does not change CI pipeline jobs.
- `comp-v-4`: comp-v-4 changes the analyzer's unit CI, not the build-and-install matrix; it already keeps Windows amd64 and manylinux integration runs out of normal CI and in manual preflight.

Beads: anchor `comp-81v`, plan questions `comp-81v.1`, dev questions `comp-81v.2`

Fixtures:
- pass: Run the macOS/Windows/Linux x64/arm64 Rust, wheel and Go matrix, including supported wheel Python versions with pip and uv, only in the integration test required for release preflight; routine sprint CI does not run it.
- fail: Execute the complete macOS/Windows/Linux x64/arm64 Rust, wheel and Go install matrix, including supported wheel Python versions with pip and uv, in every routine sprint CI run.

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

Said by randlee, Proposed; source memory/lint-must-pass-allowlist-words.md quoted Rand words; narrow original codespell context retained for review, 2026-11-03. Status: locked. Version 4.

Summary: lint passes

Scope: The spelling lint check for project-specific words in sc-compose.

Deterministic check to pair with the development gate: just lint on exact candidate; no disabling rules

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-3`: Phase V assigns this requirement to comp-v-4 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-3 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-4 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-8`: Phase V assigns this requirement to comp-v-4 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-8 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns the codespell check to comp-v-4 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 does not change the spelling lint.

Beads: anchor `comp-hfk`, plan questions `comp-hfk.1`, dev questions `comp-hfk.2`

Fixtures:
- pass: The sc-compose codespell spelling check runs successfully with exit0 and no outstanding spelling findings.
- fail: The final sc-compose sprint candidate deliberately leaves codespell failing with unresolved spelling errors and a nonzero exit; passing codespell is not required for acceptance.

## ANC-032

Anchor: Windows amd64 integration tests must run on GitHub instead of unavailable local Windows amd64 hardware.

> you can run win amd64 and manylinux on github, but do not add them to the normal ci pipeline runs.

Said by randlee, Proposed; source user transcript5764;5756 states local hardware unavailable, 2026-11-03. Status: retired. Version 2.

Summary: Windows integration runs on GitHub

Scope: u-8/u-9 full integration test platform execution

Deterministic check to pair with the development gate: inspect integration runner selection

## ANC-033

Anchor: Windows amd64 and manylinux integration jobs must not be part of the CI workflow that runs on every push or pull request.

> you can run win amd64 and manylinux on github, but do not add them to the normal ci pipeline runs.

Said by randlee, Proposed; source user transcript5764, 2026-11-03. Status: locked. Version 5.

Summary: integration runs outside normal CI

Scope: An item that adds or changes sc-compose CI workflows or the integration-test workflow, including which jobs run on Windows amd64 or manylinux.

Deterministic check to pair with the development gate: inspect CI invocation paths; does not ban ordinary Windows unit CI

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-3`: No Phase V sprint claims this for comp-v-3; Jev leans not relevant and the sprint does not change CI pipeline jobs.
- `comp-v-9`: No Phase V sprint claims this for comp-v-9; Jev leans not relevant and the sprint does not change CI pipeline jobs.

Beads: anchor `comp-eh6`, plan questions `comp-eh6.1`, dev questions `comp-eh6.2`

Fixtures:
- pass: Windows amd64 and manylinux integration runs execute outside the normal CI pipeline; ordinary sprint CI does not invoke those integration runs.
- fail: The normal CI pipeline invokes the Windows amd64 and manylinux integration runs on every sprint check.

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

Said by randlee, Lead01M6JAKMVZ5RXHGS45A59P45G6 exact new quote; bug exception transcript6183; execution detail01M6JAMQ7ZCF4C8C8K63N01S62/01M6JAN3316GWY4V43QZFK00Z2/01M6JANVK3N4Q4K535KCK9QPG3, 2026-11-03. Status: retired. Version 3.

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

Said by randlee, Meaning approved by ATM01M6JJ70WRQYV1KCEDZZK6WGJW; remains proposed because rubric is inconclusive., 2026-11-03. Status: retired. Version 2.

Summary: all30 artifact/platform builds

Scope: Phase U release builds. Rand approved via ATM01M6JJ70WRQYV1KCEDZZK6WGJW that support requires actual successful builds of Rust CLI + three Python wheels + Go native module on macOS/Windows/Linux x64/arm64, all 30 cells; this clarification is normative alongside the unchanged original quote.

Deterministic check to pair with the development gate: Compare actual successful build outputs against all30 required artifact/platform cells, no missing cell passes.

## FR-24.8

Anchor: With advanced pour selected, graph pour by path and attach accept caller bead inputs; without it, they reject those inputs exactly as v1.7.0 does.

> With advanced pour selected, graph pour by path and attach accept caller bead inputs; without it, they reject those inputs exactly as v1.7.0 does.

Said by randlee, Approved corrections ATM01M6JPXYDJAHCYXVX4WDR1KQVB; docs/requirements.md on plan/phase-u-live-beads@275f17ed; source clauses only, ADR-0024 open decisions are not implicitly approved., 2026-11-03. Status: retired. Version 3.

Summary: FR-24.8 approved replacement

Scope: Explicitly selected sc-compose rendering for graph-pour and attach operations.

Deterministic check to pair with the development gate: Verify FR-24.8 against its approved requirement clause and actual positive/negative runtime evidence.

## FR-24.7

Anchor: An explicit bd-owned mode leaves `{{ }}` expressions for bd to evaluate and keeps today's registry `--var` behavior.

> An explicit bd-owned mode leaves `{{ }}` expressions for bd to evaluate and keeps today's registry `--var` behavior.

Said by randlee, Approved corrections ATM01M6JPXYDJAHCYXVX4WDR1KQVB; docs/requirements.md on plan/phase-u-live-beads@275f17ed; source clauses only, ADR-0024 open decisions are not implicitly approved., 2026-11-03. Status: retired. Version 3.

Summary: FR-24.7 approved replacement

Scope: Explicit selection of expression evaluation ownership for Beads formulas.

Deterministic check to pair with the development gate: Verify FR-24.7 against its approved requirement clause and actual positive/negative runtime evidence.

## FR-24.10

Anchor: A graph-plan output path that is blocked, including by a directory, is refused before any bd command runs.

> A graph-plan output path that is blocked, including by a directory, is refused before any bd command runs.

Said by randlee, Approved corrections ATM01M6JPXYDJAHCYXVX4WDR1KQVB; docs/requirements.md on plan/phase-u-live-beads@275f17ed; source clauses only, ADR-0024 open decisions are not implicitly approved., 2026-11-03. Status: retired. Version 3.

Summary: FR-24.10 approved replacement

Scope: Beads graph-plan output-path preflight; other validation reads remain permitted.

Deterministic check to pair with the development gate: Verify FR-24.10 against its approved requirement clause and actual positive/negative runtime evidence.

## FR-24.2

Anchor: Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`. Goldens are generated once from v1.7.0 and are never regenerated from candidate output. Any unlisted difference fails CI. Adding an entry requires user approval.

> Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`. Goldens are generated once from v1.7.0 and are never regenerated from candidate output. Any unlisted difference fails CI. Adding an entry requires user approval.

Said by randlee, Approved corrections ATM01M6JPXYDJAHCYXVX4WDR1KQVB; docs/requirements.md on plan/phase-u-live-beads@275f17ed; source clauses only, ADR-0024 open decisions are not implicitly approved., 2026-11-03. Status: retired. Version 3.

Summary: FR-24.2 approved replacement

Scope: Schema-less output byte compatibility; every unlisted difference fails the compatibility check.

Deterministic check to pair with the development gate: Verify FR-24.2 against its approved requirement clause and actual positive/negative runtime evidence.

## FR-25.1

Anchor: All30 artifact/platform cells must build successfully: Rust CLI, sc-compose/sc-sha/sc-composer-beads wheels and sc-sha-go native module, each for macOS/Windows/Linux on x64/arm64.

> All 30 artifact/platform cells must build successfully: the Rust CLI, the
sc-compose, sc-sha and sc-composer-beads wheels, and the sc-sha-go native module,
each for macOS, Windows and Linux on x64 and arm64.

Said by randlee, Approved corrections ATM01M6JPXYDJAHCYXVX4WDR1KQVB; docs/requirements.md on plan/phase-u-live-beads@275f17ed; source clauses only, ADR-0024 open decisions are not implicitly approved., 2026-11-03. Status: locked. Version 1.

Summary: FR-25.1 approved replacement

Scope: All30 successful build cells: five named artifact families across all six OS/architecture targets.

Deterministic check to pair with the development gate: Verify FR-25.1 against its approved requirement clause and actual positive/negative runtime evidence.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-2`: Phase V assigns this requirement to comp-v-6 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-2 only defers to, consumes or re-validates it.
- `comp-v-3`: Phase V assigns this requirement to comp-v-6 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-3 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-6 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-6 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-9`: Phase V assigns this requirement to comp-v-6 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-9 only defers to, consumes or re-validates it.

Beads: anchor `comp-69i`, plan questions `comp-69i.1`, dev questions `comp-69i.2`

## FR-24.13

Anchor: Advanced pour is turned on by an explicit new option; without it, nothing changes.

> Advanced pour is turned on by an explicit new option; without it, nothing changes.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 1

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.14

Anchor: In advanced pour, sc-compose renders every loop, condition, include and variable expression; the formula or graph sent to bd contains no template expression left for bd to evaluate.

> In advanced pour, sc-compose renders every loop, condition, include and variable expression; the formula or graph sent to bd contains no template expression left for bd to evaluate.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 2

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.15

Anchor: In advanced pour, sc-compose never passes a value to bd as `--var`.

> In advanced pour, sc-compose never passes a value to bd as `--var`.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 3

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.19

Anchor: Every field bd stores, including labels, holds the rendered value, checked by reading the created bead back from bd, not by checking the pre-bd file.

> Every field bd stores, including labels, holds the rendered value, checked by reading the created bead back from bd, not by checking the pre-bd file.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 7

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.20

Anchor: Filters act on the real value: `A{BC` lowercases to `a{bc`, has length 4, and its first three characters are `A{B`.

> Filters act on the real value: `A{BC` lowercases to `a{bc`, has length 4, and its first three characters are `A{B`.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 8

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.21

Anchor: A default declared in an included template's frontmatter fills a value the caller didn't supply, and the default's text stays literal.

> A default declared in an included template's frontmatter fills a value the caller didn't supply, and the default's text stays literal.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 9

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.22

Anchor: Advanced attach keeps today's safety behavior: preview writes nothing to bd; creation happens once, atomically, and only when authorized; repeating it preserves existing status and notes.

> Advanced attach keeps today's safety behavior: preview writes nothing to bd; creation happens once, atomically, and only when authorized; repeating it preserves existing status and notes.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 11

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.23

Anchor: The real `sprint-group` and `finding-group` orchestration formulas pour through advanced mode, and the created beads' titles, descriptions, labels, ids and edges match the expected output.

> The real `sprint-group` and `finding-group` orchestration formulas pour through advanced mode, and the created beads' titles, descriptions, labels, ids and edges match the expected output.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 12

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.24

Anchor: Without the new option, every existing Rust, CLI and Python call behaves exactly as in v1.7.0: same rendered bytes, same bd command line, same error codes.

> Without the new option, every existing Rust, CLI and Python call behaves exactly as in v1.7.0: same rendered bytes, same bd command line, same error codes.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: proposed. Version 1.

Summary: Approved Beads requirement 13

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.25

Anchor: Existing Rust request struct literals compile unchanged; the new mode is added as a new option or entrypoint, not as a new required field.

> Existing Rust request struct literals compile unchanged; the new mode is added as a new option or entrypoint, not as a new required field.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 15

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.26

Anchor: The same advanced request gives identical rendered output and identical bd command lines from Rust, the CLI and Python.

> The same advanced request gives identical rendered output and identical bd command lines from Rust, the CLI and Python.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 16

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.27

Anchor: If rendering or validation fails, no bead is created.

> If rendering or validation fails, no bead is created.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 19

Scope: An item that implements, changes, tests or verifies how the sc-compose bead commands or sc-composer-beads create beads, including what happens when rendering or validation fails.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-80m`, plan questions `comp-80m.1`, dev questions `comp-80m.2`

## FR-24.28

Anchor: The undeclared-variable check runs only when explicitly requested, and only for templates that declare a schema.

> The undeclared-variable check runs only when explicitly requested, and only for templates that declare a schema.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 20

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.29

Anchor: The recovered Beads code and tests are ported, not rewritten. #791's strict-only default is not ported: strict validation stays off unless the caller passes `--strict` (or the matching API option).

> The recovered Beads code and tests are ported, not rewritten. #791's strict-only default is not ported: strict validation stays off unless the caller passes `--strict` (or the matching API option).

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 21

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.30

Anchor: `bead.md` documents each mode with a worked input-to-created-bead example.

> `bead.md` documents each mode with a worked input-to-created-bead example.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 22

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.31

Anchor: The closing fuzz sprint reruns FUZZ-171-01..14, T-02..T-06, D-01 and include-default #11 as regression tests on the final candidate.

> The closing fuzz sprint reruns FUZZ-171-01..14, T-02..T-06, D-01 and include-default #11 as regression tests on the final candidate.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 23

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.32

Anchor: The 229-template corpus that passed the v1.7.0 comparison is rerun on the final candidate, without `--strict`, through validate, render and render with variables. Any template that 1.7.0 accepted and the candidate rejects fails the check. The only allowed differences are the false-positive diagnostics already approved under D7.

> The 229-template corpus that passed the v1.7.0 comparison is rerun on the final candidate, without `--strict`, through validate, render and render with variables. Any template that 1.7.0 accepted and the candidate rejects fails the check. The only allowed differences are the false-positive diagnostics already approved under D7.

Said by randlee, User-approved Phase U Beads requirements, 2026-11-04. Status: proposed. Version 1.

Summary: Approved Beads requirement 24

Scope: Phase U Beads rendering, graph and compatibility work

## FR-24.13a

Anchor: When `--render-by` is not specified, the sc-compose bead commands must use sc-compose rendering (sc-compose renders every template expression before bd runs).

> Advanced pour is the default: sc-compose rendering is used when `--render-by` is not specified, and `--render-by bd` selects bd rendering.

Said by randlee, User-approved Phase U Beads requirement (approved list item 1); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.13a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 1

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 builds and releases artifacts and does not implement it.

Beads: anchor `comp-d63`, plan questions `comp-d63.1`, dev questions `comp-d63.2`

## FR-24.14a

Anchor: In advanced pour (the default mode: `--render-by sc-compose`, or no `--render-by`), the formula or graph that sc-compose sends to bd must contain no `{{`, `{%` or `{#` template expression.

> In advanced pour, sc-compose renders every loop, condition, include and variable expression; the formula or graph sent to bd contains no template expression left for bd to evaluate.

Said by randlee, User-approved Phase U Beads requirement (approved list item 2); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.14a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 2

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-ce8`, plan questions `comp-ce8.1`, dev questions `comp-ce8.2`

## FR-24.15a

Anchor: In advanced pour, the default mode in which sc-compose renders the template before bd runs (`--render-by sc-compose`, or no `--render-by`), sc-compose must never pass a value to bd as a `--var` argument.

> In advanced pour, sc-compose never passes a value to bd as `--var`.

Said by randlee, User-approved Phase U Beads requirement (approved list item 3); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.15a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 3.

Summary: Approved Beads requirement 3

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-o72`, plan questions `comp-o72.1`, dev questions `comp-o72.2`

## FR-24.19a

Anchor: In advanced pour, the default mode in which sc-compose renders the template before bd runs (`--render-by sc-compose`, or no `--render-by`), every field of each bead that sc-compose creates, including its labels, must hold the rendered value when that bead is read back from bd.

> Every field bd stores, including labels, holds the rendered value, checked by reading the created bead back from bd, not by checking the pre-bd file.

Said by randlee, User-approved Phase U Beads requirement (approved list item 7); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.19a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 3.

Summary: Approved Beads requirement 7

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-bat`, plan questions `comp-bat.1`, dev questions `comp-bat.2`

## FR-24.21b

Anchor: With `--render-by sc-compose`, a default declared in an included template's frontmatter must be inserted as literal text: any `{{ }}` or `{% %}` in the default's text appears in the output unchanged.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. A default declared in an included template's frontmatter fills a value the caller didn't supply, and the default's text stays literal.

Said by randlee, User-approved Phase U Beads requirement (approved list item 9b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.21b.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 4.

Summary: Approved Beads requirement 9b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-3vj`, plan questions `comp-3vj.1`, dev questions `comp-3vj.2`

## FR-24.8a

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), graph pour by path must accept caller bead inputs.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. With advanced pour, graph pour by path and attach accept caller bead inputs; with `--render-by bd`, they reject those inputs exactly as v1.7.0 does.

Said by randlee, User-approved Phase U Beads requirement (approved list item 10a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.8a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 10a

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 integrates delivery and the release matrix and does not implement bead-command behavior.

Beads: anchor `comp-oni`, plan questions `comp-oni.1`, dev questions `comp-oni.2`

## FR-24.8c

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), attach must accept caller bead inputs.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. With advanced pour, graph pour by path and attach accept caller bead inputs; with `--render-by bd`, they reject those inputs exactly as v1.7.0 does.

Said by randlee, User-approved Phase U Beads requirement (approved list item 10c); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.8c.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 10c

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-2gc`, plan questions `comp-2gc.1`, dev questions `comp-2gc.2`

## FR-24.22a

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), preview-attach must write nothing to bd.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. Advanced attach keeps today's safety behavior: preview writes nothing to bd; creation happens once, atomically, and only when authorized; repeating it preserves existing status and notes.

Said by randlee, User-approved Phase U Beads requirement (approved list item 11a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.22a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 11a

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 integrates delivery and the release matrix and does not implement bead-command behavior.

Beads: anchor `comp-de6`, plan questions `comp-de6.1`, dev questions `comp-de6.2`

## FR-24.2a

Anchor: Schema-less template output must be byte-identical to the sc-compose v1.7.0 golden output, except for differences listed in `approved-exceptions.json`.

> Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`.

Said by randlee, User-approved Phase U Beads requirement (approved list item 14a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.2a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 14a

Scope: the CI byte-regression check comparing schema-less template output with sc-compose v1.7.0

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-8`: Phase V assigns the byte-regression check to comp-v-3, with CI wiring in comp-v-6 (comps reset dispositions); comp-v-8 builds publish tooling and does not touch it.

Beads: anchor `comp-4go`, plan questions `comp-4go.1`, dev questions `comp-4go.2`

## FR-24.2b

Anchor: Every entry in `approved-exceptions.json` must name the GitHub issue linked to that difference.

> Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`. Goldens are generated once from v1.7.0 and are never regenerated from candidate output. Any unlisted difference fails CI. Adding an entry requires user approval.

Said by randlee, User-approved Phase U Beads requirement (approved list item 14b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.2b.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: retired. Version 2.

Summary: Approved Beads requirement 14b

Scope: the CI byte-regression check comparing schema-less template output with sc-compose v1.7.0

Beads: anchor `comp-q4m`, plan questions `comp-q4m.1`, dev questions `comp-q4m.2`

## FR-24.2d

Anchor: The CI byte-regression check must fail when schema-less output differs from the v1.7.0 golden output in a way not listed in `approved-exceptions.json`.

> Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`.

Said by randlee, User-approved Phase U Beads requirement (approved list item 14d); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.2d.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 14d

Scope: the CI byte-regression check comparing schema-less template output with sc-compose v1.7.0

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-2`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-2 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-kh7`, plan questions `comp-kh7.1`, dev questions `comp-kh7.2`

## FR-24.26b

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), the same request must give identical bd command lines from Rust, the CLI and Python.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. The same advanced request gives identical rendered output and identical bd command lines from Rust, the CLI and Python.

Said by randlee, User-approved Phase U Beads requirement (approved list item 16b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.26b.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 16b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 builds and releases artifacts and does not implement it.

Beads: anchor `comp-xxx`, plan questions `comp-xxx.1`, dev questions `comp-xxx.2`

## FR-24.7a

Anchor: With `--render-by bd` (sc-compose leaves `{{ }}` expressions for bd to render from `--var` values), sc-compose must send the `{{ }}` expressions to bd unrendered.

> bd-owned mode is the pour mode in which sc-compose leaves `{{ }}` expressions in the formula for bd to render from `--var` values, as sc-compose v1.7.0 does. An explicit bd-owned mode leaves `{{ }}` expressions for bd to evaluate and keeps today's registry `--var` behavior.

Said by randlee, User-approved Phase U Beads requirement (approved list item 17a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.7a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 17a

Scope: bd-owned mode of the sc-compose bead commands and sc-composer-beads: the explicit mode in which sc-compose leaves `{{ }}` template expressions for bd to evaluate (approved Phase U requirement 17)

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-nss`, plan questions `comp-nss.1`, dev questions `comp-nss.2`

## FR-24.7b

Anchor: With `--render-by bd` (sc-compose leaves `{{ }}` expressions for bd to render from `--var` values), sc-compose must keep passing registry variables to bd as `--var` arguments, as sc-compose v1.7.0 does.

> bd-owned mode is the pour mode in which sc-compose leaves `{{ }}` expressions in the formula for bd to render from `--var` values, as sc-compose v1.7.0 does. An explicit bd-owned mode leaves `{{ }}` expressions for bd to evaluate and keeps today's registry `--var` behavior.

Said by randlee, User-approved Phase U Beads requirement (approved list item 17b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.7b.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 17b

Scope: bd-owned mode of the sc-compose bead commands and sc-composer-beads: the explicit mode in which sc-compose leaves `{{ }}` template expressions for bd to evaluate (approved Phase U requirement 17)

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-2zh`, plan questions `comp-2zh.1`, dev questions `comp-2zh.2`

## FR-24.10a

Anchor: When the graph-plan output path is blocked, including by an existing directory, sc-compose must refuse the command before running any bd command.

> A graph-plan output path that is blocked, including by a directory, is refused before any bd command runs.

Said by randlee, User-approved Phase U Beads requirement (approved list item 18); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.10a.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 18

Scope: the sc-compose graph-plan command and its output path

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-8`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-8 only defers to, consumes or re-validates it.

Beads: anchor `comp-z81`, plan questions `comp-z81.1`, dev questions `comp-z81.2`

## FR-24.28b

Anchor: The undeclared-variable check must never run on a template that declares no schema.

> The undeclared-variable check runs only when explicitly requested, and only for templates that declare a schema.

Said by randlee, User-approved Phase U Beads requirement (approved list item 20b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.28b.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 20b

Scope: the undeclared-variable check in sc-composer validation and the sc-compose CLI

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-0ka`, plan questions `comp-0ka.1`, dev questions `comp-0ka.2`

## FR-24.29b

Anchor: Strict validation must stay off unless the caller passes `--strict` or the matching API option.

> The recovered Beads code and tests are ported, not rewritten. #791's strict-only default is not ported: strict validation stays off unless the caller passes `--strict` (or the matching API option).

Said by randlee, User-approved Phase U Beads requirement (approved list item 21b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.29b.json; locked on user approval 2026-11-04 ("1 - lock"), 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 21b

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-mbj`, plan questions `comp-mbj.1`, dev questions `comp-mbj.2`

## FR-24.20a

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), the `lower` filter applied to the value `A{BC` must return `a{bc`.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. Filters act on the real value: `A{BC` lowercases to `a{bc`, has length 4, and its first three characters are `A{B`.

Said by randlee, User-approved Phase U Beads requirement (item 8); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.20a.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 8

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-rcz`, plan questions `comp-rcz.1`, dev questions `comp-rcz.2`

## FR-24.20b

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), the `length` filter applied to the value `A{BC` must return 4.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. Filters act on the real value: `A{BC` lowercases to `a{bc`, has length 4, and its first three characters are `A{B`.

Said by randlee, User-approved Phase U Beads requirement (item 8b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.20b.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 8b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-87z`, plan questions `comp-87z.1`, dev questions `comp-87z.2`

## FR-24.20c

Anchor: With `--render-by sc-compose`, slicing the value `A{BC` to its first three characters (for example `{{ v[:3] }}`) must return `A{B`.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. Filters act on the real value: `A{BC` lowercases to `a{bc`, has length 4, and its first three characters are `A{B`.

Said by randlee, User-approved Phase U Beads requirement (item 8c); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.20c.json, 2026-11-04. Status: locked. Version 3.

Summary: Approved Beads requirement 8c

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-1 records the recovery contract and does not implement bead rendering.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 integrates delivery and the release matrix and does not implement bead-command behavior.

Beads: anchor `comp-b82`, plan questions `comp-b82.1`, dev questions `comp-b82.2`

## FR-24.21a

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), a variable the caller does not supply must take the default declared in the included template's frontmatter.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. A default declared in an included template's frontmatter fills a value the caller didn't supply, and the default's text stays literal.

Said by randlee, User-approved Phase U Beads requirement (item 9a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.21a.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 9a

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-96c`, plan questions `comp-96c.1`, dev questions `comp-96c.2`

## FR-24.22b

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), `sc-compose bead attach` must create all of its beads in bd or none of them.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. Advanced attach keeps today's safety behavior: preview writes nothing to bd; creation happens once, atomically, and only when authorized; repeating it preserves existing status and notes.

Said by randlee, User-approved Phase U Beads requirement (item 11b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.22b.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 11b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-gga`, plan questions `comp-gga.1`, dev questions `comp-gga.2`

## FR-24.22c

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), `sc-compose bead attach` must create no bead when its request does not set `pour_authorization` to `CreatePersistentBeads`.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. Advanced attach keeps today's safety behavior: preview writes nothing to bd; creation happens once, atomically, and only when authorized; repeating it preserves existing status and notes.

Said by randlee, User-approved Phase U Beads requirement (item 11c); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.22c.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 11c

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-6bo`, plan questions `comp-6bo.1`, dev questions `comp-6bo.2`

## FR-24.22d

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), running the same `sc-compose bead attach` request twice must leave the status and notes of the beads from the first run unchanged.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. Advanced attach keeps today's safety behavior: preview writes nothing to bd; creation happens once, atomically, and only when authorized; repeating it preserves existing status and notes.

Said by randlee, User-approved Phase U Beads requirement (item 11d); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.22d.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 11d

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-id7`, plan questions `comp-id7.1`, dev questions `comp-id7.2`

## FR-24.23b

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), pouring `.claude/skills/atm-bd-orchestration/formulas/finding-group.formula.toml.j2` must create beads whose titles, descriptions, labels, ids and dependency edges equal those recorded in `crates/sc-compose/tests/fixtures/bead/finding-group.expected.json`.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. The real `sprint-group` and `finding-group` orchestration formulas pour through advanced mode, and the created beads' titles, descriptions, labels, ids and edges match the expected output.

Said by randlee, User-approved Phase U Beads requirement (item 12b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.23b.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 12b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-kme`, plan questions `comp-kme.1`, dev questions `comp-kme.2`

## FR-24.13b

Anchor: `--render-by bd` must make the sc-compose bead commands use bd rendering, leaving `{{ }}` expressions for bd.

> Advanced pour is the default: sc-compose rendering is used when `--render-by` is not specified, and `--render-by bd` selects bd rendering.

Said by randlee, User-approved Phase U Beads requirement (1b); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.13b.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 1b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-1sh`, plan questions `comp-1sh.1`, dev questions `comp-1sh.2`

## FR-24.2c

Anchor: The golden files in `tests/fixtures/render-regressions/goldens/` must be generated once by running sc-compose v1.7.0 (commit f4fda4b9).

> Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`. Goldens are generated once from v1.7.0 and are never regenerated from candidate output. Any unlisted difference fails CI. Adding an entry requires user approval.

Said by randlee, User-approved Phase U Beads requirement (14c); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.2c.json, 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 14c

Scope: the CI byte-regression check comparing schema-less template output with sc-compose v1.7.0

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-69a`, plan questions `comp-69a.1`, dev questions `comp-69a.2`

## FR-24.2f

Anchor: The byte-regression check must never rewrite `tests/fixtures/render-regressions/goldens/` from candidate output.

> Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`. Goldens are generated once from v1.7.0 and are never regenerated from candidate output. Any unlisted difference fails CI. Adding an entry requires user approval.

Said by randlee, User-approved Phase U Beads requirement (14f); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.2f.json, 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 14f

Scope: the CI byte-regression check comparing schema-less template output with sc-compose v1.7.0

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-9`: Phase V assigns the byte-regression check to comp-v-3, with CI wiring in comp-v-6 (comps reset dispositions); comp-v-9 builds the Go native adapter and does not touch it.

Beads: anchor `comp-3nv`, plan questions `comp-3nv.1`, dev questions `comp-3nv.2`

## FR-24.25a

Anchor: Adding `--render-by` must not add a required field to any `sc-composer-beads` request struct, so existing struct literals compile unchanged.

> Existing Rust request struct literals compile unchanged; the new mode is added as a new option or entrypoint, not as a new required field.

Said by randlee, User-approved Phase U Beads requirement (15); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.25a.json, 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 15

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 integrates delivery and the release matrix and does not implement bead-command behavior.

Beads: anchor `comp-3h1`, plan questions `comp-3h1.1`, dev questions `comp-3h1.2`

## FR-24.23a

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), the beads created from the real `sprint-group` orchestration formula must match the expected output recorded in `crates/sc-compose/tests/fixtures/bead/sprint-group.expected.json`.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. The real `sprint-group` and `finding-group` orchestration formulas pour through advanced mode, and the created beads' titles, descriptions, labels, ids and edges match the expected output.

Said by randlee, User-approved Phase U Beads requirement (12a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.23a.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 12a

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-bwd`, plan questions `comp-bwd.1`, dev questions `comp-bwd.2`

## FR-24.2e

Anchor: Each entry in `tests/fixtures/render-regressions/approved-exceptions.json` must link the GitHub issue that records the user's approval of that difference.

> Schema-less template output matches v1.7.0 byte for byte. A difference is allowed only if the user has approved it as a genuine bug fix, and it is recorded with its linked issue in `approved-exceptions.json`. Goldens are generated once from v1.7.0 and are never regenerated from candidate output. Any unlisted difference fails CI. Adding an entry requires user approval.

Said by randlee, User-approved Phase U Beads requirement (14e); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.2e.json, 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 14e

Scope: the CI byte-regression check comparing schema-less template output with sc-compose v1.7.0

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-5j0`, plan questions `comp-5j0.1`, dev questions `comp-5j0.2`

## FR-24.26a

Anchor: With `--render-by sc-compose` (sc-compose renders every template expression before bd runs), one request must give the same rendered output from Rust, the CLI and Python.

> Advanced pour is the opt-in pour mode in which sc-compose renders every template expression in the formula or graph before bd runs, so bd receives only finished text. The same advanced request gives identical rendered output and identical bd command lines from Rust, the CLI and Python.

Said by randlee, User-approved Phase U Beads requirement (16a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.26a.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 16a

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 builds and releases artifacts and does not implement it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-5 builds stable-ABI wheels and does not implement bead-command behavior.

Beads: anchor `comp-36e`, plan questions `comp-36e.1`, dev questions `comp-36e.2`

## FR-24.28a

Anchor: The undeclared-variable check must run only when the caller passes `--strict` or the matching API option.

> The undeclared-variable check runs only when explicitly requested, and only for templates that declare a schema. The recovered Beads code and tests are ported, not rewritten. #791's strict-only default is not ported: strict validation stays off unless the caller passes `--strict` (or the matching API option).

Said by randlee, User-approved Phase U Beads requirement (20a); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.28a.json, 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 20a

Scope: the undeclared-variable check in sc-composer validation and the sc-compose CLI

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-0lh`, plan questions `comp-0lh.1`, dev questions `comp-0lh.2`

## FR-24.8b

Anchor: With `--render-by bd`, graph pour by path must reject caller bead inputs with `BEADS_GRAPH_FORMULA_UNSUPPORTED`.

> bd-owned mode is the pour mode in which sc-compose leaves `{{ }}` expressions in the formula for bd to render from `--var` values, as sc-compose v1.7.0 does. With advanced pour, graph pour by path and attach accept caller bead inputs; with `--render-by bd`, they reject those inputs exactly as v1.7.0 does (graph pour by path: `BEADS_GRAPH_FORMULA_UNSUPPORTED` with reason `bead_variables_set`; attach: request error `attach forbids bead_variables`).

Said by randlee, User-approved Phase U Beads requirement (10b, clarity-edited 2026-11-04); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.8b.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 10b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-d5o`, plan questions `comp-d5o.1`, dev questions `comp-d5o.2`

## FR-24.8d

Anchor: With `--render-by bd`, attach must reject caller bead inputs with the error `attach forbids bead_variables`.

> bd-owned mode is the pour mode in which sc-compose leaves `{{ }}` expressions in the formula for bd to render from `--var` values, as sc-compose v1.7.0 does. With advanced pour, graph pour by path and attach accept caller bead inputs; with `--render-by bd`, they reject those inputs exactly as v1.7.0 does (graph pour by path: `BEADS_GRAPH_FORMULA_UNSUPPORTED` with reason `bead_variables_set`; attach: request error `attach forbids bead_variables`).

Said by randlee, User-approved Phase U Beads requirement (10d, clarity-edited 2026-11-04); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.8d.json, 2026-11-04. Status: locked. Version 2.

Summary: Approved Beads requirement 10d

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-h1c`, plan questions `comp-h1c.1`, dev questions `comp-h1c.2`

## FR-24.24b

Anchor: With `--render-by bd`, each call to a Rust API function, CLI command or Python function that exists in sc-compose v1.7.0 (commit f4fda4b9) must make sc-compose run exactly the bd command line, with the same arguments in the same order, that v1.7.0 runs for the same call and inputs.

> With `--render-by bd`, every Rust API function, CLI command and Python function that exists in sc-compose v1.7.0 (commit f4fda4b9) behaves exactly as in v1.7.0: same rendered bytes, same bd command line, same error codes.

Said by randlee, User-approved Phase U Beads requirement (13b, clarity-edited 2026-11-04); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.24b.json, 2026-11-04. Status: locked. Version 4.

Summary: Approved Beads requirement 13b

Scope: An item that implements, changes, documents, tests or verifies the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach) or the sc-composer-beads library in advanced pour, the default mode (`--render-by sc-compose`) in which sc-compose renders every loop, condition, include and variable expression before bd runs.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-301`, plan questions `comp-301.1`, dev questions `comp-301.2`

## FR-24.30a

Anchor: `bead.md` must document each `--render-by` mode with a worked input-to-created-bead example.

> `bead.md` (`crates/sc-compose/docs/manual/bead.md`) documents each mode (`--render-by sc-compose` and `--render-by bd`) with a worked input-to-created-bead example.

Said by randlee, User-approved Phase U Beads requirement (22, clarity-edited 2026-11-04); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.30a.json, 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 22

Scope: advanced bead pour in sc-composer-beads and the sc-compose bead commands (render, pour, preview-pour, attach, preview-attach)

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-5e9`, plan questions `comp-5e9.1`, dev questions `comp-5e9.2`

## FR-24.31a

Anchor: Before release, the fuzz regression cases FUZZ-171-01..14, T-02..T-06, D-01 and include-default #11 must be rerun as regression tests on the release candidate.

> The closing fuzz sprint reruns FUZZ-171-01..14, T-02..T-06, D-01 and include-default #11 (recovered in `crates/sc-composer-beads/tests/single_pass_render.rs` and `crates/sc-compose/tests/cli/fuzz_regressions.rs`; FUZZ-171-10 is not yet recovered) as regression tests on the final candidate.

Said by randlee, User-approved Phase U Beads requirement (23, clarity-edited 2026-11-04); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.31a.json, 2026-11-04. Status: locked. Version 3.

Summary: Approved Beads requirement 23

Scope: An item that maps, reruns or verifies the earlier fuzz regression cases (FUZZ-171-01..14, T-02..T-06, D-01, include-default #11) against a release candidate.

Beads: anchor `comp-dkf`, plan questions `comp-dkf.1`, dev questions `comp-dkf.2`

## FR-24.32a

Anchor: Run without `--strict`, the release candidate must accept every template in the 229-template corpus (150 atm-core and 79 sc-compose `.j2` templates) that sc-compose v1.7.0 accepts, where accepting means validate, render and render with variables all succeed.

> The 229-template corpus that passed the v1.7.0 comparison (QA report 01M6HZR22HZXX4HDW1CMFHHA86: 150 atm-core and 79 sc-compose `.j2` templates) is rerun on the final candidate, without `--strict`, through validate, render and render with variables. Any template that 1.7.0 accepted and the candidate rejects fails the check. The only allowed differences are the false-positive diagnostics already approved under D7 (`ERR_VAL_UNDECLARED_TOKEN` no longer reported for True/False/None literals, filter and call targets, Jinja test and block-tag names, and comment and raw bodies).

Said by randlee, User-approved Phase U Beads requirement (24a, clarity-edited 2026-11-04); testable under sealed rubric 3f1c0f65, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v4/FR-24.32a.json, 2026-11-04. Status: locked. Version 4.

Summary: Approved Beads requirement 24a

Scope: An item that runs or verifies the 229-template corpus check against a release candidate.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-3`: Phase V assigns this requirement to comp-v-7 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-3 only defers to, consumes or re-validates it.
- `comp-v-1`: Phase V assigns the 229-template corpus check to comp-v-7 (comps reset dispositions); comp-v-1 records the recovery contract and does not run it.

Beads: anchor `comp-a2y`, plan questions `comp-a2y.1`, dev questions `comp-a2y.2`

## FR-24.24c

Anchor: With `--render-by bd` (sc-compose leaves `{{ }}` expressions for bd to render, as v1.7.0 does), an input that sc-compose v1.7.0 (commit f4fda4b9) rejects must be rejected with the same error code.

> With `--render-by bd`, every Rust API function, CLI command and Python function that exists in sc-compose v1.7.0 (commit f4fda4b9) behaves exactly as in v1.7.0: same rendered bytes, same bd command line, same error codes.

Said by randlee, User-approved Phase U Beads requirement (13c, clarity-edited 2026-11-04); testable under sealed rubric 28ce3aec, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v5/FR-24.24c.json, 2026-11-04. Status: locked. Version 1.

Summary: Approved Beads requirement 13c

Scope: bd-owned mode of the sc-compose bead commands (pour, preview-pour, attach, preview-attach) and sc-composer-beads, selected by `--render-by bd`, in which sc-compose leaves `{{ }}` expressions for bd to render, as sc-compose v1.7.0 does

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-6 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.

Beads: anchor `comp-ipw`, plan questions `comp-ipw.1`, dev questions `comp-ipw.2`

## FR-24.29a

Anchor: The release's sc-composer-beads code and tests must be carried over from `crates/sc-composer-beads` on the `rebuild/from-main` branch, not rewritten.

> The recovered Beads code and tests (`crates/sc-composer-beads` on the `rebuild/from-main` branch) are ported, not rewritten. #791's strict-only default is not ported: strict validation stays off unless the caller passes `--strict` (or the matching API option).

Said by randlee, User-approved Phase U Beads requirement (21a, clarity-edited 2026-11-04); testable under sealed rubric 28ce3aec, evidence docs/plans/phase-u/anchor-runs/approved-beads-20261104/rubric-v5/FR-24.29a.json, 2026-11-04. Status: locked. Version 4.

Summary: Approved Beads requirement 21a

Scope: An item that ports, recovers, reuses or reimplements sc-composer-beads code or its tests, including from the rebuild/from-main branch.

Excluded from (not judged; the cell passes as `excluded`):
- `comp-v-1`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-1 only defers to, consumes or re-validates it.
- `comp-v-2`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-2 only defers to, consumes or re-validates it.
- `comp-v-4`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-4 only defers to, consumes or re-validates it.
- `comp-v-5`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-5 only defers to, consumes or re-validates it.
- `comp-v-7`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 583a6c40); comp-v-7 only defers to, consumes or re-validates it.
- `comp-v-6`: Phase V assigns this requirement to comp-v-3 (comps reset dispositions, plan QA PASS at 5981c08f); comp-v-6 builds and releases artifacts and does not implement it.

Beads: anchor `comp-sb0`, plan questions `comp-sb0.1`, dev questions `comp-sb0.2`
