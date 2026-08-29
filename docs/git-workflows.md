# Git Workflows

## Branch Policy

- Use a `main` / `develop` git flow.
- Keep the primary checkout on `main` or `develop`, not feature branches.
- Create feature branches from `develop`.
- Open normal pull requests against `develop`.
- Merge `develop` into `main` for release readiness and release publication.
- Use git worktrees when parallel tasks need isolated branches.

## Naming Convention

- Phase plan doc: `docs/phase-X/phase-X-plan.md` (e.g. `docs/phase-C/phase-C-plan.md`).
- Prose sprint identifiers (doc `id:` field, headings, running text) use dot
  notation matching the phase letter's own case: `X.#` (e.g. `C.2`).
- Sprint doc filenames and worktree names use lowercase kebab syntax:
  `sprint-x-#-<description>.md` (e.g.
  `docs/phase-C/sprint-c-3-python-release-train.md`) — lowercase phase
  letter, dash-separated from the sprint number, dash-separated from a short
  kebab-case description.
- Python companion sprint prose identifiers append `-py` to the Rust sprint
  id: `X.#-py` (e.g. `D.2-py`).
- Python companion sprint doc filenames use the same Rust sprint number plus a
  `-py-` marker: `sprint-x-#-py-<description>.md` (e.g.
  `docs/phase-D/sprint-d-2-py-bindings.md`).
- Sprint numbers are contiguous within a phase, starting at 1, with no gaps.
  See `.claude/skills/plan-hardening/sprint-planning-guidelines.md`.
- `-py` companions do not consume additional entries in the contiguous Rust
  sprint-number sequence; they inherit the number of the Rust sprint they wrap.
- Branch names are always lowercase.
- Integration branch: `integrate/phase-x` (e.g. `integrate/phase-c`).
- Sprint branch: `sprint/x-#-<description>` (e.g.
  `sprint/c-2-python-release-train`), matching its sprint doc's number and
  description.
- Python companion sprint branches follow `sprint/x-#-py-<description>`
  (e.g. `sprint/d-2-py-bindings`), again inheriting the Rust sprint number
  rather than consuming a new one.
- If a Rust sprint later splits (for example `D.4` into `D.4` + `D.5`), any
  already-existing `D.4-py` companion remains attached to `D.4`'s retained
  scope. The newly-created Rust sprint gets its own new companion
  (`D.5-py`) only if it exposes bindable `sc-composer` surface and once its
  post-split scope is stable enough to document.
- This naming convention applies going forward; already-shipped phase/sprint
  docs and branches (phases A and B) are not retroactively renamed.

## Pull Request Rule

Every change should follow:
1. Create a feature branch from `develop`, or, for phase sprint work, from
   the parent declared by that phase's `integrate/phase-x` integration model
   (see Phase Integration Rule below).
2. Run local validation.
3. Push the branch.
4. Open a PR to `develop`, or, for phase sprint work, to the declared phase
   parent branch.
5. Wait for CI and review before merge.

## Phase Integration Rule

Every phase always uses its `integrate/phase-x` branch (see Naming
Convention above) — sprint work never merges straight into `develop`:

1. Create `integrate/phase-x` from `develop` before the phase's first sprint
   starts, if it does not already exist.
2. By default, every sprint branch in the phase branches from
   `integrate/phase-x`, and its PR targets `integrate/phase-x`, not `develop`.
   A phase plan may instead explicitly declare one linear `gh stack` chain.
   In that mode, `integrate/phase-x` is the bottom branch, the first sprint
   targets it, and every later sprint targets its immediately preceding sprint.
   The declared parent chain is authoritative; no sprint may create an
   independent root at `integrate/phase-x`.
3. In the default model, sprint PRs merge into `integrate/phase-x` once that
   sprint's own review passes and CI is green. In the explicit linear-stack
   model, all sprint PRs remain open as reviewed stack layers and are merged
   bottom-to-top only at phase close with `gh stack merge --yes`.
4. In the default model, `integrate/phase-x` should periodically merge
   `develop` forward in-phase to avoid large end-of-phase conflicts. In the
   explicit linear-stack model, use `gh stack sync` to cascade-rebase the
   complete stack onto the updated trunk; do not ad-hoc merge `develop` into
   only the integration root.
5. Only after every sprint has either merged into `integrate/phase-x` (the
   default model) or passed review and CI as a stack layer (the explicit
   linear-stack model) does the phase-ending review run.
6. Only after the phase-ending review passes does `integrate/phase-x` merge
   into `develop`. This is the only PR in the phase that targets `develop`
   directly; in linear-stack mode it is the bottom PR merged atomically with
   its reviewed sprint descendants.
7. A phase plan doc's `branch:` frontmatter value must match this phase's
   `integrate/phase-x` name.

## Release Rule

- Release tags must come from `main`, not from `develop` or a feature branch.
- Publishing must follow the order in `release/publish-artifacts.toml`.
- Release automation must validate publish order and version alignment before
  any tag or publish step runs.
- Release-track PRs (e.g. a release-blocking fix discovered after `develop`
  has already merged to `main`) go straight to `main`: cut a `release/*`
  branch from `main`, PR into `main`, publish from `main`. Do not route
  through `develop` first — that costs a second CI cycle and a bidirectional
  sync before the release gate (which checks commit ancestry, not content)
  will pass.
- After publish, backpropagate the `release/*` branch's commits to `develop`
  as routine post-release housekeeping (a normal `main` -> `develop` sync
  PR). This is not a pre-publish gate.

## Worktree Rule

- Do not switch the main checkout away from `main` for sprint work.
- Create dedicated worktrees for long-running or parallel tasks.
- Remove worktrees only after the user approves cleanup.
