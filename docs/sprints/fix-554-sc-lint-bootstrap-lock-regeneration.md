---
id: FIX-554
sprint: Follow-on CI fixture verification
status: complete
---

# FIX-554: Regenerate the `sc-lint` bootstrap lockfile in test coverage

## Context

The checked-in `sc-lint` bootstrap fixture carries Cargo-generated
`[[patch.unused]]` metadata for the workspace-patched `sc-composer` and
`sc-sha` crates. A manually edited fixture lockfile can therefore drift from
the workspace release version without changing the fixture manifest.

## Delivered work

- Regenerated `tests/fixtures/sc-lint/bootstrap/Cargo.lock` from the current
  workspace patches.
- Added an integration test that copies the fixture to an OS temporary
  directory, removes its copied lockfile, and regenerates it with
  `cargo metadata --offline` under the repository's Cargo configuration.
- Kept the fixture-copy mutation check and made the test fail when the
  checked-in lockfile differs from Cargo's regenerated result.
- Extended the Windows-only cleanup budget for the real pinned-`bd`
  integration test, retrying only Windows sharing violations (`OS error 32`).

## Acceptance criteria

- A deliberately stale fixture lockfile fails the regeneration assertion.
- The committed fixture passes the same regeneration assertion offline.
- The fixture test uses standard OS temporary storage and inherits the
  repository `.cargo/config.toml` through its command working directory.
- Workspace formatting, targeted linting, and repository-boundary tests pass.
