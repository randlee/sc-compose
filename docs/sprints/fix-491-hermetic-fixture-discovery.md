---
id: FIX-491
title: Hermetic sc-lint fixture utility discovery
status: in_progress
branch: fix/491-hermetic-fixture-discovery
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/491-hermetic-fixture-discovery
target: develop
---

## Root Cause

Issue #491 found that the shared sc-lint integration-test helper walked every
ancestor of the repository root looking for `sc-lint/.just`. A developer's
unrelated local checkout could therefore supply Python utilities to a fixture
test. CI materializes the pinned utilities into the repository-local `.just`
directory first, so the polluted local path was not reproduced there.

## Fix Design

The test helper accepts only two ordered sources: an explicit
`SC_LINT_SOURCE_ROOT`, then `<checkout-root>/.just`. It must never traverse
ancestors or infer a sibling checkout.

```rust
fn try_sc_lint_just_root_in(
    checkout_root: &Path,
    source_root: Option<&Path>,
    required_files: &[&str],
) -> Option<PathBuf>
```

The function returns the first source whose `.just` directory contains every
requested utility. An absent explicit source and absent checkout-local runtime
return `None`; callers retain their existing actionable setup error.

## Required Changes / Tests

- `crates/sc-compose/tests/support/mod.rs`: remove ancestor-directory
  discovery and make the two permitted sources explicit.
- `crates/sc-compose/tests/sc_lint_lint_ci.rs`: add a self-contained
  regression with a fake ancestor `sc-lint/.just`, proving it is ignored while
  checkout-local and explicit sources remain usable.
- Required validation:
  - `cargo fmt --all --check`
  - focused `sc_lint_utility_discovery_is_scoped_to_explicit_or_checkout_local_sources`
  - `cargo clippy -p sc-compose --test sc_lint_lint_ci -- -D warnings`
  - `cargo test --workspace` in the CI environment after its pinned sc-lint
    setup action materializes the repository-local runtime

## Out of Scope

- Changes to production `sc-compose` source or lint execution behavior.
- Vendoring sc-lint Python utilities into this repository.
- Altering CI's pinned sc-lint setup action.

## Acceptance Criteria

1. An ancestor `sc-lint/.just` directory cannot affect fixture discovery.
2. `SC_LINT_SOURCE_ROOT` has priority over checkout-local `.just`.
3. Checkout-local `.just` remains a supported source when no explicit source
   is configured.
4. The focused regression, formatting, clippy, and CI-provisioned workspace
   suite pass.

## References

- Issue #491: hermetic fixture utility discovery.
- Issue #493: traceability follow-up.
- `docs/phase-L/sc-lint-bootstrap-contract.md` for the pinned-runtime
  contract.

## Current Implementation Evidence

- implementation commit: `22b8593`
- PR #492; QA-492-recheck passed (all four reviewers, CI 12/12 green)
