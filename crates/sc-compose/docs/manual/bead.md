# Beads formula composition

`sc-compose bead` turns sc-compose templates into Beads work: it renders a
formula template, checks it with `bd`, and creates beads from it. Each command
takes one complete `sc-compose/beads/v1` JSON request file and returns one
receipt. The same manual is available in the installed binary with
`sc-compose help bead`.

```text
sc-compose bead render         --request request.json [--json]
sc-compose bead validate       --request request.json [--json]
sc-compose bead preview-pour   --request request.json [--json]
sc-compose bead pour           --request request.json [--json]
sc-compose bead preview-attach --request request.json [--json]
sc-compose bead attach         --request request.json [--json]
```

The subcommand determines `operation`; every other value comes from the request
file. There are no partial request flags. `bd` 1.3.1 or newer must be on `PATH`
(or named by `bd_executable`) for every operation except `render`.

## Which operation do I need?

| Goal | Operation |
|---|---|
| Produce the formula file only | `render` |
| Check that `bd` accepts the rendered formula | `validate` |
| Create a new workflow (a new root bead with its steps) | `preview-pour`, then `pour` |
| Add a workflow under a bead that already exists, safely re-runnable | `preview-attach`, then `attach` |

The `preview-*` operations never write to Beads; run them first and read the
receipt. `pour` and `attach` write, and only with the exact authorization value
`CreatePersistentBeads`.

## Two ways to build structure

sc-compose renders the formula before `bd` sees it, so structure can be built
in either of two places:

- **In the sc-compose template (recommended).** Loops, conditionals and
  includes run at render time and produce a flat list of steps with every
  value filled in. This is how `attach` and graph-mode `pour` work: `bd`
  receives the finished formula and runs none of its own substitution.
- **In Beads' formula language.** Beads `vars`, `{{ name }}` placeholders,
  `loop`, `expand`, `condition` and `gate` are evaluated by `bd` at pour time.
  They are available only through registry pour (below).

In a bead template, sc-compose values use triple braces, `{{{ value }}}`, and
blocks use ordinary `{% ... %}`. Double-brace `{{ name }}` text is never
touched by sc-compose; it is left for Beads in registry pour and kept as
literal text by `attach` and graph-mode `pour`.

## The request

```json
{
  "schema": "sc-compose/beads/v1",
  "operation": "attach",
  "working_directory": "/work/project",
  "template": "workflows/release.formula.toml.j2",
  "rendered_formula": "/work/project/build/release-1.4.formula.toml",
  "formula_name": "release",
  "compose_variables": {"version": "1.4.0", "reviewers": ["api", "cli"]},
  "bead_variables": {},
  "parent": "proj-42",
  "ref": "release",
  "relations": [],
  "pour_authorization": "CreatePersistentBeads"
}
```

| Field | Required | Meaning |
|---|---|---|
| `schema` | yes | Always `sc-compose/beads/v1`. |
| `operation` | yes | Set by the subcommand. |
| `working_directory` | yes | Absolute workspace root. Templates and rendered formulas must stay inside it; symlinks out of it are refused. |
| `template` | yes | The `.formula.toml.j2` or `.formula.json.j2` template, relative to `working_directory`. |
| `rendered_formula` | yes | Where the rendered `.formula.toml` / `.formula.json` is written. Any existing directory inside `working_directory`; no Beads `formulas/` directory is needed. |
| `formula_name` | for pour and attach | The formula's name. |
| `compose_variables` | yes (may be `{}`) | Structured JSON values for the template (`{{{ ... }}}`, loops, conditionals). |
| `bead_variables` | yes (may be `{}`) | Scalar values passed to `bd` as `--var` by registry pour only. Must be `{}` for attach and graph pour. |
| `bd_executable` | no | Path to `bd`; default `bd` on `PATH`. |
| `pour_authorization` | for `pour` and `attach` | Exactly `CreatePersistentBeads`. |
| `parent` | for attach ops only | Id of the existing bead to attach under. |
| `ref` | for attach ops only | Name of this attachment, `[A-Za-z0-9_]{1,32}` (no `-` or `.`). |
| `relations` | no | Extra edges; attach operations and graph pour only. See "Linking to existing beads". |

## render and validate

`render` writes the rendered formula and nothing else. `validate` renders, then
runs `bd cook <rendered_formula> --dry-run`, which checks the formula with
Beads' own parser and writes nothing.

## pour: create a new workflow

Where the rendered formula lives decides how it is poured, and the receipt's
`pour_mode` reports which one ran:

- **Registry pour** (`pour_mode: "registry"`): the rendered formula is in the
  active Beads registry, `<beads dir>/formulas/<formula_name>.formula.toml`
  (or `.json`). It pours by name with `bd mol pour`, exactly as earlier
  releases did, and `bead_variables` are passed as `--var`. Use this when the
  formula relies on Beads `vars`, `loop`, `expand` or `gate`. A same-name TOML
  and JSON pair in the registry is refused as ambiguous.
