---
id: sprint-I.2
title: Customer-Facing Raw-Text Mode
phase: I
status: planned
branch: sprint/i-2-customer-raw-text-mode
worktree: ../sc-compose-worktrees/sprint/i-2-customer-raw-text-mode
target: develop
---

# Sprint I.2 — Customer-Facing Raw-Text Mode

## Purpose

Promote the Phase-H format-neutral matcher into a supported known-template
`raw` extraction mode for Markdown and arbitrary text. This sprint owns the
public seam that I.3 will reuse for XML block/mixed-content matching.

## Dependencies and exact targets

- I.1 contract accepted, including raw report/path and diagnostic semantics;
- shared matcher modules under `crates/sc-composer/src/extract/`;
- extraction request/report exports in `crates/sc-composer/src/extract/mod.rs`
  and `crates/sc-composer/src/lib.rs`;
- CLI extract command and format selector under `crates/sc-compose/src/`;
- Python bindings under `bindings/python/`;
- shared extraction diagnostics and registry.

## Deliverables

- Add the raw-text format adapter using the existing matcher, not a copied
  delimiter scanner.
- Expose `format="raw"` through the Rust request, `sc-compose extract`, and
  `sc_compose.extract_variables` Python binding.
- Define flat raw occurrence evidence (byte/line offsets or the I.1-approved
  equivalent) and preserve report ordering and ambiguity behavior.
- Keep XML/JSON/YAML/TOML structural adapters unchanged except for importing
  the shared seam they already use.
- Add Markdown fixtures with static text, one variable, multiple separated
  variables, adjacent-variable ambiguity, filters, excludes, escaped text,
  and delimiter-count behavior.
- Add CLI JSON diagnostics and Python exception/report parity tests.

## Acceptance criteria

- A known Markdown template and rendered Markdown recover the expected values
  through Rust, CLI, and Python with identical values, order, confidence, and
  diagnostics.
- A static mismatch, missing delimiter, unsupported Jinja statement, and
  adjacent ambiguous capture fail with the I.1-approved stable code.
- `format="raw"` never invokes an XML/YAML/JSON/TOML parser and accepts no
  file path or unknown-template mode.
- Existing format tests remain unchanged and pass; no adapter duplicates
  `match_raw_text` or its delimiter rules.
- Documentation and public help text describe raw mode as known-template
  matching, not rendering or arbitrary inverse-Jinja evaluation.

## Required validation

Use the [authoritative Phase I validation
checklist](phase-I-plan.md#authoritative-validation-checklist), then run the
focused Rust, CLI, and Python cases first. Preserve machine-readable
diagnostics in a reviewable evidence file and include one realistic Markdown
example plus one deliberately rejected input.

## Removal path

If raw mode is rejected at QA, revert only the `raw` selector, adapter,
surface wiring, and raw-mode fixtures; retain the Phase-H shared matcher for
the shipped structured adapters and keep the Phase-I contract decision
historical until a replacement is accepted.

## Out of scope

- XML block/mixed-content structural matching (I.3);
- dirty-prefix XML normalization (I.4);
- unknown-template discovery, loop reconstruction, typed-value inference, or
  arbitrary Jinja execution;
- changes to YAML var-file decoding.
