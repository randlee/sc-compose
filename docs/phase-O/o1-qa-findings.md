# O.1 QA Findings Task List

Source: quality-mgr QA gate for PR #423.

## Findings

| ID | Severity | Finding | Status |
| --- | --- | --- | --- |
| ARCH-001 | Blocking | JSON escape-mode precedence (`CLI > root frontmatter > auto`) is implemented independently in `composer.rs`, `validation/diagnostics.rs`, and `compose_render.rs`; the custom-delimiter path also lacks precedence coverage. | Resolved |

## Required closure evidence

- [x] One canonical resolver function owns precedence.
- [x] Composer, validation, and custom-delimiter rendering delegate to it.
- [x] Custom-delimiter precedence tests cover CLI override, frontmatter fallback,
      and default `auto` behavior.
- [x] Existing O.1 tests remain green.
- [x] Final critical review confirms no duplicated resolver logic remains and
      every finding is closed.

## Closure review

ARCH-001 is closed. `resolve_json_escape_mode` in
`crates/sc-composer/src/renderer.rs` is the only precedence implementation;
the composer, validation diagnostics, and custom-delimiter CLI render path all
delegate to it. The renderer unit test covers all three precedence branches,
and `render_json_custom_delimiters_respect_mode_precedence` covers those same
branches through the previously untested custom-delimiter path.

Verification completed:

- `cargo test --workspace` — pass.
- `cargo fmt --all --check` — pass.
- `cargo clippy --all-targets --all-features -- -D warnings` — pass.
- `git diff --check` — pass.
- Resolver audit found no remaining `effective_json_escape_mode` or inline
  `Option<JsonEscapeMode>` precedence implementation.

## QA423 round 2 findings task list

Source: quality-mgr review of PR #423 at the pre-ARCH-001 commit.

| ID | Severity | Finding | Status |
| --- | --- | --- | --- |
| RBP-F001 | Important | Quoted JSON placeholder diagnostics only recognized bare identifiers. | Resolved |
| RBP-F002 | Important | JSON string serialization used a silent empty-string fallback for an infallible operation. | Resolved |
| ATM-QA-002 | Important | `JsonEscapeMode` added `Default`/`#[default]` and lower-case serde behavior beyond the documented signature. | Resolved with rationale |
| ATM-QA-003 | Important | Template-init JSON round-trip lacked a true CLI integration test. | Resolved |
| ATM-QA-004 | Important | `ERR_JSON_ESCAPE_MODE_NON_JSON` had no coverage. | Resolved |
| ARCH-002 | Minor | `assemble_output` was duplicated between the composer library and CLI custom-render path. | Resolved |
| ATM-QA-005 | Minor | No test asserted one legacy deprecation warning with multiple quoted placeholders. | Resolved |
| ATM-QA-007 | Minor | O.1 plan cited a stale ADR filename. | Resolved |

ATM-QA-002 rationale: `Default` and `#[default]` were removed. Lowercase serde
renaming remains intentional because `json_escape_mode: legacy|auto` is the
documented frontmatter and JSON wire contract; removing it would reject the
required lowercase values and serialize the public mode as `Legacy`/`Auto`.
This rationale was sent to team-lead and approved before proceeding.

Finding-specific closure evidence:

- RBP-F001: `quoted_json_placeholder_expressions` now retains dotted and
  filtered expressions for migration warnings; direct variable expressions
  still receive the existing non-string validation when resolvable.
- RBP-F002: `serde_json::to_string` now uses an explicit `expect` for the
  infallible Rust-string serialization path.
- ATM-QA-003: `template_init_json_then_cli_render_is_a_semantic_round_trip`
  invokes both CLI commands and compares parsed JSON values.
- ATM-QA-004: `validate_json_rejects_json_escape_mode_on_non_json_templates`
  covers `ERR_JSON_ESCAPE_MODE_NON_JSON`.
- ARCH-002: `assemble_output` is public in `sc-composer` and the CLI delegates
  to that single implementation.
- ATM-QA-005: the multiple-placeholder validation test asserts exactly one
  `WARN_JSON_LEGACY_ESCAPE_MODE` diagnostic.
- ATM-QA-007: the O.1 plan now cites the actual ADR filename.

Round-2 critical review: all eight listed findings are closed or explicitly
justified; `rg` confirms one `assemble_output` implementation and one JSON
mode resolver, with no silent JSON serialization fallback remaining.

Round-2 verification:

- `cargo test --workspace` — pass.
- `cargo fmt --all --check` — pass.
- `cargo clippy --all-targets --all-features -- -D warnings` — pass.
- `git diff --check` — pass.

The pre-existing ARCH-001 closure above was re-confirmed before these fixes:
`resolve_json_escape_mode` is public, exported, used by all three former call
sites, and covered through custom delimiters.
