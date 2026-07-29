# Fuzz-session report contract

This contract extends the generic `html-report` package contract. A fuzz
session is a coordinated run of multiple workers. Each worker receives one
distinct fuzz task, executes one bounded fuzz test, and returns one structured
JSON result. Each worker produces exactly one XHTML panel. The report package
contains one HTML page, one JSON sidecar, and one XHTML companion per worker.

## Session package

The top-level report data uses the generic fields `output_path`,
`json_output_path`, `title`, `status`, `summary_html`, and `sections[]`.
`summary_html` must contain a table with one row per worker and these columns:

| Column | Meaning |
| --- | --- |
| Fuzz run description | The distinct task assigned to the worker |
| Iterations | Number of bounded test inputs exercised |
| Pass | Fraction in `passed/iterations` form |
| Result | Simple `PASS` or `FAIL` verdict |

Each `sections[]` entry is one worker panel. It must include `id`, `title`,
`status`, `body_html`, `context_text`, `json_payload`, and `xhtml_path`, with
`fragment_source: "auto-generated"`. The panel XHTML must be generated from
`templates/fuzz-run-agent.xhtml.j2`; do not concatenate multiple workers into
one fragment.

## Worker panel fields

| Field | Required | Source / meaning |
| --- | --- | --- |
| `session_id` | yes | Durable fuzz-session identifier |
| `agent_id` | yes | Stable worker/task identifier |
| `fuzz_run_description` | yes | Human-readable task the worker exercised |
| `worker_correlation_id` | yes | Worker identity from the evidence envelope |
| `classification` | yes | `pass`, `confirmed_bug`, `intentional_boundary`, or `inconclusive` |
| `iterations` | yes | Bounded number of test inputs exercised |
| `passed` | yes | Number of inputs satisfying the worker oracle |
| `failed` | yes | Number of inputs not satisfying the oracle |
| `result` | yes | `PASS` only when `failed` is zero; otherwise `FAIL` |
| `summary` | yes | What this worker tested and what happened |
| `test_inputs` | yes | Compact `{case_id, description, outcome}` rows showing real inputs |
| `json_payload` | yes | Full worker evidence envelope used by copy-JSON |
| `copy_json` | yes | Deterministic escaped JSON serialization |
| `context_text` | yes | Human-readable worker context used by copy-context |

The worker payload may include the original `cases[]`, `findings[]`, command,
minimal template, minimal input, and recommended test fields from the E.3
evidence contract. Preserve those names in `json_payload`; do not invent a
parallel finding schema.

## Primary-agent failure investigation

The primary coordinator owns triage after workers return. For every candidate
failure it may deploy background explore agents to identify the applicable
requirement, ADR, or NFR; establish the evidence-backed root cause; and
propose recommended changes. The primary coordinator merges those conclusions
into the worker's structured JSON before rendering the panel. A worker panel
must therefore show the exact frontmatter/template and input that caused each
failure, not only a prose verdict.

## Failed worker findings

When `classification` is not a successful alias, `findings[]` is required.
Each finding must expose the original finding fields plus:

| Field | Required | Meaning |
| --- | --- | --- |
| `requirement_trace` | yes | Requirement/ADR/NFR citation, or the explicit text `No requirement or ADR currently covers this behavior.` |
| `requirement_follow_up` | yes when no existing reference | Recommend creating/updating a requirement or ADR only when the behavior is supported and the gap is genuine; otherwise record the rationale for no new document or name the owner of the product decision |
| `root_cause` | yes | Current evidence-backed cause, or an honest unresolved note |
| `recommended_fix` | yes | Next test, design, or implementation action |

A missing requirement reference is not by itself a reason to create an ADR or
requirement.

## Output naming and placement

Real session artifacts are written under `site/reports/` after the
`adversarial-fuzzing` skill completes. The main HTML and JSON use one shared
session stem:

`YYYYMMDD-N-fuzz-report.html`

Each companion panel uses the same designated stem plus the deterministic
worker suffix, for example:

`20260729-1-fuzz-report-shape-probe.xhtml`

Review mocks remain under `docs/examples/fuzz-run-report/` and are not session
output.
