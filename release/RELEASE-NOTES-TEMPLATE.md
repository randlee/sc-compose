# Release Notes — sc-compose v<VERSION>

<!--
  Fill out this template before creating the GitHub release.
  Delete sections that don't apply.
  Replace <VERSION> with the actual version number (e.g., 1.3.0).
-->

## Summary

<!-- 3-5 sentences describing the release at a high level. -->

-

## Included Packages

| Package | Version | Channel |
|---------|---------|---------|
| `sc-sha` | <VERSION> | crates.io |
| `sc-composer` | <VERSION> | crates.io |
| `sc-compose` | <VERSION> | crates.io · Homebrew · Winget · Scoop · GitHub Releases |
| `sc-sha` (Python) | <VERSION> | PyPI |
| `sc-compose` (Python) | <VERSION> | PyPI |

## What's New

### Added

<!-- New features, commands, APIs, platforms, or capabilities. -->

-

### Changed

<!-- Behavioral changes, API renames, default changes. -->

-

### Fixed

<!-- Bug fixes with issue references where applicable. -->

-

### Removed

<!-- Deprecated features that were removed. -->

-

## Compatibility Notes

<!-- Breaking changes, migration steps, and cutover notes. -->

- **MSRV:** Rust <RUST_VERSION>
- **Rust edition:** <EDITION>
- **Python:** <PYTHON_VERSION>+

### Migration from v<PREVIOUS_VERSION>

<!-- Steps required to upgrade. Delete if no migration needed. -->

-

## Known Issues

<!-- Any known limitations or issues to be addressed in a future release. -->

-

## Full Changelog

See [CHANGELOG.md](https://github.com/randlee/sc-compose/blob/develop/CHANGELOG.md)
for the complete list of changes.

## Verification

```bash
# Verify crate availability
cargo add sc-composer@<VERSION> --dry-run
cargo install sc-compose@<VERSION> --dry-run

# Verify Python package
pip install sc-compose==<VERSION> --dry-run

# Verify binary
sc-compose --version
sc-compose observability-health
```
