# ADR-0024: Advanced Beads Pour Without Delegated Variable Substitution

## Status

**Draft / Proposed (2026-11-04).** Requirements are FR-24 in
[requirements.md](../requirements.md). User-directed requirements are separated
from the open design choices below. This is documentation only; quality-mgr
reviews the draft before any advanced-render port. No API spelling, algorithm
or routing choice in Open decisions is silently approved by this draft.

Extends [ADR-0021](0021-beads-formula-composition-integration.md) and
[ADR-0023](0023-beads-attach-and-by-path-pour.md) only through an explicit new
opt-in. Their existing default paths remain in force. The user places this
work after the Phase U upgrades, followed by a v1.7.0 differential and a closing
regression exercise. This ADR is not a new publish or process gate.

## Context

The purpose of sc-compose Beads support is **advanced pour**: sc-compose can
construct loops, structure and variables beyond what bd supports natively;
bd receives flat output and validates/creates beads. The user has ruled that
bd variable substitution is buggy and cannot be a dependency of the advanced
path. A successful `bd mol pour --var` exit is not proof that every intended
field received a value.

Issue [#618](https://github.com/randlee/sc-compose/issues/618) reported ordinary
placeholders left in created titles/descriptions. Later investigation found a
native tight/padded-brace difference. Comp's recorded bd1.3.2-dev407fe27 probes
also show field-dependent behavior: title/description/notes substitute,
while labels retain placeholders. Exact commands/results and binary checksum are in FR-24's bd evidence
table. The old padded-title defect did not reproduce on this build;
arbitrary fields/escaping/platforms remain untested, not inferred from help.

Issue [#785](https://github.com/randlee/sc-compose/issues/785) concerns frontmatter
consumption and false references in comments. Frontmatter stripping already
exists in v1.7.0; that is not a reason to add mandatory declarations or repeat
the implementation. Issue [#791](https://github.com/randlee/sc-compose/issues/791)
asks for declaration checks and triple-brace validation. Its global strict-only
request conflicts with the current user compatibility ruling and is not
adopted. Explicit schema validation can address that concern without changing
schema-less defaults.

The historical commits listed below show intended capabilities and tests, not
permission to change the v1.7.0 default or break Rust struct literals. Their
sequential output-rendering and marker algorithms are not accepted merely by
copying the commits.

## How a pour works today

```text
v1 request + template + compose_variables + bead_variables
                              |
         sc-compose expands includes / Jinja blocks
         and {{{ compose expressions }}}
                              |
          rendered .formula.toml or .formula.json
           {{ ordinary expressions }} remain text
                              |
                existing operation/path selection
          +-------------------+--------------------+
          |                                        |
    active-registry pour                   by-path graph / attach
    bd cook / bd where                     bd parses rendered formula
    bd mol pour --var ...                  sc-compose builds graph plan
    native bd substitution                 bd create --graph [--dry-run]
          |                                        |
          +---------------- persistent beads ------+
```

`render` only writes its declared rendered file. `validate` renders and invokes
bd validation. A preview may create its declared local artifacts but never
persistent Beads state. Registry pour uses bd's native formula route; by-path
pour creates a molecule root and steps; attach creates stable children under
an existing bead. ADR-0023 owns stage details, private input snapshots,
authorization, atomicity, provenance and retry/conflict rules.

bd natively parses formula declarations and has its own substitution,
`extends`, `loop`, `expand`, condition and gate constructs. Their results and
limits are specific to the pinned version. ADR-0023 records, for bd 1.3.1,
that parsing may expand some native constructs before sc-compose sees them,
and that expanded identifiers may fail the graph id rules. This is not a
promise that bd 1.3.2 behaves identically or that all fields support native
substitution. bd does not execute the full sc-compose/MiniJinja template
language, and its field-dependent substitution is insufficient for advanced
pour's final-value guarantee.

## Decision: requirements fixed by the user

1. **sc-compose constructs; bd validates and creates.** The advanced path
   must finish source loops, includes, conditions and variables itself. It
   must not quietly delegate a remaining expression to bd.
2. **No reliance on native substitution.** Advanced data reaches bd with no
   unresolved `{{ ... }}` expression for bd to touch. Literal brace-shaped
   data requires a specifically tested literal-safe transport or escape;
   omitting `--var` alone is not an assumed proof of safety.
3. **Legacy defaults are preserved.** Existing schema-less calls remain
   byte-identical to v1.7.0 and retain normal MiniJinja/policy behavior,
   preserving only the already user-approved Phase U compatibility exceptions.
   Changed ordinary-variable rendering is explicitly opt-in. No global
   strict-only environment, removed policy method/field or implicit schema.
4. **Data never becomes template source.** Caller/default/include values and
   expression results are not reparsed as Jinja or bd expressions. Filters
   must see the intended data, not a distorted protection marker.
5. **FR-23 safety remains.** Paths, authorization, preview semantics, stable
   ids, provenance, atomic graph creation, conflicts and existing codes are
   preserved; graph/attach bead inputs are a new explicitly selected
   sc-compose capability, not a silent change to old graph requests.
6. **No implementation by assumption.** Use real pinned bd evidence for
   semantics and limits. Unknown behavior and unsourced design choices stay
   open. No upstream issue is filed without the user's decision.

### Ownership after explicit advanced selection

```text
original template source + inputs + included defaults
                           |
              sc-compose evaluates source
            (structure + variables + escaping)
                           |
            flat final fields / graph values
                           |
       bd syntax/graph validation, then one authorized create
```

`sc-composer` remains a pure general renderer. `sc-composer-beads` owns the
host-neutral request/options integration, bd argv/receipts and existing graph
mapping. CLI/Python are adapters, not independent render engines. bd owns its
input parser, accepted graph/storage rules and persistent state; sc-compose
does not access the database or import Beads/ATM runtime packages.

The production bd pin remains ADR-0021/0023's unless explicitly updated;
the1.3.2-dev probe does not itself advance that pin. The native parser
boundary remains ADR-0023's: no new TOML parser/dependency
or copied full Beads schema is authorized here. The method of proving that an
advanced input is flat, especially when native cook might expand constructs,
is part of Open decision 3 and must be settled before claiming that behavior.

## Modes and worked examples

The names below describe semantic modes. New public field/flag/API names are
**not selected**; Open decision 1 owns them. Examples describe desired output,
not a claim the advanced API is already implemented.

### 1. Legacy default / bd-owned expressions

Inputs: `compose_variables = {"project":"app"}`,
`bead_variables = {"release":"1.8.0"}`; no new option.

```toml
formula = "release"
[[steps]]
id = "build"
title = "{{{ project }}} {{ release }}"
```

sc-compose's rendered text remains:

```toml
formula = "release"
[[steps]]
id = "build"
title = "app {{ release }}"
```

Registry pour preserves its `--var release=1.8.0` route. Actual native
substitution, including its defects, is documented from the tested bd version;
it is not an advanced-mode guarantee. With the legacy graph/attach path,
nonempty bead_variables still produce the existing FR-23 refusal.

### 2. Explicit advanced variable rendering and source structure

Inputs: structured `items = [{"id":"build","title":"Build"},
{"id":"verify","title":"Verify"}]`, scalar bead input `release="1.8.0"`.
The caller explicitly selects advanced rendering; there is no frontmatter
requirement solely for using it.

```toml
formula = "release"
{% for item in items %}
[[steps]]
id = {{{ item.id | tojson }}}
title = {{ (item.title ~ " " ~ release) | tojson }}
{% endfor %}
```

The logical flat output is:

```toml
formula = "release"
[[steps]]
id = "build"
title = "Build 1.8.0"
[[steps]]
id = "verify"
title = "Verify 1.8.0"
```

Both ordinary and compose expressions see the original loop-local binding.
No bd `--var`, loop or expression engine is needed to produce those values.
A pipeline that renders the loop first and later reparses emitted ordinary
expressions after the binding has disappeared is not an acceptable substitute.

### 3. Advanced by-path graph pour

Render the flat steps to `build/release.formula.toml` inside the workspace,
with `verify` needing `build`. Preview validates the proposed graph without
writing Beads state; authorized pour creates one molecule root plus the two
children and their blocks edge atomically. The receipt maps template step ids
to actual bead ids. The advanced caller's bead inputs have already been
consumed by sc-compose; they are not passed as bd `--var`.

This opt-in capability extends FR-23.1/23.8 for sc-compose-owned input values;
legacy graph requests and unsupported native formula constructs are not
silently reinterpreted. What an advanced request does when the destination is
inside the active registry is deliberately open, not a new path-routing
choice hidden in this example.

### 4. Advanced attach

For parent `proj-42`, ref `release`, and the same flat `build`/`verify` steps,
attach produces `proj-42.release-build` and `proj-42.release-verify` plus the
blocks edge. Repeating an unchanged attachment preserves status, notes,
claims and provenance and performs no unnecessary creation. A conflict is a
refusal, never an update/reset of an existing bead. Values must have the same
literal and ownership guarantees as advanced by-path pour.

## Literal values, escaping and defaults

Given a supplied value `literal {{ untouched }} {% if anything %}`, rendering
that value produces literal data, not a second variable lookup or conditional.
For `A{BC`, `lower`, `length` and a first-three-character slice must observe
`a{bc`, `4` and `A{B`. Source-generated brace text has the same data status as
caller values. Protection must not change replacement/filter semantics or
leak a marker. Normal format serialization applies once; the actual stored
value, not just an apparently valid TOML/JSON file, is the oracle.

Literal `{{ ... }}` in a final field is only safe to hand to bd through a
route supported by BD-V6 for its recorded graph/attach samples; BD-V4 demonstrates native corruption, not a safe route. Comp's BD-V6 demonstrates the recorded graph/attach samples preserve
literal title/description/notes/labels on407fe27; this draft does not
generalize that result to every value or invent an escape. The implementation
must not hide an unsupported path behind a passing renderer-only test.

Include-default example: a root supplies caller `release="1.8.0"`; an included
step template supplies default `caption="default {{ literal }}"`. Advanced
output must keep the caller release and preserve the caption as data under
the approved scope/precedence rule, rather than turn the default into source
or silently drop it. Root/include collisions, independent include contexts
and a genuinely missing value are Open decision 4; this example does not
silently set a precedence rule. Frontmatter is consumed, not emitted before
the formula body. Any new undeclared-reference check is separately explicit
and cannot invent a schema for a schema-less template.

## Failure behavior

| Condition | Required behavior |
|---|---|
| Bad new selector/options | Request/usage class, exit 3; reuse existing request code where applicable; no silent fallback to another renderer. |
| Render/default/declaration failure | `BEADS_RENDER_FAILED`; no subsequent persistent creation. Required/default semantics and any opt-in strict policy must be identified, not global strict-only. |
| bd rejects cooked formula | Existing `BEADS_COOK_FAILED` stage/receipt contract; retain actual version/exit/output. |
| Unsupported graph/native construct | Existing `BEADS_GRAPH_FORMULA_UNSUPPORTED` / id or relation errors under ADR-0023, not ignored structure. Flatness enforcement must be settled under Open decision 3. |
| No persistent-write authorization | Existing `BEADS_POUR_AUTH_REQUIRED` / `BEADS_POUR_AUTH_INVALID`; no creation. |
| Attach provenance/id/edge conflict | Existing graph refusal semantics; do not edit existing work. |
| Failure to preserve literal data | Does not satisfy advanced pour; do not report success based only on an exit code or fabricate an escape. |

Render/preview filesystem effects remain their declared FR-23 artifacts; a
no-DB-write preview is not falsely advertised as zero filesystem writes.
Receipts and error names remain the same across Rust, CLI and Python. A new
error condition/code, if needed, must be documented rather than improvised.

## Source trace and acceptance obligations

| Historical source | Intended behavior to cover | Compatibility limit |
|---|---|---|
| `66cfacf1` | #618 values materialized before bd | Default change is rejected; require explicit new mode. |
| `f0c876ea` | Values remain literal across rendering | Do not accept emitted-output reparsing as equivalent. |
| `56be73ef` | Filter behavior over brace-containing values | No visible marker/length/case corruption or hidden filter ban. |
| `d754230a` | bd-owned rendering choice | Do not add a required field to the public Rust request or make sc-compose ownership the default. |
| `cd363380` | CLI/request selection and native-mode checks | Tests need the compatible selector and real bd evidence. |
| `5743907a` | Honest bd limits in the manual/stubs | Historical tight/padded claims are version-specific. |
| `e1c086df` | Distinct fixtures for native/advanced ownership | Do not rewrite the old oracle merely to pass a changed default. |
| `5f2633ac` | Redirected native registry placeholder | Preserve legacy bd resolution/argv separately from advanced data. |
| `61366f2a` | Wire operations/path/error documentation | Document the approved contract, not unavailable fields. |

FR-24's numbered requirements each contain a source, bd evidence reference
and acceptance idea. The implementations must supply positive real-bd created
value checks, paired failure cases and a v1.7.0 differential. The closing
Phase U regression work also examines FUZZ-171-01..14, T-02..T-06, D-01 and
include-default #11 once the original reports are available. Those are
acceptance inputs; no prior fix or proof is imported merely by naming them.
No tests or implementation accompany this draft.

## Open decisions

1. **Selector and source-compatible API.** Options: optional wire/CLI choice
   plus a new Rust options executor, or a new versioned request envelope.
   Recommendation: additive options API with old executor/request layout
   unchanged and legacy behavior as default. Proposed names/signatures must
   be approved; the historical request-field addition is not source compatible.
2. **Compose/bead input name collisions.** Options: compose precedence,
   explicit duplicate refusal, or separate namespaces. Recommendation:
   explicit refusal until a caller-visible rule is approved. Do not adopt the
   old merge order silently.
3. **Advanced registry routing and literal-safe bd boundary — needs probe.**
   Options: verified flat graph dispatch even for registry destinations, or a
   verified literal-safe native formula path. Recommendation: prefer the graph
   writer if BD-V6 proves value preservation; no advanced dependency on bd
   substitution. Settle how to enforce flatness without an unapproved TOML
   parser/schema fork, especially if cook expands native constructs first.
4. **Include/default precedence and missing values.** Options: caller then
   root defaults with scoped include fallback, or explicit include contexts
   and conflict rejection. Recommendation: retain caller precedence and scope
   child defaults, subject to v1.7.0 characterization and include-default #11.
5. **Explicit schema-validation surface.** Options: caller policy/strict
   control or a dedicated per-request option. Recommendation: explicit
   per-request choice that has no global strict-only/default-schema effect.
6. **Literal-safe render evaluation.** Options: source-aware single
   evaluation or another strategy proving equivalent value/filter/scope
   semantics. Recommendation: never reparse emitted output. The old two-pass
   marker implementation is not automatically the design.
7. **Unclear bd 1.3.2 semantics — needs probe.** Use the completed BD-V/BD-S table
   for407fe27 and probe any broader value/field/platform/released-binary claim,
   including matching source documentation for the condition discrepancy. Confirmed candidate upstream bugs are
   listed for the user's decision; do not file them automatically.

8. **Historical regression inputs and policy (needs source/user resolution).**
   Original independent FUZZ-171/T/D reports have not been located in the
   searched worktrees/shared files. Preserved minimized tests exist in
   bdb3bdae/06fc2e2d/25e3678e; 171-10's original title/repro remains missing.
   Options: recover the originals, or obtain an explicitly approved equivalent
   requirement/repro from the owner. Recommendation: recover/approve the input
   before claiming that finding verified; never invent it or silently drop it.
   Historical171-14 duplicate rejection and D-01's changed default are not
   approved policies: evaluate them under the selected collision rule and
   explicit advanced mode, while preserving legacy defaults.

## Consequences and documentation

- Advanced capability does not require a Beads fork, database access or a
  stricter default renderer; source and existing caller compatibility stay
  explicit acceptance conditions.
- bd-owned compatibility is preserved but never marketed as a reliable
  advanced substitution engine.
- The implementation strategy, flat handoff and interfaces remain decisions
  to review, not unstated commitments. Unsupported transport behavior blocks
  the corresponding claim rather than silently weakening literal safety.
- Update `crates/sc-compose/docs/manual/bead.md` and adapter documentation
  after the choices are approved: pipeline, selector/default, variable and
  structure ownership, supported formats/fields, defaults/escaping, real bd
  limitations, path/attach routing and exact errors. Existing ADR-0021/0023
  text and FR-23 need an explicit narrow amendment if an accepted opt-in
  changes their currently blanket graph-variable rules.

## References

- [FR-24 and FR-23](../requirements.md)
- [ADR-0021](0021-beads-formula-composition-integration.md)
- [ADR-0023](0023-beads-attach-and-by-path-pour.md)
- [#618: bead variable substitution](https://github.com/randlee/sc-compose/issues/618)
- [#785: frontmatter and comment handling](https://github.com/randlee/sc-compose/issues/785)
- [#791: declaration and triple-brace validation](https://github.com/randlee/sc-compose/issues/791)
- v1.7.0 render/request/execute and graph fixtures; the nine historical
  commits in Source trace. Historical approval comments do not override the
  current compatibility and buggy-bd rulings.
