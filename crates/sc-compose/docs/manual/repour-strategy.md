# Plan a re-pour safely

Use `sc-compose help repour` for command syntax and receipt details. This guide
helps decide whether a plan revision belongs in an existing workflow and how
to review its consequences.

## Choose pour or repour

Use `pour` for an independent run that needs its own root, ids and history.
Use `repour` to revise the plan of work already represented by an existing
graph. Keep its original pour root, or its attachment's parent and ref.
Changing those identifiers creates a different scope rather than migrating
the original work.

A targeted pour of a changed plan refuses with `BEADS_GRAPH_CONFLICT` and
recommends `repour`; its dry-run predicts the same conflict. Follow that
recovery only after deciding that changing the existing plan is intended.
A foreign bead or an invalid target is a different conflict: inspect the exact
reason instead of treating every refusal as permission to overwrite it.

## Establish a recoverable baseline

1. Coordinate one writer for the plan and choose a revision window.
2. Save the formula and all inputs so preview, apply and recovery use the same
   plan. Review template conditions as well as the visible formula edits.
3. Review the entire bd database's uncommitted work, commit it with
   `bd dolt commit`, and sync the intended baseline. Database history is the
   recovery mechanism; repour does not provide a transaction spanning every
   write.
4. Run `repour --dry-run` with the original target and read every change.
   Dirty or unknown database state refuses with its exact cause. Use
   `--allow-uncommitted` only when you accept that baseline risk; an unsupported
   dirty-state probe is unknown, not evidence of a clean database.

Commit and sync history allows deliberate rollback using your bd/Dolt
workflow. Restoring the database can also affect unrelated changes. Coordinate
that recovery with other users; do not automatically reset a shared database
or assume that editing the formula back will undo every side effect.

## Review the plan as work, not just structure

A field change may affect a bead that is already in progress or closed. Check
its before/after title, description, design, acceptance criteria, priority,
labels and metadata against the work actually performed. Repour preserves
status, notes and assignee: updating acceptance criteria does not validate a
closed bead against the new criteria or assign new work to anyone.

Read dependency direction carefully. Internal edges absent from the revised
plan are removed. For example, flattening `build -> review -> publish` can
release work that previously waited on a prerequisite; reversing an edge
changes what can run first. Check that the intended plan is valid before
accepting these scheduling changes.

External edges are retained and individually reported in both directions.
An external gate remains effective, and an outside bead blocked by a graph
bead stays blocked. Repour is not a way to bypass those dependencies. Review
them even when no internal edge changes.

When a condition removes a step, that bead keeps its status and receives
`repour:removed`; its internal edges are pruned. Remove or condition any
relations to the omitted step too, since a dangling reference is refused
before mutation. Use structured booleans or explicit comparisons for CLI
string inputs, rather than interpreting the string `"false"` as false.

## Apply and reconcile

Apply the same formula, inputs and target after accepting the dry-run. If the
graph changes between preview and apply, run a new preview. Compare the actual
receipt with the intended field and edge changes, including retained external
edges and removed nodes.

On partial failure, read what was applied, what is pending, and the exact
failed operation and cause. Inspect uncertain outcomes, fix the cause and
rerun the same command. Sequential writes converge on rerun; they are not
rolled back automatically. A fresh pour is a new workflow, not a retry.

## Clean up removed nodes separately

Repour never deletes removed beads. Both dry-run and apply list them with
their current status and a recommended separate remove command. Treat that
list as a review queue, not a script to execute automatically.

Before using a suggested command:

1. Inspect the bead's status, notes, assignee and history. Decide whether the
   retained record is still needed.
2. Inspect incoming and outgoing external dependencies. They survived repour;
   deleting the bead later deletes its edges too and may release outside work
   or remove an outside gate relationship.
3. Coordinate cleanup with the owners of affected work, then run only the
   specific suggested command you accepted. Retaining a labelled bead is valid
   when its history or external relationships are still useful.

Review the resulting graph and keep the database history needed for recovery.
Cleanup is a separate decision from approving the plan revision.
