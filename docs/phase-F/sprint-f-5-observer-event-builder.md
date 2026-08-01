---
id: F.5
title: Observer Typed Event Builder
status: planned
branch: sprint/f-5-observer-event-builder
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-5-observer-event-builder
target: develop
---

# Sprint F.5 — Observer Typed Event Builder

## Goal

- Reduce defect risk in the high-churn CLI observability adapter by separating event-family mapping from common log-envelope construction, while preserving the documented observer and sc-observability boundary.
## Hard Dependencies

- None. The current crates/sc-compose/src/observer_impl.rs, sc-composer observer traits, sc-observability Logger API, and observer/CLI tests are the baseline.
- The architecture boundary is mandatory: sc-composer keeps local observer interfaces and must not depend on sc-observability, sc-observability-types, or ATM.
## Exact Targets

- `crates/sc-compose/src/observer_impl.rs`
- `crates/sc-compose/src/observability.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- F5-D1 — Introduce a typed internal log-record/event-builder value or equivalent focused builders so observer callbacks no longer pass seven loosely related arguments into emit_log.
- F5-D2 — Separate command-lifecycle mapping from composition-pipeline mapping while preserving target, action, outcome, level, message, fields, diagnostic, and schema-version behavior.
- F5-D3 — Keep normalize_event_labels and fallback behavior explicit, validated, and observable when a requested label is invalid; do not hide contract failures by silently changing labels.
- F5-D4 — Add table-driven coverage for resolve/include/validate/render, command start/end, success/failure/warning outcomes, diagnostic fields, JSON-mode cleanliness, logger degradation, and shutdown behavior.
- F5-D5 — Rendered plan evidence: `sc-compose render --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --var-file var-files/phase-f-5.json --output docs/phase-F/sprint-f-5-observer-event-builder.md` exits 0.
## Required Work

- Keep CliObserver implementing the existing sc-composer CompositionObserver, ObservationSink, and CommandLifecycleObserver traits; the refactor is internal to sc-compose.
- Make the event builder own normalization inputs and fields construction only where that improves cohesion; retain the concrete Logger submission and graceful shutdown semantics in the CLI adapter.
- Preserve --json console suppression, logger health behavior, file sink behavior, event messages, validated newtypes, fallback labels, and schema version.
- Use the existing event tests as characterization coverage, then add a matrix so every callback has explicit success and failure expectations.
- Do not make sc-composer depend on sc-observability or ATM, and do not replace the trait-injection model with a runtime-specific library hook.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
struct LogRecord {
    level: Level,
    target: &'static str,
    action: &'static str,
    message: &'static str,
    outcome: Option<&'static str>,
    fields: Map<String, Value>,
}

impl CliObserver {
    fn emit_record(&self, record: LogRecord);
}
```
The exact ownership may use an enum for command/pipeline event families, but callbacks must construct a typed record and the common emitter must remain responsible for envelope creation, label normalization, logger submission, and ignoring sink errors as today.
## This Sprint Does Not Close

- This sprint does not change the sc-composer observer trait contract, event names/messages, observability schema, logging levels, or public CLI behavior.
- This sprint does not move concrete logger wiring into sc-composer or bindings/python and does not add ATM/runtime dependencies.
- This sprint does not remove the documented fallback behavior or turn static schema/service validation failures into recoverable user errors.
## Acceptance Criteria

- All observer callbacks use the typed record/builder seam or an equivalent cohesive internal abstraction; emit_log is no longer a seven-parameter primitive-obsession hotspot.
- Success, warning, and failure events retain current target/action/outcome/message/field values and validated schema/service labels.
- Command lifecycle and composition pipeline events remain distinguishable and are both covered by table-driven tests.
- JSON mode remains stdout-clean, logger shutdown is preserved before process exit, and degraded logger behavior remains non-fatal as currently specified.
- The sc-composer pure-library and standalone observability dependency boundaries remain unchanged.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test cli observability_health_text_reports_process_local_status`
- `cargo test -p sc-compose --test json_cli observability_health_json_uses_diagnostic_envelope_and_stays_stdout_clean`
- `git diff --check`