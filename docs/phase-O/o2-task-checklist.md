# O.2 checked-render completion checklist

This is the local implementation and closure checklist for Sprint O.2. Each
item is checked against `sprint-o-2-checked-render-contract.md` and the
authoritative contract in `phase-O-plan.md`.

## Initial audit

- [x] O2-001: Add the library-owned format-aware output checker, typed parser
      error, stable line/column/byte-offset diagnostic, and state-shaped
      `RenderCheckReport`/`CheckedOutput` API.
- [x] O2-002: Add the stable malformed-render diagnostic code and update the
      exhaustive diagnostic compatibility table.
- [x] O2-003: Add canonical `--check-render` flags to `render` and `validate`;
      make plain `validate` explicitly report `static_only` without rendering.
- [x] O2-004: Gate ordinary JSON render output before any stdout/file write,
      including JSON envelopes, custom delimiters, `--all`, guidance, and
      prompt paths.
- [x] O2-005: Implement `validate --check-render` and
      `validate --lint --check-render` using the same checker and exact request
      context, with no body/file emission.
- [x] O2-006: Expose the ATM-core machine contract and migration guidance
      without adding ATM runtime dependencies.
- [x] O2-007: Add the complete O.2 test matrix: valid JSON shapes, parser
      locations and secret redaction, emission refusal, report states, CLI
      envelopes, multi-pass failures, non-JSON compatibility, and legacy/auto
      JSON behavior.
- [x] O2-008: Re-read the sprint plan and perform a closure review after the
      first implementation pass; add and resolve any newly discovered gaps.
- [x] O2-009: Run `just test`, workspace quality gates, and repository lint;
      record evidence before commit and handoff.

## Closure review

Second-pass findings and closure:

- Added multi-pass failure attribution to the stable JSON parser diagnostic and
  covered it with a JSON-envelope integration test.
- Added checked-vs-unchecked non-JSON compatibility coverage.
- Added the missing FR-7/FR-8/FR-8a, CLI manual, and ATM adapter guidance for
  fail-closed output, static-only validation, structured states, and redacted
  diagnostics.
- `cargo test --workspace`, `cargo fmt --all --check`, `cargo clippy
  --all-targets --all-features -- -D warnings`, `git diff --check`, and
  `just test` pass.
- `sc-lint version` reports real `sc-lint 0.4.0`. `just lint` was attempted
  through the real sc-compose/sc-lint integration but is a local configuration
  failure because CI-only `.just/lint_cargo_deny.py` has not been materialized;
  the repository's setup action downloads those pinned utilities in CI. No
  lint suppression or ignore attribute was added for this implementation.
