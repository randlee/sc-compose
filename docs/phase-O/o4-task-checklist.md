# O.4 Template Migration — Local Closure Checklist

This checklist records the implementation audit for
`docs/phase-O/sprint-o-4-template-migration.md`. It is local sprint evidence;
O.5 owns cross-repository inventory, release-candidate fuzzing, and release
readiness claims.

## Initial audit

- [x] O4-001 — Migrate the six named JSON templates to explicit
  `json_escape_mode: auto`, removing literal quotes only for complete scalar
  JSON value slots and preserving structured/raw contracts.
- [x] O4-002 — Classify every interpolation in all six templates as scalar,
  structured value, loop element, conditional fragment, raw JSON, or ambiguous
  macro/include, with a migration or documented legacy decision.
- [x] O4-003 — Add semantic fixtures for all six templates with representative
  and hostile values: quotes, backslashes, Unicode, newlines, empty strings,
  arrays, objects, nulls, and permitted control characters.
- [x] O4-004 — Assert parsed JSON semantic values and injection resistance,
  rather than relying on output snapshots alone.
- [x] O4-005 — Add explicit legacy compatibility fixtures proving the old source
  shape remains valid JSON and emits the documented deprecation warning.
- [x] O4-006 — Cover `template-init` generated JSON round-trips and ensure the
  six migrated templates are validated with `validate --lint`.
- [x] O4-007 — Document the migration matrix, fixture contexts, expected
  values, legacy exceptions, and O.5 evidence handoff.
- [x] O4-008 — Update `docs/requirements.md`, `docs/migration/json-escape-mode.md`,
  `docs/migration-notes.md`, and `CHANGELOG.md` without making cross-repository
  release claims.
- [x] O4-009 — Run the required workspace, formatting, clippy, template-lint,
  and diff gates; record any environment-only limitation without adding lint
  suppressions.
- [x] O4-010 — Re-read the O.4 plan after implementation and perform a second
  closure audit; add and resolve any newly discovered gaps.

## Closure review

Second audit findings were resolved:

- [x] O4-011 — The initial fixture needed an isolated clean-corpus
  `template-contracts` gate because the repository also contains intentional
  negative lint fixtures owned by O.3; the clean six-template gate now passes.
- [x] O4-012 — The hostile corpus needed explicit empty-string and null-branch
  cases; the fixture now covers those values and asserts their JSON types.
- [x] O4-013 — Conditional placeholders in `phase`, `sprint`, `sprint_doc`,
  and optional `worktree_path` were found during the re-audit and migrated to
  bare auto-mode slots.
- [x] O4-014 — The repository-wide `template-contracts` command reports the
  pre-existing O.3 negative fixtures by design (exit 2); the isolated O.4
  six-template corpus passes the same target with six templates. No O.4
  migration template is among those findings.

Verification recorded before final `just test`:

- `cargo test --workspace` — PASS.
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS.
- `cargo fmt --all --check` — PASS.
- `git diff --check` — PASS.
- `validate --lint` for each six-template corpus member — PASS.
- isolated `sc-compose lint --target template-contracts` over six files — PASS.
- repository-wide `sc-compose lint` / `just lint target=template-contracts` —
  expected exit 2 from existing O.3 negative fixtures; no O.4 findings.

- [x] All initial-audit items are resolved with tests or documentation evidence.
- [x] No O.5 cross-repository or release-readiness claim is made by O.4.
- [x] Final `just test` passes and the worktree is clean except for committed
  O.4 deliverables.
