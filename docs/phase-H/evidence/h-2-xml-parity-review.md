# H.2 XML/raw-text parity validation record

- Independent reviewer: **none**. No independent review was performed for
  this record; attributing one would be inaccurate.
- Validation date: 2026-08-02
- Baseline: `origin/integrate/phase-h` at `33d6504`
- Implementation validated: `0644187..d85b9b9`
- Diff inspected by the implementer: `crates/sc-composer/src/extract/raw_text.rs`, the XML
  delegation in `crates/sc-composer/src/extract/xml.rs`, the frozen fixture at
  `crates/sc-composer/tests/fixtures/reverse-extract/xml-regression-baseline.json`,
  and `fixture_xml_reports_match_frozen_h2_baseline` in
  `crates/sc-composer/tests/extract_integration.rs`.
- Developer check performed: the XML reports produced after the shared matcher and
  format-specific report refactors were compared with the frozen fixture;
  `cargo test -p sc-composer --test extract_integration
  fixture_xml_reports_match_frozen_h2_baseline` passed.
- Decision: **INDEPENDENT REVIEW OUTSTANDING**. The developer validation shows
  that the shared raw-text extraction seam preserves the frozen XML report
  values, occurrence paths/sources, diagnostics, and confidence, but it is not
  an independent green review and does not satisfy the H2-D5 review gate.

The Phase-H amendments are still not merged to `origin/develop`: the diff from
`origin/develop` to `origin/integrate/phase-h` includes the Phase-H plan,
H.1-H.6 sprint documents, and ADR-0012. `origin/develop` does contain the
earlier H.2 planning file, so this confirmation refers specifically to the
post-rescope H.1/H.2 amendments rather than the existence of any Phase-H file.
