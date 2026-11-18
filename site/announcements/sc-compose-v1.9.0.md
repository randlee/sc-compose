# sc-compose v1.9.0 — Convergent Re-Pour, Previewed Graph Revisions, Exact Sibling Pins

**Released:** November 17, 2026 · **Install:** `cargo install sc-compose` (Rust/crates.io), `pip install sc-compose` (Python ≥3.9/PyPI), `brew install randlee/tap/sc-compose` (macOS/Homebrew), `winget install randlee.sc-compose` (Windows) — the [release binaries](https://github.com/randlee/sc-compose/releases/tag/v1.9.0) are the preferred pinned install

[Changelog](https://github.com/randlee/sc-compose/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-compose/releases/tag/v1.9.0) · [Upgrade notes](#upgrading-from-v180)

---

## Beads Alchemist

**As a beads alchemist, I want to re-pour an updated plan onto a graph I already poured, so that revising a workflow does not mean tearing down live beads and losing their state.**

v1.8.0 ended with a hard limitation: `pour` and `attach` refused a run that mixed
planned additions with already-existing beads, because no atomic `bd` update carrying
ownership preconditions was qualified. v1.9.0 removes the limitation instead of the
use case. `sc-compose bead repour FORMULA --root <bead-id>` targets a prior pour root —
an attach graph uses its existing `parent` and `ref` — and converges the graph onto the
new plan. Existing steps are matched by provenance identity (mode, formula, parent, ref,
step), not by revision, input or node-definition hashes, so a comment or a
re-ordering does not read as a new bead. Plan-owned fields that changed are applied to
the existing bead, missing nodes and edges are created, and IDs and runtime state
(status, notes, assignee, close state) are preserved. Metadata merges without clobbering
unrelated user keys. An interrupted apply is completed by rerunning it, and an exact
replay changes nothing.

Removal is now explicit rather than destructive. Internal edges absent from the new plan
are removed — an internal edge being one where both endpoints belong to the graph.
External edges have an endpoint outside the graph, and every one of them is retained and
reported, in both directions, including a graph bead that gates outside work. Removed
nodes are retained in their existing status, labelled `repour:removed`, and listed with a
recommended separate remove command. Nothing that a human might still be working on is
deleted on the alchemist's behalf.

Previews are the contract. Dry-run and apply both list every change someone may or may
not accept: field changes on closed or in-progress beads, removed edges, retained
external edges and removed nodes. Dry-run predicts conflicts assuming the `bd` graph does
not move between runs, and dry-run and apply agree under that assumption. Pouring a
changed existing graph — including `pour --dry-run` and `preview_pour` — refuses with
`BEADS_GRAPH_CONFLICT`, whose recovery text names `repour` as the recommended command
and explains which fields changed. Repour refuses dirty or unknown database state by
default; `--allow-uncommitted` is the deliberate override. A partial failure reports
exactly what was applied and what is pending, with a rerun hint, and never claims nothing
was written. A blocking dependency cycle in the resulting graph is rejected before any
write, as is a conflicting edge type or a foreign or unowned planned ID. Run
`sc-compose help repour` and `sc-compose help repour-strategy` — both ship as bundled
help topics.

## Task-Template Author

**As a task-template author, I want variable discovery to follow the declared argument list, so that Jinja tag names never turn into phantom required inputs.**

Templates with frontmatter now use argument-only variable discovery even without
`--strict`. Jinja tag names and filter targets no longer count as variables, and
comments and literal raw-block bodies are skipped (#1043). The visible consequence is at
the boundary: passing an undeclared input such as `autoescape` now produces an
unknown/extra-input diagnostic under the configured unknown-variable policy instead of
being silently absorbed into the render. Templates without frontmatter keep the v1.7.0
default scan, so an unannotated template renders exactly as before. Strict diagnostics
and the public `ComposePolicy` structure are unchanged, and `--strict` remains opt-in.

`sc-compose examples` also behaves now: a bare listing succeeds, listings advertise the
required variables or the sample var-file for each pack, and an unknown pack name
suggests close names or matching template formats. A new `evidence-report` pack
(`.xhtml.j2`) renders well-formed XHTML, asserted by parsing the output as XML.

## Release & Ops Engineer

**As a release/ops engineer, I want workspace crates to resolve only to their exact siblings, so that a published composition lands on one coherent release instead of a mixture of versions.**

Workspace crate dependencies (`sc-sha`, `sc-composer`, `sc-composer-beads`) now require
the exact matching version, which prevents a consumer from resolving a newer sibling
release under a locked older one. The prerelease tag tool preserves those exact `=`
requirements when it bumps workspace versions, so the pin survives the release machinery
that could quietly undo it. The documentation was corrected in the same pass: the release
binary is the preferred pinned install, and the limits of a source install through Cargo
— where a lockfile can be regenerated under you — are now stated rather than implied.

Release-candidate qualification got stricter and broader at once: candidates build and
smoke-test the three Python packages across the six wheel platforms with CPython 3.9 and
3.11–3.15 (Windows ARM64 starts at 3.11), and publishing requires a passing check bound
to the candidate commit. Homebrew now installs the bundled examples under
`pkgshare/examples`, where pack discovery looks, so `sc-compose examples list` finds them
after `brew install` (#1039) — and release CI runs `examples list` against every staged
archive so a broken layout fails the release instead of the user. Winget's example layout
is not yet verified; that check follows the 1.9.0 publish.

## Python Developer

**As a Python developer, I want the same repour previews and structured changes as the CLI, so that a graph revision can be inspected before it is applied from my own tooling.**

The Python surface gains the repour path with the same shape as the CLI: dry-run previews
that report proposed changes and risks, structured revision changes, and
`allow_uncommitted=True` as the explicit dirty-state override. Partial failures surface
`BEADS_GRAPH_APPLY_PARTIAL` with applied and pending work where known, and receipt actions
carry the new `update`, `updated` and `retained` values additively, so existing consumers
that switch on the older actions keep working. Wheel coordinates and the abi3-py39 build
are unchanged — six runner coordinates per package, `requires-python >=3.9`.

## Library Consumer (Rust)

**As a Rust library consumer, I want the Beads revision API to be extensible without breaking me, so that embedding composition does not turn every release into a source migration.**

The Beads revision APIs are now built for growth: revision enums are non-exhaustive, and
receipt and options are constructed through `GraphRevisionOptions::new`,
`GraphApplyPartial::new` and `BeadGraph::new` rather than struct literals, with wildcard
match arms required on the operation, stage, outcome and revision enums. `GraphChange::FieldChange`
uses `GraphField` with `as_str()` / `as_flag()`, and its JSON field names are unchanged.
Partial-apply details add `cause_code` and `outcome_uncertain`, and unknown database-state
errors retain `cause_code`. Partial-error details are boxed to keep the error type small —
the same `result_large_err` pressure sc-agent's 0.2 line resolved the same way.

These are source-level changes without a major version bump, because the Beads API has no
external users yet; the decision is recorded in the changelog rather than assumed. Request
layouts and the rest of the public API are unchanged from v1.8.0.

---

## Upgrading from v1.8.0

- **`BEADS_GRAPH_ATOMIC_UPDATE_UNAVAILABLE` is retired.** Mixed graph updates are now
  supported through `repour`; the refusal branches are deleted. `pour`/`attach` still
  refuse a *changed existing* target — that refusal is `BEADS_GRAPH_CONFLICT` and directs
  the caller to `repour`.
- **Repour refuses dirty or unknown database state** unless `--allow-uncommitted` /
  `allow_uncommitted=True` is passed deliberately.
- **Rust:** construct revision enums and receipt/options structs through the `new`
  constructors, and add wildcard match arms — the enums are non-exhaustive.
- **Frontmatter templates** discover variables from the argument list only; an undeclared
  extra input now produces a diagnostic instead of being ignored.
- **Exact sibling pins:** workspace crates now require their exact matching versions, so a
  mixed-version resolution that previously succeeded will now fail to resolve rather than
  mislink.

---

> The reporting surface (`reports init` / `smoke` / `finalize` / `render-spec` / `index`
> / `verify` / `publish-manifest`) and the observability contract (FR-9 to FR-11) are
> unchanged in v1.9.0. Server-mode dirty/clean detection, real partial-failure injection
> and real graph cycles are deferred to the post-Phase W integration suite and remain
> known limitations.