- **Graph pour** (`pour_mode: "graph"`): the rendered formula is anywhere else
  inside `working_directory`, for example `build/`. sc-compose reads it through
  `bd cook <path> --json`, writes a graph plan beside it
  (`<rendered_formula>.graph.json`), and creates a new root bead of type
  `molecule` plus one child per step, with all edges, in a single `bd create
  --graph` transaction. The root shows up in `bd mol show` like any poured
  molecule. Each `pour` creates a new workflow with new ids.

```shell
sc-compose bead preview-pour --request pour.json --json   # plan and validate; no writes
sc-compose bead pour         --request pour.json --json   # create
```

## attach: add a workflow under an existing bead

`attach` creates the formula's steps as children of `parent`, with stable ids
`<parent>.<ref>-<step id>`. Given parent `proj-42`, ref `release` and steps
`build`, `verify`, `publish`, it creates `proj-42.release-build`,
`proj-42.release-verify` and `proj-42.release-publish`, each a direct child of
`proj-42`, with `blocks` edges from each step to the steps it `needs`.

```shell
sc-compose bead preview-attach --request attach.json --json   # what would be created
sc-compose bead attach         --request attach.json --json   # create
```

Attach is safe to repeat:

- **Re-run:** every bead it created carries provenance metadata
  (`sc_compose_graph`: formula, revision, inputs, parent, ref, step). A bead
  whose provenance matches is reported `existing` and left alone. Running the
  same request again writes nothing and succeeds.
- **Work in progress is never touched:** status, notes, assignee, claims and
  every other field of an existing bead are never compared or written.
  sc-compose never updates, closes, reopens or deletes a bead.
- **All or nothing:** everything missing is created in one `bd create --graph`
  transaction, so an interrupted `attach` leaves either nothing or the whole
  workflow. Just run it again.
- **Changes are refused, not merged:** if the rendered formula changed (a
  different revision), the relations changed, or a bead at a planned id was
  not created by this attachment, the whole request is refused with
  `BEADS_GRAPH_CONFLICT` before anything is written. To attach a new version
  alongside, use a new `ref`.
- **Scope must agree:** if `compose_variables` also carries `parent` or `ref`
  (for example to print them in step text), the values must equal the
  top-level fields, else `BEADS_GRAPH_SCOPE_MISMATCH`.

To attach one workflow under many parents, loop in your script: one request
per parent, each with its own `parent`. Every request is independent, so an
interrupted loop is simply run again.

## What a formula may contain for attach and graph pour

Steps are flat. Allowed step keys: `id`, `title`, `description`, `notes`,
`type`, `priority`, `labels`, `metadata`, `assignee`, `needs`, `depends_on`.
Step ids match `[A-Za-z0-9_-]{1,64}`. The formula may not declare `vars`, and a
step may not use `children`, `expand`, `loop`, `condition`, `gate`,
`waits_for` or `on_complete`; formula-level `extends`, `compose`, `advice` and
`pointcuts` are refused too. Each of these is refused with
`BEADS_GRAPH_FORMULA_UNSUPPORTED` and a reason, never silently ignored. Write
the same structure with template loops and conditionals, or use registry pour.

## Recipes

### A chain of N steps

`compose_variables`: `{"count": 10}`

```jinja
formula = "batch"
version = 1
type = "workflow"
{% for i in range(1, count + 1) %}
[[steps]]
id = "item-{{{ i }}}"
title = "Process item {{{ i }}} of {{{ count }}}"
{% if i > 1 %}needs = ["item-{{{ i - 1 }}}"]
{% endif %}
{% endfor %}
```

Attached under `proj-42` with `ref: "batch"`, this creates
`proj-42.batch-item-1` ... `proj-42.batch-item-10`, each blocked by the one
before it.

### One review chain per item in a list

`compose_variables`: `{"components": ["api", "cli", "docs"]}`

```jinja
formula = "reviews"
version = 1
type = "workflow"
{% for c in components %}
[[steps]]
id = "{{{ c }}}-review"
title = "Review {{{ c }}}"

[[steps]]
id = "{{{ c }}}-fix"
title = "Address {{{ c }}} review findings"
needs = ["{{{ c }}}-review"]
{% endfor %}

[[steps]]
id = "signoff"
title = "Sign off"
needs = [{% for c in components %}"{{{ c }}}-fix"{% if not loop.last %}, {% endif %}{% endfor %}]
```

### Optional steps

```jinja
{% if security_review %}
[[steps]]
id = "security"
title = "Security review"
needs = ["build"]
{% endif %}
```

### Shared step blocks

Put common steps in a fragment and include it with sc-compose's `@<path>`
include syntax, so many formulas share one definition.

### dev -> sanity -> QA under every sprint

One formula with steps `dev`, `sanity` (needs `dev`) and `qa` (needs
`sanity`); a script runs `attach` once per sprint with `parent` = the sprint
bead and `ref: "chain"`. Each sprint gets `<sprint>.chain-dev`,
`<sprint>.chain-sanity` and `<sprint>.chain-qa`.

## Linking to existing beads

`relations` adds edges that involve at least one step. An endpoint is
`"step:<step id>"` or `"bead:<existing bead id>"`:

