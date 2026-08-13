# O.5 release corpus evidence

Campaign date: 2026-08-13
Campaign ID: `o5-20260813-0001`
Candidate parent: `0b24aa7e72eaab911d66520a762449e8597a7eb5`
Tool: `sc-compose` from this worktree, with parser-backed JSON checks
Scope: read-only scan of the roots pinned in
[`../release-corpus-roots.txt`](../release-corpus-roots.txt)

## Root verification

All seven roots passed `git -C ROOT rev-parse PINNED_COMMIT`:

| Repository | Pinned commit | Files enumerated | Scan result |
| --- | --- | ---: | --- |
| sc-compose O.4 worktree | `0b24aa7e72eaab911d66520a762449e8597a7eb5` | 11 | actionable expected fixtures only |
| atm-core | `aebfc5b18a7a1c086726fb2ecd2ec47a074891b5` | 7 | 6 owned legacy findings; 1 clean |
| cpo | `478de8f00151cc911160fb3ea13253af8bde6d03` | 7 | 7 owned legacy findings |
| raptor | `39ccb49c91aeb090f1b67ad9b83906a55f7bb46b` | 3 | 3 owned legacy findings |
| sc-lint | `455f164d29f29af7e8604387c88205aefe16ce75` | 6 | 6 owned legacy findings |
| synaptic-canvas | `862fa2bcf5718de58948cdfb2d78420e88c57d81` | 3 | 3 owned legacy findings |
| roslyn-lint | `27b4114b31b1bf8a3b435980a2e530f030141ade` | 3 | 3 owned legacy findings |
| **Total** | 7 pinned roots | **40** | conditional release gate |

The inventory used `rg --files --hidden -g '!.git'` and included
`.json.j2`, `.json.jinja`, and `.json.jinja2` paths. The initial visible-only
probe was rejected during the closure review because it omitted `.claude`;
the counts above are the corrected source-of-truth counts.

## Scope reconciliation

The comp2 O5 cross-repository inventory named five downstream candidates:
cpo, raptor, roslyn-lint, sc-lint, and synaptic-canvas. All five are now
present in the roots file and this rescan; roslyn-lint is pinned at
`27b4114b31b1bf8a3b435980a2e530f030141ade` and contributes three owned
`ERR_JSON_MODE_CONTRACT` findings. No comp2 candidate was excluded. The
sc-compose and atm-core roots remain required sprint roots in addition to
those five downstream candidates.

## Path-level inventory

### sc-compose O.4 root

| Path | Mode/source shape | Finding | Owner / disposition |
| --- | --- | --- | --- |
| `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2` | explicit `auto`, bare complete values | O5-SC-001 | clean; O.4 fixture |
| `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2` | explicit `auto`, bare complete values | O5-SC-002 | clean; O.4 fixture |
| `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2` | explicit `auto`, bare complete values | O5-SC-003 | clean; O.4 fixture |
| `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2` | explicit `auto`, bare complete values | O5-SC-004 | clean; O.4 fixture |
| `.claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2` | explicit `auto`, bare complete values | O5-SC-005 | clean; O.4 fixture |
| `.claude/skills/codex-orchestration/req-qa-assignment.json.j2` | explicit `auto`, bare complete values | O5-SC-006 | clean; O.4 fixture |
| `crates/sc-composer/tests/fixtures/reverse-extract/json-atm-payload.json.j2` | explicit `legacy`, quoted strings | O5-SC-007 | intentional compatibility fixture; warning expected |
| `crates/sc-composer/tests/fixtures/reverse-extract/json-malformed.json.j2` | explicit `legacy`, malformed test fixture | O5-SC-008 | intentional negative fixture; warning/failure expected |
| `tests/fixtures/sc-lint/template-contracts/findings/auto.json.j2` | explicit `auto`, quoted scalar | O5-SC-009 | intentional lint-negative fixture; must remain red |
| `tests/fixtures/sc-lint/template-contracts/findings/legacy.json.j2` | explicit `legacy`, quoted scalar | O5-SC-010 | intentional lint-warning fixture |
| `tests/fixtures/sc-lint/template-contracts/findings/valid-auto.json.j2` | explicit `auto`, bare scalar | O5-SC-011 | clean lint-positive fixture |

