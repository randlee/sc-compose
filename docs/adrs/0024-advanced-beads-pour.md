# ADR-0024: Advanced Beads Pour Without Delegated Variable Substitution

## Status

User-approved requirements in FR-24 and the concrete Phase V design contract
below are recorded for plan review (2026-11-04). Implementation is pending.
No remaining product-policy choice is inferred from an unspecified API;
the bd transaction capability is an explicit implementation prerequisite.

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

## Phase V design contract

The public sc-composer ComposeRequest in crates/sc-composer/src/types.rs
gets no new public field; its existing layout is unchanged. The additive
BeadRenderBy and BeadExecutionOptions types will be defined by V3 in
crates/sc-composer-beads/src/contract.rs and re-exported from lib.rs;
options/runner entrypoints will be in execute.rs. BeadComposeRequest is unchanged.

The request layout stays unchanged. Rendering owner and strictness are
additive execution options; the existing functions will delegate to the same
engine with the advanced default. Explicit Bd options select the preserved
v1.7.0 path. There is no second renderer, parser or process engine.

```rust
pub enum BeadRenderBy { ScCompose, Bd }
pub struct BeadExecutionOptions {
    pub render_by: BeadRenderBy, // Default: ScCompose
    pub strict: bool,           // Default: false; only schema-declaring templates
}
pub fn execute_bead_request_with_options(
    request: &BeadComposeRequest,
    options: &BeadExecutionOptions,
) -> Result<BeadComposeReceipt, BeadComposeError>;
pub fn execute_bead_request_with_options_and_runner(
    request: &BeadComposeRequest,
    options: &BeadExecutionOptions,
    runner: &dyn ProcessRunner,
) -> Result<BeadComposeReceipt, BeadComposeError>;
```

Existing public entrypoints and struct literals remain available. V3 adds the
new facade/composition roots to the Beads and Python boundary manifests;
allowed crate dependency edges are unchanged. The runner option provides
controlled inputs in tests, not another execution path.

Python retains the positional request and adds the same keyword-only execution
options to each existing entrypoint. The convenience entrypoints continue to
select their named operation; `execute` honors `request.operation`:

```python
execute(request, *, render_by="sc-compose", strict=False)
render(request, *, render_by="sc-compose", strict=False)
validate(request, *, render_by="sc-compose", strict=False)
preview_pour(request, *, render_by="sc-compose", strict=False)
pour(request, *, render_by="sc-compose", strict=False)
preview_attach(request, *, render_by="sc-compose", strict=False)
attach(request, *, render_by="sc-compose", strict=False)
```

`render_by` accepts `"sc-compose"` or `"bd"` and maps to
`BeadRenderBy::ScCompose` or `BeadRenderBy::Bd`; `strict` maps to
`BeadExecutionOptions::strict`. The default is advanced sc-compose rendering
with strict checks disabled. These are additive keyword-only arguments; the
existing positional request and its layout do not change.

CLI and JSON requests map to these options. `--request` is mutually exclusive
with a positional formula and direct --var inputs; it is not silently merged.
Repeated distinct --var entries map to string-valued bead_variables. Duplicate
keys are request errors before mutation, as duplicate JSON bead variables
already are. Reuse cli::parse_var, not a second key=value parser. Compose/bead
name collisions are refused rather than silently overwritten; explicit caller
values win over root defaults and scoped included defaults. An include default
fills an absent caller/root value and remains literal.

```text
sc-compose bead pour plan-fix --var plan=phase-v --var finding=EDGE-1
    -> BeadOperation::Pour, bead_variables={plan:"phase-v",finding:"EDGE-1"}
    -> BeadExecutionOptions { render_by: ScCompose, strict: false }
sc-compose bead pour --request request.json --render-by bd
    -> existing request layout, explicit preserved bd-owned execution
```

The formula operand accepts an explicit confined path or a registry name.
Named resolution follows bd's documented active/repo/user/shared search
order; bd formula list --json exposes name/source for native-listed files.
Raw .j2 sources are resolved without parsing them as native TOML before
sc-compose expands them. Reuse existing path/source confinement and snapshot
helpers; do not implement another TOML engine. The direct adapter allocates
its rendered_formula inside a private workspace staging directory and maps
that path into the same request used by the JSON API.

V3 adds a versioned worked example at
examples/beads/plan-fix.formula.toml.j2 and pins that exact file in its tests.
It includes caller variables, a review/fix dependency, labels and literal
brace data. The registry-name test registers that same source in an isolated
bd registry; the path test uses the explicit source. Neither claims a missing
historical production formula was recovered. Unresolved formula, duplicate
var, mixed request/direct inputs and missing required input all create none.

## Remaining decisions and evidence limits

The owner selected additive edges between existing beads, including new
relations after formula changes, plus optional beads/edges controlled by
Jinja. FR-24.34–35 and the ADR-0023 Phase V amendment govern provenance v2,
state preservation, conditional selection and one atomic apply. V1 must
verify/pin the bd transaction capability before V3's affected apply work;
that is an implementation dependency, not an unresolved product decision.

The FUZZ-171 cases and the 229-template corpus are unavailable historical
evidence. They are never counted as recovered, rerun or passing, and their
recovery is not a prerequisite for any current work. The 2026-11-05 user
ruling replaces them with bounded, deterministic regressions of the current
approved contract, owned by V3, with final reruns owned by V7. The only
remaining pinning obligation is the v1.7.0 byte compatibility compared under
FR-24.24: those bytes must be pinned from the preserved sources, and missing
evidence there stays a narrow unresolved acceptance input, never a passing
test. Historical bd probes stay version-qualified.

## Consequences

Quality-mgr reviews existing and retained implementation as well as new code.
Documentation covers both rendering modes through created-bead examples.
This ADR is a product contract, not a new publishing or process gate.

FR-24.36 and ADR-0023 govern identifier/argv safety, graph-plan diagnostics,
missing non-UTF8 request-path handling and request/authorization error precedence.
V3 tests paired permitted/refused cases; no generic template-input ban is added.

FR-24.14 transport acceptance inspects the exact formula/graph bytes passed
to bd: all source template expressions are evaluated before handoff, with no
executable expression left for bd. Paired literal-delimiter values remain
unchanged data under FR-24.18; a blanket delimiter-byte ban is invalid.
The V3 contract and isolated-bd tests cover both cases.
