---
id: H.8
title: Phase-Ending Review Remediation
status: in_progress
branch: fix/phase-h-ending-review
worktree: ../sc-compose-worktrees/fix/phase-h-ending-review
target: integrate/phase-h
---

# Sprint H.8 — Phase-Ending Review Remediation

## Goal

- Close the Blocking and Important findings from the Phase H phase-ending
  review re-run (quality-mgr verdict FAIL, deliverable completion 4/11,
  `phase-h-production-readiness-verify-2`), so `integrate/phase-h` can pass a
  clean phase-ending re-verification and become mergeable to `develop`.
- Do not reopen or touch anything already confirmed fixed by Sprint H.7's own
  QA-1 through QA-4 (JSON/YAML incremental depth guards, XML depth+Drop,
  the original XML alias/anchor named cases).

## Hard Dependencies

- H.1 through H.7 are complete and merged on `integrate/phase-h` at `b69e564`.
- This sprint targets `integrate/phase-h` directly (not a fresh
  `sprint/h-*` branch off it), since it remediates phase-level findings
  discovered after H.7 already merged.

## Exact Targets

- `crates/sc-composer/src/extract/yaml.rs`
- `crates/sc-composer/src/extract/json.rs`
- `crates/sc-composer/src/extract/toml.rs`
- `crates/sc-composer/src/extract/xml.rs`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-compose/tests/cli/extract.rs`
- `docs/error-code-registry.md`
- `docs/phase-H/phase-H-plan.md`
- `docs/phase-H/sprint-h-7-alias-input-limit-hardening.md`
- `docs/phase-H/sprint-h-1-reverse-extraction-extension-contract.md`
- `docs/architecture.md`
- `docs/requirements.md`
- `docs/project-plan.md`
- `docs/phase-H/evidence/h-6-cross-format-campaign.json`
- `site/reports/20260803-1-fuzz-report.html` (or successor report path)

## Required Fixes

1. **finding-1-yaml-tag-bypass (Blocking)** — `yaml.rs`'s `is_tag`/
   `contains_yaml_features` does not flag a bare non-specific YAML tag (`!`
   alone, e.g. `value: !` or `! "x"`). rust-qa-agent confirmed the document is
   currently still rejected end-to-end, but only incidentally, because the
   hand-written `Visitor` has no `visit_enum` implementation for serde_yaml's
   tagged-node representation — the wrong diagnostic code is emitted
   (`ERR_EXTRACT_YAML_MALFORMED` instead of `ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED`),
   and there is zero regression-test coverage, so this fail-safe is
   version-fragile and could silently reopen as a real data-leak bypass on any
   serde_yaml upgrade. Widen `is_tag` to explicitly flag bare `!` at a feature
   boundary so the scanner itself catches this case (do not rely on the
   incidental Visitor gap), and emit `ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED`.
   Add a regression test asserting the correct diagnostic code for both
   `value: !` and `! "x"` forms.
2. **finding-2-toml-post-parse-depth (Important)** — `toml.rs` has no
   pre-parse depth guard, unlike `json.rs`/`yaml.rs`'s
   `validate_parse_depth`. Add an equivalent pre-parse depth guard to
   `toml.rs` so deep-input failures return `ERR_EXTRACT_INPUT_LIMIT` instead
   of `ERR_EXTRACT_TOML_MALFORMED`, with a regression test past TOML's
   internal recursion threshold. (Optional but recommended, per
   RBP-F004: factor the now-3-format-duplicated pre-parse depth-guard logic
   into one shared helper used by json.rs/yaml.rs/toml.rs, rather than a
   fourth independent copy.)
3. **finding-3-pathless-diagnostics (Important)** — raw-text
   `StaticMismatch` errors in `json.rs`, `yaml.rs`, `toml.rs`, and `xml.rs`
   lose structural path context (bare `$`, or in XML's case
   `xml.rs:797-801`, no path argument at all). Thread the current
   path/provenance through mismatch mapping in all four adapters so nested
   failures report a real structural location (e.g. `$.profile.name`). Add
   nested-failure regression tests for each format.
4. **finding-4-registry-drift (Important)** — `docs/error-code-registry.md`
   lists `ERR_EXTRACT_SECURITY_POLICY`, which is not implemented anywhere in
   `crates/sc-composer/src/diagnostics.rs` (YAML actually emits the stable
   `ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED`). This drift is also documented in
   `docs/phase-H/sprint-h-1-reverse-extraction-extension-contract.md:200-201`.
   Correct the registry and the H.1 contract doc to reflect the real,
   implemented diagnostic contract (do not implement a new code unless a
   genuine generic security-policy diagnostic is intentionally wanted —
   default to correcting the docs to match implementation). Separately, add
   the currently-missing `WARN_CONFIG_SINGLE_PASS_ALL_FALLBACK` row to the
   registry (`diagnostics.rs:63,175` already implements it; the registry does
   not document it) — the drift is bidirectional.
5. **finding-5-stale-phase-docs (Important, elevated to Blocking by req-qa)**
   — `docs/architecture.md` and `docs/requirements.md` still describe
   H.2–H.6 as the closure set; `docs/project-plan.md`'s phase summary omits
   H.7; `docs/phase-H/phase-H-plan.md`'s front matter still says
   `status: planned`; `docs/phase-H/sprint-h-7-alias-input-limit-hardening.md`'s
   front matter still says `status: in_progress` despite H.7 already being
   merged (PR #209, commit b69e564). Update all of the above to accurately
   reflect H.1–H.7 (and this H.8 remediation sprint) as the current phase
   scope and status, per requirements.md's own self-declared authority
   language.
6. **finding-6-fuzz-campaign-overclaim (Important, docs-only)** — the H.6
   evidence file's `execution_mode` field already honestly discloses 4
   bounded local workers / 36 cases with the Agent Runner unavailable, but
   `docs/requirements.md`'s "36/36 expected outcomes" framing does not carry
   that caveat forward. Propagate the bounded-evidence caveat into
   `requirements.md` (and any other summary doc making the stronger claim)
   so the scope of the H.6 evidence is accurately represented everywhere it
   is cited. Running a genuine distributed adversarial-fuzz campaign is out
   of scope for this remediation sprint (may be proposed as a future sprint
   if desired) — this fix is documentation-caveat propagation only.
7. **finding-7-absolute-paths-in-evidence (Minor)** —
   `docs/phase-H/evidence/h-6-cross-format-campaign.json:5` and its rendered
   HTML report embed an absolute local worktree path
   (`/Users/randlee/...`). Replace with a repository-relative path or
   portable metadata.

## Explicitly Out of Scope

- Do not re-run or re-litigate H.7's own QA-1 through QA-4 verdicts.
- Do not run a genuine distributed adversarial-fuzz campaign as part of this
  sprint (finding 6 is a docs-caveat fix only here).
- RBP-F004's shared pre-parse depth-guard abstraction is optional/recommended,
  not a hard blocker, if item 2's TOML fix is delivered correctly without it.

## Acceptance Criteria

- `value: !` and `! "x"` YAML input is rejected by the scanner itself with
  `ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED`, with a passing regression test.
- TOML over-depth input returns `ERR_EXTRACT_INPUT_LIMIT` via a genuine
  pre-parse guard, with a passing regression test.
- Nested raw-text mismatch diagnostics report a real structural path for
  JSON, YAML, TOML, and XML, with passing regression tests for each.
- `docs/error-code-registry.md` and
  `docs/phase-H/sprint-h-1-reverse-extraction-extension-contract.md` no
  longer document `ERR_EXTRACT_SECURITY_POLICY` as implemented, and the
  registry includes `WARN_CONFIG_SINGLE_PASS_ALL_FALLBACK`.
- `docs/architecture.md`, `docs/requirements.md`, `docs/project-plan.md`,
  `docs/phase-H/phase-H-plan.md`, and this sprint's own front matter
  accurately reflect H.1–H.8 status.
- `docs/requirements.md`'s fuzz-campaign framing carries the bounded-evidence
  caveat.
- No absolute local paths remain in committed evidence/report artifacts.
- Full validation gates green: `cargo fmt --all --check`;
  `cargo clippy --all-targets --all-features -- -D warnings`;
  `cargo test --workspace`; `cargo test -p sc-compose --test repo_boundaries`;
  `cargo test -p sc-compose-py`; `maturin develop --release` then
  `python3 -m pytest bindings/python/tests/test_smoke.py`; `git diff --check`.
