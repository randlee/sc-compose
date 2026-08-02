# ADR-0011: Known-Template Reverse Extraction and Phase-G Sprint Shape

## Status

Accepted

## Context

Phase G delivers a supported feature for known-template, XML-first extraction,
informed by prior reverse-extraction research. The reversible contract needs a
durable record for its string-only report, structural occurrence provenance,
and fail-closed handling of unsupported or ambiguous inputs.

The repository's normal naming convention also uses `X.#-py` companion sprints
for Python parity. Phase G has a different product need: Python is the first
customer of the extraction API, so its binding work is a first-class delivery
surface rather than a parity-only companion. Giving it a standalone contiguous
G.3 sprint therefore moves the existing CLI, corpus, and adversarial sprints to
G.4, G.5, and G.6.

## Decision

- Phase G supports known-template XML extraction for the documented scalar
  subset and returns rendered strings with structural occurrence provenance.
- A variable name that maps to more than one distinct structural occurrence is
  classified as `ambiguous`; the extractor emits no `values` entry for that
  variable and must never silently overwrite one occurrence with another.
- Phase G uses six contiguous standalone sprints:
  - G.1 — extraction contract and analysis model;
  - G.2 — deterministic XML extraction engine;
  - G.3 — Python extraction bindings for the first customer;
  - G.4 — CLI extract surface;
  - G.5 — corpus and regression closure;
  - G.6 — adversarial evidence and hardening.
- G.3 is not a `G.2-py` companion because it introduces the first customer
  surface and its own acceptance contract. This is a Phase-G exception to the
  default `X.#-py` companion convention; later phases continue to use the
  repository convention unless they record their own decision.
- G.1 establishes the report/error contract, G.2 implements the Rust library
  API, and G.3 may begin once that API is stable. G.4 depends only on G.1 and
  G.2 and may proceed independently of G.3 because the CLI delegates directly
  to `sc-composer`; G.3 does not gate CLI implementation. G.5 requires all
  three product surfaces, and G.6 follows G.5's corpus/regression gate.

## Consequences

- The six Phase-G docs and the phase/project indexes must retain these exact
  numbers, links, and dependency statements.
- Reviewers must treat the G.3 standalone number as intentional, not as a
  missing `-py` companion or an extra unnumbered sprint.
- Future Phase-G changes that alter the Python-first rationale or G.3/G.4
  gating must amend this ADR and the affected sprint documents together.
- The repository boundary test is a required Phase-G gate. It checks source
  imports and Cargo manifests so research-only extraction artifacts cannot
  become a production dependency and the library/adapter/CLI dependency
  direction remains enforced.
