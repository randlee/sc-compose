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
