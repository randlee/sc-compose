---
id: H.7
title: Alias and Input-Limit Hardening
status: complete
branch: sprint/h-7-alias-input-limit-hardening
worktree: ../sc-compose-worktrees/sprint/h-7-alias-input-limit-hardening
target: integrate/phase-h
---

# Sprint H.7 — Alias and Input-Limit Hardening

## Goal

- Close the confirmed adversarial findings promoted from the Phase-H
  cross-format campaign and its QA follow-up.
- Preserve the accepted H.1 known-template contracts while making parser
  resource limits effective before expensive or recursive work can exhaust the
  process.
- Keep the Rust library authoritative for CLI and Python behavior; do not add
  a second adapter implementation in either wrapper.

## Hard Dependencies

- H.1 through H.6 are complete on the Phase-H integration baseline.
- The H.1 contract remains authoritative for JSON, YAML, TOML, and the
  retained XML behavior.
- The adversarial-fuzzing campaign protocol and promoted finding records are
  available for replay and regression coverage.

## Exact Targets

- `crates/sc-composer/src/extract/json.rs`
- `crates/sc-composer/src/extract/yaml.rs`
- `crates/sc-composer/src/extract/xml.rs`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `crates/sc-compose/tests/cli/extract.rs`
- `bindings/python/tests/test_smoke.py`
- `docs/phase-H/phase-H-plan.md`
- `docs/phase-H/sprint-h-7-alias-input-limit-hardening.md`
- `docs/project-plan.md`
- `docs/error-code-registry.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. No deliverable may be silently dropped or
partially deferred.

- H7-D1 — Mirror TOML's byte-size, nesting-depth, and occurrence-count
  protections in the JSON, YAML, and XML extraction adapters with the stable
  `ERR_EXTRACT_INPUT_LIMIT` diagnostic and equivalent Rust, CLI, and Python
  behavior. JSON and YAML depth checks must run during deserialization so the
  format adapters do not leak serde's internal recursion diagnostic.
- H7-D2 — Reject YAML anchors, aliases, and tags at token boundaries in block
  and flow syntax, while tracking single- and double-quoted state correctly
  across YAML escapes. Keep the existing YAML policy diagnostic stable.
- H7-D3 — Make YAML mapping matching use indexed rendered keys so occurrence
  matching is not quadratic in the number of mapping entries.
- H7-D4 — Enforce XML nesting depth incrementally while parsing, before a deep
  tree is fully materialized, and provide iterative cleanup for XML nodes and
  documents so deep rejected or partially built trees cannot overflow the
  process stack during destruction.
- H7-D5 — Promote the confirmed adversarial cases into the owning Rust and CLI
  suites, including 60,000-level XML process-safety coverage, and retain
  cross-surface smoke coverage for the stable input-limit code.
- H7-D6 — Register H.7 in the Phase-H numbered sprint plan and keep this
  document as the authoritative scope and acceptance record for the follow-on
  hardening work.

## Required Work

- Validate template and rendered byte sizes before parsing for every guarded
  format.
- Enforce depth before pushing a new XML parser frame. The parser must return a
  normal `ERR_EXTRACT_INPUT_LIMIT` error rather than materializing an
  unbounded tree.
- Enforce JSON and YAML depth while deserializing, before serde's internal
  recursion guard can replace the stable `ERR_EXTRACT_INPUT_LIMIT` diagnostic.
- Ensure XML cleanup is iterative for `XmlElement`, `XmlNode`, and
  `XmlDocument`; the cleanup defense must remain safe even if a deep tree is
  constructed before rejection.
- Cap captured occurrences at the shared 10,000 ceiling for JSON, YAML, and
  XML, and preserve the existing TOML boundary.
- Use an indexed mapping lookup in YAML for both expected-key lookup and
  unexpected-key detection.
- Replay the promoted alias, escaped-quote, size, occurrence, and deep-XML
  cases without modifying the separate fuzz-campaign artifact directory.
- Keep diagnostics, format selection, provenance, report shape, and all
  intentional H.1 boundaries unchanged outside this scope.

## This Sprint Does Not Close

- XML mixed-content extraction, dirty-prefix tolerance, or unknown-template
  identification.
- Best-effort/degraded parsing or a customer-facing raw-text mode.
- Typed-value recovery, Jinja evaluation, loops, branches, or schema
  inference.
- A new extraction algorithm in the CLI or Python wrapper.
- Generic refactoring of all format-limit helpers unless it is required to
  preserve the scoped behavior and validation gates.

## Acceptance Criteria

- JSON, YAML, and XML oversized, over-depth, and high-occurrence inputs return
  `ERR_EXTRACT_INPUT_LIMIT` without accepting unbounded work.
- YAML flow and block anchors, aliases, and tags are rejected consistently,
  including cases following flow delimiters and cases after escaped quotes.
- YAML matching no longer performs a rendered-vector scan for each template
  key.
- A 60,000-level XML CLI input exits with a controlled
  `ERR_EXTRACT_INPUT_LIMIT` diagnostic, without a signal or stack-abort.
- Existing PHV-1, PHV-2, and YAML occurrence-limit regressions remain green.
- H.7 is listed in `phase-H-plan.md` and this document accurately records its
  scope, ownership, non-goals, and validation evidence.
- H.7 is listed in `docs/project-plan.md`, and the input-limit registry lists
  JSON, YAML, and XML as its promoted format emitters.

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`

## Completion Evidence

The completion report must include the final commit, the promoted regression
test names, all required gate results, and confirmation that the worktree is
clean and the branch is pushed. QA must independently verify the deep-XML
process-safety case and the H.7 governance entry before this sprint is marked
complete.
