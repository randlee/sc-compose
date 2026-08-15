---
id: FIX-PR507-RELEASE-CHANNEL-RUNTIME
status: complete
branch: feature/publish-kit-preflight-hardening
---

## Closure Checklist

- [x] Remove the literal quotes from the preflight manifest argument so the
  helper receives the repository-relative path.
- [x] Render the Homebrew formula with the Homebrew Pathname destination from
  the manifest and verify Ruby syntax.
- [x] Render a Scoop manifest at the bucket's documented `bucket/` path and
  verify JSON parsing with strings that require escaping.
- [x] Run focused regression tests, manifest validation, workflow YAML parsing,
  formatter, and diff checks; then critically re-read these findings.

## Scope Guard

Workflow/tooling correction only. Do not dispatch, tag, or publish a release.
