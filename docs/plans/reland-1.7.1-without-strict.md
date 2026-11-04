# Plan: Re-land 1.7.1 Beads Work Without Global Strict

Status: for QA review. Replaces re-deriving Phase U bead work from scratch.

## Goal

Ship the 1.7.1 Beads fixes. Non-bead behavior (`render`, `validate`, the
`sc-composer` library, the Python package) stays identical to v1.7.0.

## The one decision

1.7.1 broke consumers because of one decision: #783, "strict is the only render
mode", commit `8b6294f1`. It made strict validation global.

- **Reversed:** global strict (#783). Re-close #783 as won't-do so it is not
  implemented again.
- **Kept:** bead-scoped strict. #791 asks that bead render/attach/pour enforce
  front matter "always, with no option". It was implemented in `473c74d6` and
  stays.

Every conflict in this plan is resolved by that rule: strict inside
`sc-compose bead` / `sc-composer-beads`, v1.7.0 behavior everywhere else.

## Requirements source (no re-derivation)

| Requirement | Issue | Proof |
| --- | --- | --- |
| Bead variables render before bd, padded and tight | #618 | `66cfacf1`, `pinned_bd_*`, `render_substitutes_*bead_variables*` tests |
| Bead render consumes front matter, emits body only | #785 (part 1) | 1.7.1 bead tests |
| Bead render enforces front matter strictly; validate understands `{{{ }}}` | #791 | `473c74d6`, `validate_strict_discovers_triple_brace_tokens_with_brace_count` |
| One-pass bead render (values never re-parsed, raw stays literal, locals shadow, filters see braces, include defaults) | FUZZ-171 | `bdb3bdae`, `single_pass_render.rs`, `fuzz_171_*` |
| bd-owned rendering mode `--true-bd-render` | 1.7.1 Added | `d754230a`, `bd_mode_*`, `bd_variable_rendering_*` |
| Strict scan fixes (comments, keywords, filters) | #785 (part 2), #784, #792 | already on develop |
| Python checked JSON render | #787 | already on develop; consumer-visible, needs explicit sign-off |
| Non-bead behavior identical to v1.7.0 | intent | v1.7.0 differential harness |

The plain-language spec is the backup tag's `CHANGELOG.md` `[1.7.1]` Added and
Fixed sections. The Changed section is #783 and is excluded.

## Measured starting point (2026-11-04)

- `backup/develop-pre-recovery-20261104` (old develop) has all 1.7.1 work and tests.
- `43070a18` (recovered base, now develop) has v1.7.0 behavior plus dependency
  upgrades (minijinja =2.18.0, yaml_serde, sha2 0.11, similar 3, ulid 3,
  PyO3 0.30, UniFFI 0.31.2) and tooling. It has none of the bead features.
- Reverting `8b6294f1` on the backup tag: **6 conflicting files**: `CHANGELOG.md`,
  two `toml-workflow.formula.toml.j2` fixtures, `sc-composer-beads/src/render.rs`,
  `sc-composer-beads/src/execute.rs`, `sc-composer/src/discovery/identifiers.rs`.
- Merging `43070a18` into the backup tag: **7 conflicting files**: `CHANGELOG.md`,
  `Cargo.toml`, `bindings/python/tests/test_smoke.py`,
  `sc-compose/tests/cli/render.rs`, `sc-compose/tests/cli/validate.rs`,
  `sc-composer/src/discovery.rs`, `sc-composer/src/discovery/identifiers.rs`.

## Steps

1. Branch `reland/1.7.1-without-strict` from `backup/develop-pre-recovery-20261104`.
2. `git revert 8b6294f1`. Resolve the 6 conflicts by the rule above. Tests that
   only assert global strict leave with the revert.
3. `git merge 43070a18`. Resolve the 7 conflicts: dependencies and tooling from
   `43070a18`; bead code from the branch.
4. Run gates G1–G3.
5. Land on develop as one tree-equal commit (the method #843 used), so
   develop's history is not rewritten.

## Gates (all blocking)

- **G1:** `cargo test --workspace`, clippy, and the Python binding tests pass.
- **G2:** the v1.7.0 differential harness on `render`, `validate` and the Python
  package shows zero diffs, except an approved list (initially: #787).
- **G3:** the restored 1.7.1 bead tests pass, including `single_pass_render.rs`
  and `fuzz_171_*`.

## Stop conditions

Stop, write one paragraph, and wait for Rand when:

- a conflict cannot be resolved by the one-decision rule;
- three or more failures share a cause: name the decision causing them;
  do not fix them one by one;
- a fix round opens as many findings as it closes;
- a fix would change a test that encodes a listed requirement;
- steps 1–3 are not done in 2 hours.

## Out of scope

- New fuzz campaigns. The `fuzz_171_*` regressions are the fuzz coverage.
- Platform delivery and the Python ABI floor (#820, #611, #819, FR-25): a
  separate phase.
- Anchors, FR-24 rewrite, and Phase U sprint beads (#848, #849): parked until
  this lands.
- Any design discussion. Open questions go under Stop conditions.
