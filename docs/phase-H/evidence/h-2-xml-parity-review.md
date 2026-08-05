# H.2 XML/raw-text parity validation record

- Independent reviewer: **team-lead**, review performed 2026-08-02 by
  inspecting the diff directly and independently re-running the test suite
  (not by re-running comp's reported commands or accepting comp's summary).
- Validation date: 2026-08-02
- Baseline: `origin/integrate/phase-h` at `33d6504`
- Implementation validated: `0644187..d85b9b9` (raw_text.rs extraction and xml.rs
  delegation), confirmed unchanged by the subsequent QA-fix commits
  (`df267ea`, `9377c79`, `271fd92`), which touched error-chaining, scope
  propagation, path/source types, and this evidence file but not the
  parity-relevant matching logic itself.
- Diff inspected: `crates/sc-composer/src/extract/raw_text.rs` (new
  format-neutral matcher — delimiter scanning, longest-match-first
  replacement, multi-pass brace counting, ambiguity detection) against the
  prior inline `xml.rs` implementation it replaces (`parse_value_segments`,
  `match_value`, `TemplateSegment`). The extracted logic is a direct,
  semantics-preserving lift: same double-brace parsing rules, same
  static/variable segment handling, same ambiguity conditions, same error
  messages. `xml.rs`'s `match_value` now delegates to
  `raw_text::match_raw_text` and converts the result back into
  `Capture`/`ExtractError` without altering XML structural traversal,
  path/source provenance (`XmlPathSegment`, `ExtractionSource`), or
  format-specific diagnostics.
- Independent check performed: read the full diff of both files; separately
  ran `cargo test -p sc-composer extract::` (22 passed) and
  `cargo test -p sc-composer --test extract_integration` (22 passed,
  including `fixture_xml_reports_match_frozen_h2_baseline`) directly in the
  worktree at commit `271fd92`, without relying on comp's reported command
  output.
- Decision: **ACCEPT**. The shared raw-text extraction seam preserves the
  frozen XML report values, occurrence paths/sources, diagnostics, and
  confidence. JSON-specific policy work was correctly allowed to proceed from
  this parity checkpoint. H2-D5's independent-review requirement is now
  satisfied.

The Phase-H amendments are still not merged to `origin/develop`: the diff from
`origin/develop` to `origin/integrate/phase-h` includes the Phase-H plan,
H.1-H.6 sprint documents, and ADR-0012. `origin/develop` does contain the
earlier H.2 planning file, so this confirmation refers specifically to the
post-rescope H.1/H.2 amendments rather than the existence of any Phase-H file.
