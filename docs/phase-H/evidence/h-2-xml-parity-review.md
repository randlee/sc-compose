# H.2 XML/raw-text parity review

- Reviewer: team-lead (independent review of the promoted H.2 implementation
  diff)
- Review date: 2026-08-02
- Baseline: `origin/integrate/phase-h` at `33d6504`
- Reviewed implementation: `0644187..d85b9b9`
- Diff inspected: `crates/sc-composer/src/extract/raw_text.rs`, the XML
  delegation in `crates/sc-composer/src/extract/xml.rs`, the frozen fixture at
  `crates/sc-composer/tests/fixtures/reverse-extract/xml-regression-baseline.json`,
  and `fixture_xml_reports_match_frozen_h2_baseline` in
  `crates/sc-composer/tests/extract_integration.rs`.
- Check performed: the XML reports produced after the shared matcher and
  format-specific report refactors were compared with the frozen fixture;
  `cargo test -p sc-composer --test extract_integration
  fixture_xml_reports_match_frozen_h2_baseline` passed.
- Decision: **ACCEPT**. The shared raw-text extraction seam preserves the
  frozen XML report values, occurrence paths/sources, diagnostics, and
  confidence. JSON-specific policy work is therefore allowed to proceed from
  the parity checkpoint.

The Phase-H amendments are still not merged to `origin/develop`: the diff from
`origin/develop` to `origin/integrate/phase-h` includes the Phase-H plan,
H.1-H.6 sprint documents, and ADR-0012. `origin/develop` does contain the
earlier H.2 planning file, so this confirmation refers specifically to the
post-rescope H.1/H.2 amendments rather than the existence of any Phase-H file.
