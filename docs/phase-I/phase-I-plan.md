---
id: phase-I
title: Raw Text and Extraction Gap Closure
status: planned
branch: integrate/phase-i
worktree: ../sc-compose-worktrees/integrate/phase-i
target: develop
---

# Phase I — Raw Text and Extraction Gap Closure

## Objective

Close the remaining customer-facing extraction and input-safety gaps identified
by GitHub issues [#193](https://github.com/randlee/sc-compose/issues/193),
[#167](https://github.com/randlee/sc-compose/issues/167), and
[#166](https://github.com/randlee/sc-compose/issues/166). Phase I promotes the
format-neutral raw-text matcher created in Phase H into the first-class mode
needed for Markdown and other text documents, then uses that contract to make
XML block/mixed-content extraction useful without weakening the known-template
and fail-closed boundaries.

The phase has four issue-derived outcomes:

1. XML block text and XML mixed content can be extracted when a known template
   contains a full-content placeholder (`#193` Gap 1).
2. A narrowly defined non-XML preamble can be normalized before rendered XML
   parsing (`#193` Gap 5), with observable recovery behavior.
3. Jinja loop-context values such as `loop.last` are not reported as
   undeclared variables inside a `for` scope, while a user variable named
   `loop` outside a loop remains subject to normal validation (`#167`).
4. YAML merge keys in variable files cannot silently discard inherited fields;
   the supported policy is explicit and diagnostic (`#166`).

The additional product work is the customer-facing raw-text mode. It is not
silently attributed to #193: it is recorded here because the product owner
identified raw text as the core use case for Markdown and because XML block
matching should reuse the same candidate-value matcher.

## Current baseline and authority

- Phase G's known-template XML contract remains authoritative until the
  contract gate in I.1 is accepted.
- Phase H's shared raw-text matcher and cross-surface report model are the
  implementation seam; Phase I must not fork a second matcher.
- The current XML adapter is scalar/text-node oriented and rejects a rendered
  child-node shape when the template has one placeholder. I.3 owns the
  structural extension.
- `discover_tokens_with_brace_count` already tracks loop bindings, but it
  treats loop-context names as ordinary identifiers. I.5 owns scope-aware
  built-ins.
- YAML var-file conversion currently unwraps tagged values in
  `input_value_from_yaml`; I.6 must make merge-key handling explicit before
  conversion rather than relying on `serde_yaml`'s representation.

Normative amendments are made by I.1 and its review gate, not inferred from
this plan. No runtime support is claimed merely because a sprint is listed.

## Sprint sequence and concurrency

1. [Sprint I.1 — Contract, Raw-Text Semantics, and Traceability](sprint-i-1-contract-and-traceability.md)
   freezes the raw-text, XML recovery, loop-built-in, and YAML merge-key
   policies; records the requirement/ADR/error-code changes; and produces no
   executable implementation.
2. [Sprint I.2 — Customer-Facing Raw-Text Mode](sprint-i-2-customer-raw-text-mode.md)
   promotes the shared matcher through the Rust, CLI, and Python surfaces,
   including Markdown/text use cases.
3. [Sprint I.3 — XML Block and Mixed-Content Extraction](sprint-i-3-xml-block-mixed-content.md)
   closes #193 Gap 1 using the I.1 contract and the shared matcher.
4. [Sprint I.4 — XML Dirty-Prefix Normalization](sprint-i-4-xml-dirty-prefix.md)
   closes #193 Gap 5 with a bounded, observable preamble policy.
5. [Sprint I.5 — Jinja Loop-Context Built-ins](sprint-i-5-loop-context-builtins.md)
   closes #167 in strict validation without making `loop` globally implicit.
6. [Sprint I.6 — YAML Merge-Key Var-File Safety](sprint-i-6-yaml-merge-key-safety.md)
   closes #166 by preventing silent data loss at the var-file boundary.

I.2 through I.6 depend on I.1's accepted contract. I.3 also depends on the
I.2 public matcher seam. I.4, I.5, and I.6 are otherwise independent and may
be developed, reviewed, QA-tested, and merged in parallel. A failed QA gate
for one independent sprint does not block starting another; only a missing
declared dependency blocks implementation. No sprint document creates an
implicit requirement to wait for every earlier sprint's QA result.

## Plan review and handoff

The plan is not implementation-ready until it passes four recorded reviews:

1. traceability review: each issue finding, product-directed addition,
   requirement, ADR, and acceptance case has one owner;
2. architecture review: the pure-library/wrapper boundary and shared matcher
   seam are consistent with the current code;
3. adversarial review: malformed, ambiguous, oversized, shadowed, and
   security-relevant inputs have explicit outcomes;
4. implementability review: exact files, tests, commands, dependencies,
   rollback paths, and cross-surface evidence are sufficient for an agent to
   execute without inventing scope.

After the fourth review, accepted sprint deliverables are converted into the
team's execution beads/tasks. Rejected or superseded plan text is removed or
marked historical in the same review change; it is not left as a second
authority.

## Hard boundaries

- `sc-composer` remains a pure in-memory library. It owns matching, parsing,
  extraction reports, and diagnostics; it performs no file I/O or CLI parsing.
- `sc-compose` owns files, flags, var-file loading, exit codes, and output
  shaping. It must not implement a second raw-text or XML matcher.
- Python remains a binding over Rust semantics. It must expose the same
  report, diagnostics, limits, and failure policy.
- Raw-text mode matches a known template and rendered text. It does not infer
  an unknown template, execute arbitrary Jinja, reconstruct loops, or recover
  original types.
- XML dirty-prefix recovery applies only to rendered input and only to the
  exact preamble class accepted by I.1. It rejects unmatched or truncated
  markup, multiple roots, post-root content, and malformed suffixes rather
  than attempting to repair them.
- YAML merge keys are not silently expanded by generic tagged-value
  unwrapping. I.6 must either implement a fully specified, bounded expansion
  or reject the construct with an actionable stable diagnostic; it may not
  preserve today's silent-loss behavior.

## Authoritative validation checklist

Every implementation sprint owns tests with the behavior it changes. The
minimum validation for each implementation sprint is:

- focused Rust unit/integration tests;
- `cargo test --workspace`;
- `cargo clippy --all-targets --all-features -- -D warnings`;
- Python binding tests for any public-surface behavior;
- CLI tests for flags, exit codes, and JSON diagnostics where applicable;
- `cargo fmt --all --check` and `git diff --check`;
- a focused adversarial boundary set covering the rejection path, not only
  the happy path.

I.2 and I.3 must include equivalent Rust, CLI, and Python cases. I.4 must
  prove that accepted preambles, malformed suffixes, declarations/comments,
  and multiple roots remain distinguishable. I.5 must include nested loops,
  loop built-ins, shadowing, and a `loop` reference outside a loop. I.6 must
  include inherited scalar and nested fields, anchor/alias controls, malformed
  YAML, and the exact #166 reproduction.

Sprint documents reference this checklist rather than restating these common
commands. Each sprint's document adds only its own evidence requirements.

## Exit gate

Phase I is complete only when:

- I.1's accepted requirements, architecture, ADR, and diagnostic registry
  amendments match the shipped behavior;
- raw-text mode is available through Rust, CLI, and Python with one shared
  matcher and documented Markdown/text examples;
- XML Gap 1 and Gap 5 have direct regression tests and diagnostics that do
  not broaden the malformed-input boundary accidentally;
- strict validation recognizes loop-context built-ins only in active `for`
  scopes;
- YAML merge-key behavior is deterministic, fail-closed or fully specified,
  and cannot silently produce missing inherited fields;
- all independent sprint QA records are complete, with no unreviewed
  implementation or documentation drift;
- the full workspace, Python, CLI, boundary, and formatting checks pass; and
- team-lead, quality-mgr, req-qa, and arch-qa can review each sprint from its
  authoritative document and evidence.

## Traceability matrix

| Source | Gap / decision | Owning sprint | Status | Verification |
| --- | --- | --- | --- | --- |
| #193 Gap 1 | block text and mixed-content value extraction | I.3 | complete/accepted | XML/Rust, CLI, Python corpus and negative-shape tests |
| #193 Gap 5 | bounded dirty-prefix normalization | I.4 | complete/accepted | accepted-prefix, malformed-suffix, multi-root and diagnostic tests |
| #167 | loop-context names in strict validation | I.5 | planned | nested-loop, shadowing, outside-loop and CLI strict tests |
| #166 | YAML merge-key var-file silent data loss | I.6 | planned | exact reproduction, policy/error, and preservation tests |
| I.1 contract gate | raw report/path shape, XML recovery, loop built-ins, and YAML merge-key policy | I.1 | complete/accepted | ADR-0013, FR/architecture amendments, registry, and docs-only diff gate |
| Product direction | customer-facing raw-text mode for Markdown/text | I.2 | planned | raw mode API/CLI/Python parity and Markdown fixtures |