The six O.4 production templates were rendered with hostile quotes,
backslashes, Unicode, newlines, control-safe characters, arrays, objects,
nulls, and empty values. Each successful body was parsed as one complete JSON
document and checked for injection. The two reverse-extract files and three
lint fixtures are test corpus members, not production release blockers.

### atm-core root — external, read-only

| Path | Mode/source shape | Finding | Owner / handoff |
| --- | --- | --- | --- |
| `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2` | no mode, manually quoted placeholders | O5-ATM-001 | atm-core owner: migrate to `auto` bare values or pin explicit `legacy` |
| `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2` | no mode, manually quoted placeholders | O5-ATM-002 | same handoff; preserve arrays/booleans as typed values |
| `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2` | no mode, manually quoted placeholders | O5-ATM-003 | same handoff |
| `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2` | no mode, manually quoted placeholders | O5-ATM-004 | same handoff; review conditional null fields |
| `.claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2` | no mode, manually quoted placeholders | O5-ATM-005 | same handoff |
| `.claude/skills/codex-orchestration/req-qa-assignment.json.j2` | no mode, manually quoted placeholders | O5-ATM-006 | same handoff |
| `.claude/skills/codex-orchestration/ruthless-boundary-qa-assignment.json.j2` | no quoted interpolation found in scan | O5-ATM-007 | inventory clean |

`atm-core` does not contain `.sc/sc-lint/targets/template-contracts.toml` at
the pinned commit. That is reported as `O5-ATM-CONFIG-001`, owned by
atm-core/team-lead, and is not treated as a pass. The six quoted templates
are concrete path-level migration work, not speculative findings. No files
were changed in atm-core.

### Additional supplied roots — read-only

The following roots were added after the initial inventory request. They are
included because their commits and paths were available locally, not because
the campaign assumes an unverified high-traffic repository count.

| Repository | Path-level result | Owner / action |
| --- | --- | --- |
| cpo | `cpo-core/skills/critical-path-orchestration/templates/assignments/arch-qa-assignment.json.j2`; `cpo-core/skills/critical-path-orchestration/templates/assignments/flaky-test-qa-assignment.json.j2`; `cpo-core/skills/critical-path-orchestration/templates/assignments/req-qa-assignment.json.j2`; `cpo-core/skills/critical-path-orchestration/templates/assignments/ruthless-boundary-qa-assignment.json.j2`; `cpo-rust/assets/templates/rust-best-practices-assignment.json.j2`; `cpo-rust/assets/templates/rust-qa-assignment.json.j2`; `cpo-rust/assets/templates/rust-service-hardening-assignment.json.j2` — 7/7 contain manually quoted placeholders and no explicit `json_escape_mode` | cpo owner: migrate to explicit `auto`/bare values or explicitly pin `legacy`; rerun on merged owner commit |
| raptor | `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`; `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2`; `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2` — 3/3 contain manually quoted placeholders and no explicit mode | raptor owner: same migration handoff |
| sc-lint | `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`; `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2`; `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2`; `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2`; `.claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2`; `.claude/skills/codex-orchestration/req-qa-assignment.json.j2` — 6/6 contain manually quoted placeholders and no explicit mode | sc-lint owner: migrate shared templates or coordinate source-package update |
| synaptic-canvas | `packages/sc-rust/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`; `packages/sc-rust/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2`; `packages/sc-rust/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2` — 3/3 contain manually quoted placeholders and no explicit mode | synaptic-canvas owner: migrate shared templates or consume the canonical package |
| roslyn-lint | `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2`; `.claude/skills/codex-orchestration/rlint-qa-assignment.json.j2`; `.claude/skills/codex-orchestration/req-qa-assignment.json.j2` — 3/3 contain manually quoted placeholders and no explicit mode | roslyn-lint owner: migrate to explicit `auto`/bare values or explicitly pin `legacy`; rerun O.5 |

These five roots have no
sc-compose `.sc/sc-lint/targets/template-contracts.toml` contract descriptor,
so the static source-shape result is the actionable evidence rather than a
claim that their local lint command passed.

## Campaign oracle and evidence

The campaign used four bounded worker tasks and the existing O.2/O.3/O.4
contracts; it did not introduce a second parser or scanner:

