---
id: FIX-PR507-RELEASE-CHANNEL-RUNTIME
status: complete
branch: feature/publish-kit-preflight-hardening
---

## Closure Checklist

- [x] Remove the literal quotes from the preflight manifest argument so the
  helper receives the repository-relative path.
- [x] Select every manifest-declared Homebrew formula for the requested
  `release_track`, render every declared `binaries` archive entry, verify Ruby
  syntax, and commit every selected formula path. Stable tags select `stable`
  entries; prerelease tags select only `prerelease` entries.
- [x] Render a Scoop manifest at the bucket's documented `bucket/` path and
  verify JSON parsing with strings that require escaping.
- [x] Run focused regression tests, manifest validation, workflow YAML parsing,
  formatter, and diff checks; then critically re-read these findings.

## Scope Guard

Workflow/tooling correction only. Do not dispatch, tag, or publish a release.
