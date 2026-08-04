# ADR-0014: Phase-J Maintainability Decomposition Boundaries

## Status

Accepted

## Context

GitHub issue [#212](https://github.com/randlee/sc-compose/issues/212)
identified maintainability hotspots in `sc-compose/src/cli.rs`,
`sc-composer/src/validation.rs`, and `sc-composer/src/frontmatter.rs`.
Phase J addresses that debt through behavior-preserving structural
decomposition. The phase must preserve the shipped CLI, validation, extraction,
diagnostic, and public Rust/Python contracts while making ownership boundaries
reviewable.

## Decision

Phase J uses four implementation sprints in this order:

1. **J.1** splits CLI schema, pass-input normalization, and capability mapping
   behind the existing `crate::cli::*` surface.
2. **J.2** extracts validation-state and context assembly, and freezes the
   `ValidationState` contract before moving code.
3. **J.3** extracts validation policy and required-path diagnostics after J.2's
   state contract and characterization coverage are available.
4. **J.4** splits frontmatter parsing and normalization after J.2 and J.3's
   characterization coverage is available.

J.1 is independent. J.2 precedes J.3, and J.4 follows both J.2 and J.3.
These are implementation sprints, not planning/design sprints; each uses the
full Phase J validation checklist.

### Frozen `ValidationState` contract

J.2 must preserve this crate-private shape and its field invariants:

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

The existing precedence, source attribution, required/include origin
attribution, pass isolation, and loop-context discovery invariants remain
unchanged. Any field, type, or visibility change requires a separately
reviewed follow-up ADR; it may not be introduced as part of the Phase J move.

### Public-surface and boundary invariant

New implementation files are private submodules. Existing public and
crate-public paths remain the only supported import surfaces: `crate::cli::*`,
`Frontmatter`, `ParsedTemplate`, `parse_template_document`, and the existing
validation entry points. Phase J creates no public `context`, `tokens`, or
alternate validation/frontmatter APIs, and no downstream call site changes are
required.

`discovery.rs` remains unchanged in contents, public surface, and semantics.
J.2 may relocate a caller elsewhere in the crate only when the existing
exported discovery function and behavior are preserved. Extraction adapters,
diagnostics, and shared types remain outside the decomposition target.

## Consequences

- The sprint order makes the highest-risk state contract explicit before
  policy and frontmatter moves depend on it.
- Characterization tests and re-exports provide evidence that decomposition
  does not become an accidental API or behavior change.
- Repowise results are a quality diagnostic after integration, while sprint
  closure is based on concrete decomposition evidence and full validation.
- Future public-surface or `ValidationState` changes require their own scoped
  decision rather than being hidden in a maintainability refactor.

## References

- [Phase J plan](../phase-J/phase-J-plan.md)
- [Sprint J.1](../phase-J/sprint-j-1-cli-argument-seams.md)
- [Sprint J.2](../phase-J/sprint-j-2-validation-state-assembly.md)
- [Sprint J.3](../phase-J/sprint-j-3-validation-policy-diagnostics.md)
- [Sprint J.4](../phase-J/sprint-j-4-frontmatter-parser-split.md)
- [Issue #212](https://github.com/randlee/sc-compose/issues/212)
- [Architecture](../architecture.md)
