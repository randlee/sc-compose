# SC-Compose Project Plan

## Status

This repo is in initial extraction/setup.

The immediate goal is to establish:
- correct standalone crate boundaries
- zero `agent-team-mail-*` dependencies
- a clean publishable workspace structure

## Near-Term Work

1. Set up repository git flow:
   - use `main` and `develop`
   - feature branches target `develop`
   - release tags and release publication come from `main`
   - keep repo workflow and review discipline aligned with ATM
2. Match GitHub automation and protection to ATM:
   - CI triggers match the ATM repo pattern for `pull_request` and `push`
   - branch protection and rulesets match ATM for `main` and `develop`
   - GitHub secrets and environments are configured and use the same variable
     names as ATM where the workflows overlap
3. Verify repository setup end to end:
   - release preflight validates publish order and version alignment
   - release workflow is ready to publish `sc-composer` then `sc-compose`
   - workspace version stays above the source ATM workspace version that last
     published these crate names
4. Complete crates.io ownership and release readiness:
   - verify crate ownership/maintainers for `sc-composer` and `sc-compose`
   - verify publish tokens and first-release permissions
   - document the handoff from ATM-published crates to this repo
5. Make `sc-composer` fully standalone.
6. Remove any `ATM_HOME` or ATM path assumptions from `sc-compose`.
7. Verify ATM cutover readiness:
   - published crate names match the existing names used in ATM
   - replacement instructions are documented
   - no `agent-team-mail-*` dependencies remain
8. Write the migration plan after the agents are live and operating on the new
   repos.

## Implementation Phase

After repository extraction is stable, the next implementation phase is the
FR-1 through FR-9 redesign defined in:

- `docs/requirements.md`
- `docs/architecture.md`

That phase includes:

1. Implement the resolver layout for `.claude/{agents,commands,skills}` and
   `.agents/{agents,commands,skills}`.
2. Implement the normalized frontmatter schema and scalar variable model.
3. Implement undeclared-token behavior for default mode and strict mode.
4. Implement CLI support for profile/file mode, variable files, guidance/prompt
   inputs, and deterministic output path handling.
5. Implement trait-hook observability in `sc-composer` and the concrete
   `sc-observability` binding in `sc-compose`.
6. Implement `frontmatter-init` and `init` with repository validation behavior.
7. Add tests covering the full FR-1 through FR-9 surface.

If this phase is deferred or split into sprints, the split must preserve the
normative behavior defined in the requirements and architecture docs.

## Rule

Any sprint plan added here must preserve the standalone boundary defined by:
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/git-workflows.md`
- `docs/publishing.md`
