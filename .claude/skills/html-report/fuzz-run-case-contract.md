# Fuzz-run case report contract

This contract extends the generic `html-report` package contract. The
`fuzz-run-case.xhtml.j2` template renders one case at a time. The
adversarial-fuzzing skill supplies the `case` object after a coordinator run
and passes the same case data to `html-report-generator` as the section's
`json_payload` and `context_text`.

## Common fields

| Field | Required | Source / meaning |
| --- | --- | --- |
| `campaign_id` | yes | `campaign.campaign_id` from the durable campaign evidence |
| `case_id` | yes | `execution.case_batches[].cases[]` identifier, or the finding ID for a minimized finding case |
| `worker_correlation_id` | yes | `workers[].correlation_id` / matching case batch |
| `classification` | yes | `findings[].classification`, or `pass` for a completed case with no finding |
| `summary` | yes | One short paragraph describing what the case tested and the result |
| `input_variables` | yes | Compact list of `{name, value}` pairs used by the case |
| `json_payload` | yes | Full machine-readable case envelope used by the copy-JSON action |
| `copy_json` | yes | Deterministic JSON serialization of `json_payload`, HTML-escaped for the icon-only copy action |
| `context_text` | yes | Human-readable case context used by the copy-context action |

The finding fields retain the names emitted by the E.3 evidence contract:
`finding_id`, `command`, `minimal_template`, `minimal_input`,
`expected_oracle`, `observed_result`, `diagnostic_code`,
`reproduction_count`, and `recommended_test`. The template does not rename or
remove those fields; they remain in `json_payload`.

## Successful case

Use `classification: pass` (the aliases `success`, `successful`, and `ok` are
also accepted). Required presentation fields are the common fields only. The
rendered fragment contains identity, the summary, and input variables. It
must not render failure, requirement, root-cause, or fix sections.

## Failed or inconclusive case

For any classification other than the successful aliases, provide:

| Field | Required | Meaning |
| --- | --- | --- |
| `expected_oracle` | yes | Contract expectation tested by the case |
| `observed_result` | yes | Actual output, exit status, or diagnostic |
| `requirement_trace` | yes | Requirement/ADR citation, or the explicit text `No requirement or ADR currently covers this behavior.` |
| `requirement_follow_up` | yes when `requirement_trace` has no existing reference | Assess the gap: recommend creating/updating a requirement or ADR only when the behavior is supported and the gap is genuine; otherwise record the rationale for no new document or name the owner of the product decision |
| `root_cause` | yes | Current evidence-backed cause; use an honest unresolved note when unknown |
| `recommended_fix` | yes | Next test, design, or implementation action |

The failed presentation adds expected-vs-observed, requirement/ADR trace,
requirement-gap follow-up, root-cause, and recommended-fix sections while
retaining the common identity and input table. A missing reference is not by
itself a reason to create an ADR or requirement.

## Output naming and placement

Real campaign artifacts are written under `site/reports/` after the
`adversarial-fuzzing` skill completes. Each case uses a 1-based sequence reset
per campaign day:

`YYYYMMDD-N-fuzz-report.html`

The JSON sidecar and optional failure fragment use the same stem. Review mocks
remain under `docs/examples/fuzz-run-report/` and are not campaign output.
