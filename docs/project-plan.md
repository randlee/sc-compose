# SC-Compose Project Plan

## Status

This repo is in initial extraction/setup.

The immediate goal is to establish:
- correct standalone crate boundaries
- zero `agent-team-mail-*` dependencies
- a clean publishable workspace structure

## Near-Term Work

1. Verify repository setup end to end:
   - CI workflow runs on pull requests and `main`
   - release preflight validates publish order and version alignment
   - release workflow is ready to publish `sc-composer` then `sc-compose`
   - workspace version stays above the source ATM workspace version that last published these crate names
2. Make `sc-composer` fully standalone.
3. Remove any `ATM_HOME` or ATM path assumptions from `sc-compose`.
4. Verify ATM cutover readiness:
   - published crate names match the existing names used in ATM
   - replacement instructions are documented
   - no `agent-team-mail-*` dependencies remain

## Rule

Any sprint plan added here must preserve the standalone boundary defined by:
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/git-workflows.md`
- `docs/publishing.md`
