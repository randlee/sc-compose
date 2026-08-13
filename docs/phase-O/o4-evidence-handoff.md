# O.4 Evidence Handoff to O.5

This is the repository-local semantic fixture handoff. It does not make a
cross-repository release-readiness claim; O.5 must add the external corpus and
release fuzz evidence.

## Corpus

The six exact corpus members are:

1. `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`
2. `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2`
3. `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2`
4. `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2`
5. `.claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2`
6. `.claude/skills/codex-orchestration/req-qa-assignment.json.j2`

The executable evidence is
`crates/sc-compose/tests/json_cli/o4_templates.rs`:

- `six_known_templates_render_semantically_with_hostile_values` renders all
  six with quoted strings, backslashes, Unicode, newlines, arrays, nested raw
  objects, booleans, null-capable branches, and hostile key-injection text.
  It parses each complete body and asserts that no `injected` key appears.
- `six_known_templates_validate_with_lint_in_auto_mode` runs
  `validate --lint --json` against each corpus member and requires a successful
  auto-mode contract with no `ERR_JSON_MODE_CONTRACT` diagnostic.
- `template_contract_lint_passes_for_a_clean_o4_corpus` copies exactly the six
  files into an isolated root and requires the repository-level
  `template-contracts` target to pass with six scanned templates.

## Expected semantic values

- `rust-best-practices-assignment`: `review_mode` and path values remain
  strings; `review_targets` and `practice_ids` remain arrays of strings;
  `carry_forward_findings` parses as an array of objects.
- `rust-qa-assignment`: path and baseline fields remain strings; `run_checks`
  remains boolean-valued; command and file arrays preserve element boundaries.
- `rust-service-hardening-assignment`: `topics` and extra service indicators
  remain arrays; the static dependency list remains unchanged.
- `arch-qa-assignment`: `scope.phase` and `scope.sprint` remain strings or
  `null`; references and review targets remain arrays of strings.
- `flaky-test-qa-assignment`: optional scope values remain strings or `null`.
- `req-qa-assignment`: document, branch, commit, and worktree values remain
  strings or `null`, and both document arrays preserve Unicode paths.

## Compatibility exception

`legacy_compatibility_fixture_is_valid_and_warns_once` keeps the old quoted
`"{{ value }}"` source shape under explicit `json_escape_mode: legacy`. It
requires valid parsed JSON, preserves the hostile string as one value, and
requires exactly one `WARN_JSON_LEGACY_ESCAPE_MODE` diagnostic. No O.4 corpus
member uses legacy mode.

## O.5 handoff boundary

O.5 should consume this corpus and expected-value contract, then add external
repository roots, release-candidate fuzz probes, reproducibility counts, and
the final release decision. O.4 does not certify those external roots.
