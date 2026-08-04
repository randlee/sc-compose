---
id: J.2
title: Validation State and Context Assembly
phase: J
status: planned
branch: sprint/j-2-validation-state-assembly
worktree: ../sc-compose-worktrees/sprint/j-2-validation-state-assembly
target: integrate/phase-j
---

# Sprint J.2 — Validation State and Context Assembly

## Purpose

Reduce `crates/sc-composer/src/validation.rs`'s hot-spot risk (Repowise score
2.37, issue #212) by extracting `ValidationState` assembly — frontmatter and
default merging, per-pass token discovery maps, required-path origins,
variable-source precedence, and built-in environment/date injection — into a
dedicated state module, without changing validation behavior. This is the
highest-risk sprint in Phase J: I.5 (loop-context built-ins) recently changed
this exact state/discovery boundary, and `ValidationState` is directly
consumed by `composer.rs`.

## Dependencies and exact targets

- `crates/sc-composer/src/validation.rs:46-57` (`ValidationState` struct);
- `crates/sc-composer/src/validation.rs:525-769` (state assembly: frontmatter/
  default merging, per-pass discovery maps, required origins, precedence,
  built-in injection);
- the coupling to `discovery::discover_all_pass_tokens` — this sprint may
  move *where* that call happens but must not alter discovery semantics;
- `composer.rs`'s consumption of `ValidationState` — must not require a
  call-site or shape change.

Depends on J.1 only for phase sequencing (no code dependency); must land and
have its characterization suite passing before J.3 begins.

## Deliverables

- Freeze a `ValidationState` shape contract (documented field-by-field: what
  each field means, who populates it, what invariants hold) *before* moving
  any code — this is a decomposition safety requirement, not documentation
  afterthought.
- Move `ValidationState` construction, per-pass discovery-map assembly,
  required-path origin tracking, variable-source precedence resolution, and
  built-in environment/date injection into a new state module (e.g.
  `validation/state.rs`), reachable only through crate-private or private
  APIs — no new public surface.
- Preserve `composer.rs`'s existing consumption of `ValidationState`
  unchanged in shape and behavior.
- Do not alter `discovery::discover_all_pass_tokens` semantics; this sprint
  relocates the caller, not the callee.
- Add characterization tests for `ValidationState` assembly covering: I.5
  loop-context discovery output, default-merge precedence, required-path
  origin attribution, and built-in injection — captured against the current
  (pre-move) behavior before any code moves.

## ValidationState contract to freeze before the move

The contract belongs in the implementation evidence for this sprint and must
be reviewed before code relocation:

| Field | Populated by | Invariant |
| --- | --- | --- |
| `context` | frontmatter/default merge, then request inputs and built-ins | precedence remains explicit input > environment > built-in > input defaults > frontmatter defaults |
| `variable_sources` | every context insertion path | each retained value has its actual winning source |
| `required_origins` | required-variable declarations | first declaration origin is retained for diagnostics |
| `required_include_chains` | expanded include graph | origin diagnostics preserve the include chain for the declaring file |
| `default_origins` | frontmatter and request defaults | default-use diagnostics point to the owning frontmatter when applicable |
| `default_pass_numbers` | parsed root passes | pass-scoped defaults map only to their declared pass |
| `declared_variables` / `referenced_variables` | declarations and token discovery | top-level and dotted names preserve current set semantics |
| `*_by_pass` maps | `discover_all_pass_tokens` and parsed passes | loop-context discovery and brace-width/pass isolation are unchanged |

The implementation seam must preserve the existing crate-private signatures
used by `composer.rs`, including `inject_builtin_vars` and the `ValidationState`
fields consumed while building render contexts.

The current struct shape is frozen for this sprint. Any field, type, or
visibility change requires a separately reviewed follow-up ADR rather than an
implicit change during module relocation:

```rust
#[derive(Debug, Default)]
pub(crate) struct ValidationState {
    pub(crate) context: BTreeMap<VariableName, InputValue>,
    pub(crate) variable_sources: BTreeMap<VariableName, VariableSource>,
    pub(crate) required_origins: BTreeMap<VariableName, PathBuf>,
    required_include_chains: BTreeMap<VariableName, Vec<PathBuf>>,
    default_origins: BTreeMap<VariableName, Option<PathBuf>>,
    default_pass_numbers: BTreeMap<VariableName, BTreeSet<usize>>,
    pub(crate) declared_variables: BTreeSet<VariableName>,
    pub(crate) referenced_variables: BTreeSet<VariableName>,
    referenced_variables_by_pass: BTreeMap<usize, BTreeSet<VariableName>>,
    declared_variables_by_pass: BTreeMap<usize, BTreeSet<VariableName>>,
}
```

## Acceptance criteria

- Every existing validation diagnostic (code, severity, message, order,
  location, include-chain attribution) is unchanged for the full existing
  fixture set.
- The full I.5 loop-context regression suite
  (`validation::tests::strict_mode_accepts_approved_loop_context_builtins`
  and siblings) passes unchanged, run both before and after the move.
- `discovery.rs` is not modified by this sprint.
- `composer.rs` requires no call-site changes.
- The phase-level Repowise rescan and NLOC-evidence condition is governed by the [Phase J Exit Gate](phase-J-plan.md#exit-gate) and remains non-blocking.

## Required validation

Use the [Phase J authoritative validation
checklist](phase-J-plan.md#authoritative-validation-checklist), including
the additional J.2-specific requirement to re-run the full I.5 loop-context
regression suite unchanged.

## Removal path

If any characterization test fails post-move, or if `composer.rs` integration
breaks, revert to the single-module `validation.rs` state assembly and keep
only the frozen state-shape contract and added characterization tests as
documentation for a future retry.

## Out of scope

- any change to `discovery.rs` semantics or its public surface;
- `validate_expanded`'s diagnostic-policy and required-path collector logic
  (owned by J.3, which depends on this sprint's frozen state contract);
- `crates/sc-composer/src/extract/*` (excluded from Phase J entirely).
