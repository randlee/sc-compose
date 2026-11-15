# Re-pour an existing workflow

`sc-compose bead repour` applies an edited formula to the graph that an earlier
pour or attach created. It keeps bead ids and work history while changing the
plan. Read this guide with `sc-compose help repour`; use
`sc-compose help repour-strategy` to plan a revision and its cleanup.

## Choose the target

Use `pour` to create a new workflow. Use `repour` when the existing workflow
should follow a revised plan. A pour aimed at an existing graph refuses a
changed plan with `BEADS_GRAPH_CONFLICT`; its human and JSON recovery name
`repour` and explain the conflict. `pour --dry-run` and `preview-pour` predict
that refusal before writing.

For a previously poured workflow, keep the original root id and supply the
revised formula and its inputs:

```sh
sc-compose bead repour release.formula.toml.j2 --root proj-7 --dry-run --json
sc-compose bead repour release.formula.toml.j2 --root proj-7 --json
```

For an attached workflow, keep its original `parent` and `ref` in the JSON
request. Point `template` at the revised formula and keep the request's
`pour_authorization` set to `CreatePersistentBeads` for the real apply:

```sh
sc-compose bead repour --request attach-request.json --dry-run --json
sc-compose bead repour --request attach-request.json --json
```

Use the same variables in preview and apply. `--dry-run` reads and validates
without writing to Beads. Its prediction assumes the graph and inputs
do not change before apply. Keep one writer per plan; if another actor changes
the graph, review a fresh dry-run before continuing.

## Read the dry-run

Inspect the complete change list, rather than only the number of new beads:

| Change | Meaning and decision |
| --- | --- |
| Field change | Check the bead id, current status, field, and before/after values. A closed or in-progress bead can receive a new title, description, design, acceptance criteria, priority, declared labels or plan metadata. |
| New node or edge | Confirm its identity and dependency direction. Missing nodes and declared edges are created, including an edge between existing beads. |
| Removed internal edge | Both endpoints belong to this graph. The old dependency is absent from the new plan and will be removed, including edges involving removed nodes. |
| Retained external edge | An endpoint is outside this graph. Every such edge remains and is reported, in both directions. An outside gate still gates the graph; an outside bead still waits on its graph blocker. |
| Removed node | The bead remains in its current status, gains `repour:removed`, and loses obsolete internal edges. Review the suggested separate remove command; repour does not delete the bead. |

Repour never writes runtime status, notes or assignee, including on closed
beads. Explicit plan values update plan-owned fields; unspecified fields are
retained, and unrelated user metadata is preserved. A changed description on
a closed bead therefore does not reopen its work or prove the revised work
was performed.

Conditional templates follow the same rules. Enabling an optional step or
edge adds it; disabling a step retains the bead with `repour:removed` and
prunes its internal edges. Update relations too: an edge to an omitted step
is a dangling reference, refused before mutation. For `--var` strings, compare
explicit values such as `flag == "true"`; the nonempty string `"false"` is
not a typed boolean false.

## Commit database changes before applying

Repour checks uncommitted Dolt changes across the entire bd database, including
changes unrelated to this graph. It refuses with `BEADS_GRAPH_DIRTY` when
changes exist, or `BEADS_GRAPH_DIRTY_UNKNOWN` when the state cannot be read.
The refusal gives the exact cause. Unknown does not mean clean: older bd or
embedded mode may not support the read-only `bd sql` probe.

Review and commit the intended database state with `bd dolt commit`, then
retry. If the probe remains unsupported or you deliberately accept applying
on top of uncommitted changes, pass `--allow-uncommitted` explicitly:

```sh
sc-compose bead repour release.formula.toml.j2 --root proj-7 \
  --allow-uncommitted --dry-run --json
sc-compose bead repour release.formula.toml.j2 --root proj-7 \
  --allow-uncommitted --json
```

The override bypasses the dirty/unknown-state refusal; it does not make the
operation atomic or bypass plan validation. Commit and sync a known baseline
before revising so Dolt history can support rollback. See the strategy guide
before restoring a database shared with other work.

## Apply, inspect, and retry

The target, planned ids, references and edge types are validated before the
first write. Invalid targets and foreign or unowned planned ids are refused
with an exact reason; they are not adopted by changing the root or ref.

Repour applies sequentially and reads back its results. Review the real
receipt as well as the preview: it reports changed fields, removed internal
edges, retained external edges and retained removed nodes. An unchanged replay
is a no-op.

There is no all-or-nothing guarantee across writes. `BEADS_GRAPH_APPLY_PARTIAL`
reports the applied and pending work, failed operation and cause, with a rerun
hint; uncertain write outcomes must be inspected rather than treated as
unwritten. Fix the reported cause and rerun the same intended plan and target
to finish. Do not start a fresh pour to recover a partial repour: that creates
a different workflow. Review removed nodes for separate cleanup as described
in `sc-compose help repour-strategy`.