| Worker | Iterations | Pass | Result | Evidence |
| --- | ---: | ---: | --- | --- |
| six-template parser oracle | 6 | 6/6 | PASS | `cargo test -p sc-compose --test json_cli o4_templates -- --nocapture` |
| mode compatibility and 1.4 regression | 6 | 6/6 | PASS | auto quoted fixture fails closed; legacy fixture parses and warns once |
| cross-repository inventory | 40 | 40/40 | PASS* | 11 + 7 + 7 + 3 + 6 + 3 + 3 paths pinned and counted; 28 external findings owned |
| hostile/nested/boundary corpus | 12 | 12/12 | PASS | O.2/O.3/O.4 JSON CLI and lint tests |
| **Total** | **64** | **64/64** | **PASS*** | `*` means no unowned failure; release remains conditional on external owner handoff |

The original regression is permanently represented by
`tests/fixtures/sc-lint/template-contracts/findings/auto.json.j2`:
`{"value": "{{ value }}"}` under auto mode is rejected before output. The
explicit legacy fixture renders one safe JSON string and emits exactly one
`WARN_JSON_LEGACY_ESCAPE_MODE`. A successful JSON body is never marked PASS
without `serde_json` parsing it as a complete document.

## Workspace gate results

The final workspace gates were run after the campaign artifacts were
materialized. The results below distinguish green checks from intentionally
failing negative fixtures and from a real integration gap in the full lint
wrapper:

| Gate | Result | Evidence / disposition |
| --- | --- | --- |
| `just test` | PASS | report-render-many, report rendering, and report finalization passed |
| `cargo test --workspace` | PASS | all workspace suites passed |
| `cargo fmt --all --check` | PASS | no formatting drift |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS | no warnings denied |
| `just lint target=fast` | PASS | all five fast-profile steps passed |
| `just lint target=sc-boundary` | PASS | direct sc-compose target used `sc-lint-boundary` 0.4.0 and scanned 3 crates |
| `just lint target=template-contracts` | EXPECTED FAIL | negative `auto.json.j2` is required to emit `ERR_JSON_MODE_CONTRACT`; legacy/reverse-extract fixtures are required warning cases |
| `just lint` | BLOCKED | the full wrapper calls the repository's `sc-compose lint` recipes directly, which shell out to `sc-lint` and the `sc-lint-boundary`, `sc-lint-portability`, and `sc-lint-runtime` sibling binaries. Those release binaries are absent from a bare local worktree and are provisioned by CI's `.github/actions/setup-sc-lint/action.yml`; CI's `just lint-ci-consumer` is green and is the authoritative gate. This is `O5-SC-LINT-BOOTSTRAP-001`, and no suppression was added |
| report validation | PASS | `html-validate` passed the top-level report; `xmllint --noout` passed all four XHTML panels; `jq empty` passed the sidecar |
| `git diff --check` | PASS | no whitespace errors |

Therefore O.5 has complete parser/corpus evidence and a conditional release
recommendation, but it does not claim that a bare local worktree can run every
repository-level lint wrapper. The local BLOCKED result is a tooling-
provisioning gap, not a missing `.just/` script or a repository regression:
the pinned sc-lint release binaries are supplied by CI's setup action. CI's
green `just lint-ci-consumer` run is the authoritative provisioned lint gate.

## Release recommendation

**CONDITIONAL — do not claim an unconditional 1.4.1 release yet.** The
sc-compose O.4 corpus and local parser gate are green, but the pinned external
roots contain 28 unannotated quoted assignment templates across atm-core, cpo,
raptor, sc-lint, synaptic-canvas, and roslyn-lint. Each owner must either
migrate those templates to explicit `auto` with bare placeholders or explicitly
pin `legacy`, then rerun this corpus against the merged consumer commit.

Legacy mode may remain during the 1.4.1 deprecation window. Removal is allowed
only after every pinned consumer root is clean, the external ownership list is
empty, and a release-candidate campaign demonstrates parser-backed success
for scalar, array, object, null, loop, include, conditional, Unicode, and
hostile-string cases. The full campaign package is
`site/reports/20260813-1-fuzz-report.html` with its JSON sidecar and four XHTML
worker panels.
