# ADR-0024: Advanced Beads Pour Without Delegated Variable Substitution

## Status

Approved requirements in FR-24; API and routing details remain proposed until
the unresolved choices below are recorded. Updated for the user's consolidated
requirements of 2026-11-04. No implementation or test completion is implied.

## Decision

sc-compose constructs loops, includes, conditions and variables before bd
validates and creates beads. Advanced rendering is the default. The CLI
accepts `--render-by sc-compose` or `--render-by bd`; omission selects
sc-compose. The bd selector preserves the v1.7.0 API behavior, rendered bytes,
command lines and error codes described by FR-24.24. Schema-less general
rendering remains byte-identical to v1.7.0 except user-approved genuine bug
fixes with linked issues. Global strict-only rendering is excluded.

Inputs are data and never become template source. Advanced mode sends no
`--var` values to bd; actual created-value readback verifies literal safety,
including labels. Omission of --var alone does not prove safe transport.
Existing Rust request struct literals compile unchanged; the implementation
must use an additive option/entrypoint, with agreed Rust/Python spelling.
The real orchestration formulas, include defaults, graph/path refusal,
authorization, atomicity and retry behavior are FR-24 acceptance obligations.

## Plan-fix command

FR-24.33 adds a direct CLI equivalent of bd mol pour with repeated --var:

```sh
sc-compose bead pour <plan-fix-formula> --var key=value
```

The existing --request JSON command remains supported. The direct form must
resolve a real named/path formula, use the approved advanced default, and
verify created values through the same library execution contract; it is not
a separate renderer or a delegation of variables to bd.

FR-24.34 requires this workflow to create declared edges automatically as
part of the supported graph operation, with stored-edge readback. Existing
FR-23/ADR-0023 authorization, validation, atomicity and retry semantics apply;
no subsequent manual dependency commands or implicit scheduling edges are
part of successful completion.

## Existing boundaries

Extends ADR-0021 and ADR-0023. General composition stays in sc-composer;
Beads graph/process policy stays in sc-composer-beads; CLI/Python are thin
adapters. Reuse reviewed implementations and tests already in develop or the
recovered source; only missing behavior and necessary corrections are work.
The earlier opt-in-only recommendation is superseded by the approved default.
No ATM runtime dependency or duplicate parser/framework is introduced.

## Open decisions and evidence

FR-24's open-decision section governs unresolved selector API, collision and
default rules, literal-safe evaluation/transport, validation interface, corpus
identity and regression recovery. These are prerequisites of affected changes,
not permission to pick a changed public default or required request field.
Historical bd 1.3.2-dev probes are qualified observations, not universal
substitution guarantees. The named FUZZ-171/T/D cases must be recovered and
mapped to real tests; a missing historical test cannot count as PASS.

## Consequences

Quality-mgr reviews existing and retained implementation as well as new code.
Documentation covers both rendering modes through created-bead examples.
This ADR is a product contract, not a new publishing or process gate.