```json
"relations": [
  {"from": "step:publish", "to": "bead:proj-17", "type": "blocks"},
  {"from": "bead:proj-90", "to": "step:verify", "type": "validates"},
  {"from": "step:build",   "to": "bead:proj-3",  "type": "related"}
]
```

`from` depends on `to`: the first entry makes `publish` wait for `proj-17`. An
edge from an existing bead adds a dependency only and changes nothing else
about that bead. `type` is any Beads dependency type except `parent-child`:
`blocks`, `conditional-blocks`, `waits-for`, `related`, `discovered-from`,
`replies-to`, `relates-to`, `duplicates`, `supersedes`, `authored-by`,
`assigned-to`, `approved-by`, `attests`, `tracks`, `until`, `caused-by`,
`validates`, `delegated-from`.

Refused with `BEADS_GRAPH_RELATION_INVALID`: an unknown step, an edge from a
bead to itself, two `bead:` endpoints, a duplicate of another relation or a
`needs` edge, any relation between a step and the attach `parent`, and a
`bead:` id that does not exist.

## The receipt

Every operation returns a `sc-compose/beads/v1` receipt: `outcome`
(`succeeded`, `refused` or `failed`), and one entry per stage with the `bd`
argv, exit status, elapsed time and bounded output excerpts. Graph pour and
attach add a `graph` block:

```json
"graph": {
  "mode": "attach",
  "parent": "proj-42",
  "ref": "release",
  "formula": "release",
  "revision": "sha256:...",
  "plan_path": "/work/project/build/release-1.4.formula.toml.graph.json",
  "ids": {"build": "proj-42.release-build", "verify": "proj-42.release-verify"},
  "nodes": [{"step": "build", "id": "proj-42.release-build", "action": "created"}],
  "edges": [{"from": "proj-42.release-verify", "to": "proj-42.release-build", "type": "blocks", "action": "added"}]
}
```

Node actions are `create` (preview) / `created` (applied) / `existing`. Edge
actions are `add` / `added` / `existing` / `missing`; `missing` means an edge
between two existing beads was removed by someone since, and sc-compose leaves
it removed. `ids` maps every step to its bead id so a planner can dispatch the
work without querying `bd` again.

## Codes

Request errors (exit `3`, no receipt stages): a malformed request,
`BEADS_REQUEST_DESERIALIZATION_FAILED` (including `parent`/`ref` missing on an
attach operation or present on another, `bead_variables` set for an attach
operation, `relations` on `render` or `validate`), `BEADS_UNKNOWN_SCHEMA`, path
and authorization errors such as `BEADS_POUR_AUTH_REQUIRED`.

A pour only knows whether it is a registry or graph pour after locating the
registry, so two pour mistakes are refused receipts (exit `2`) instead:
`bead_variables` set on a graph pour (`BEADS_GRAPH_FORMULA_UNSUPPORTED`,
reason `bead_variables_set`) and `relations` on a registry pour
(`BEADS_GRAPH_RELATION_INVALID`, reason `registry_pour`).

Refused or failed receipts (exit `2`):

| Code | Meaning | What to do |
|---|---|---|
| `BEADS_GRAPH_PARENT_NOT_FOUND` | `parent` does not exist | create it or name an existing bead |
| `BEADS_GRAPH_ID_INVALID` | bad `ref` or step id | fix the name |
| `BEADS_GRAPH_SCOPE_MISMATCH` | `compose_variables` disagrees with `parent`/`ref` | make them equal, or drop them |
| `BEADS_GRAPH_FORMULA_UNSUPPORTED` | the formula uses a construct graph mode does not take | express it in the template, or use registry pour |
| `BEADS_GRAPH_RELATION_INVALID` | a relation is malformed or names a missing bead | fix the relation |
| `BEADS_GRAPH_CONFLICT` | a bead at a planned id is not this attachment's | inspect it; use a new `ref` |
| `BEADS_GRAPH_EDGE_CONFLICT` | an edge exists with another type | inspect it, or change the relation |
| `BEADS_GRAPH_APPLY_FAILED` | `bd create --graph` failed; nothing was written | fix the cause shown in the stage output and re-run |

Earlier codes are unchanged, for example `BEADS_RENDER_FAILED`,
`BEADS_COOK_FAILED`, `BEADS_PREVIEW_POUR_FAILED`, `BEADS_POUR_FAILED` and
`BEADS_FORMULA_REGISTRY_AMBIGUOUS`.

## JSON output and human output

Use `--json` for the standard sc-compose diagnostic envelope. On success its
payload is the receipt. On a request error its payload carries the stable code
and message. Human output is derived from the receipt and does not include the
`bd` version.

## Troubleshooting

- **`bd` cannot cook in proxied-server mode.** `validate`, graph pour and
  attach read the formula with `bd cook`; use an embedded or external-server
  Beads workspace.
- **A step's `{{ name }}` text appears verbatim.** In attach and graph pour,
  Beads substitution does not run; render the value with `{{{ name }}}`.
- **A re-run says `BEADS_GRAPH_CONFLICT`.** The template, its values or the
  relations changed since the first run. Attach the new version under a new
  `ref`, or keep the old inputs.
- **`bd children <parent>` does not list the steps after `preview-attach`.**
  Preview writes nothing; run `attach`.
