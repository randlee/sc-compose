# ADR-0023: Beads Attach and By-Path Pour

## Status

Accepted (2026-10-06, user approval on plan PR #619; proposed the same day in
phase t planning). Extends
[ADR-0021](0021-beads-formula-composition-integration.md); nothing in ADR-0021
is withdrawn.

## Context

ADR-0021 (Phase R) shipped `render`, `validate`, `preview-pour` and `pour`.
Two gaps were filed against it:

- **#615, by-path pour.** `preview-pour` / `pour` accept only a formula
  rendered into the active `formulas/` registry. A formula rendered anywhere
  else in the workspace is refused, and a fresh `bd init` workspace has no
  `formulas/` directory at all, so even `render` fails there with
  `BEADS_TEMPLATE_PATH_INVALID`.
- **#613, attach.** Callers need to create a formula's steps under an
  EXISTING bead (a release workflow under an epic, a dev -> sanity -> QA chain
  under each sprint, a review chain under each task) and to repeat that
  safely after an interruption. Beads offers no such operation: `bd mol pour`
  creates a new parentless molecule each time, and repeating `bd mol bond`
  was observed to reopen closed children and clear their notes.

Two facts shape the answer:

1. **sc-compose is the more capable composer.** Beads formulas have their own
   `{{var}}` substitution, `loop`, `expand`, `condition` and `gate`
   constructs. sc-compose templates already handle loops over numbers and
   lists, conditionals, includes and multi-pass rendering, with strict
   variable validation. Beads' substitution path has also had injection
   defects. Routing structure through Beads' engine adds a second, weaker
   template language and its defects.
2. **Beads v1.3.1 already has an atomic graph writer.** `bd create --graph
   <plan.json>` creates a set of beads and their edges in one storage
   transaction. A plan node may carry an explicit `id`, a `parent_id` naming
   an existing bead, `metadata`, and edges to existing beads by id; an
   explicit id that already exists refuses the whole plan; `--dry-run`
   validates the plan and writes nothing. (Beads source v1.3.1:
   `cmd/bd/graph_apply.go`; exercised against bd 1.3.1 during planning: an
   attach under an existing epic, a re-run refused with nothing written, and a
   `molecule` root that `bd mol show` reports as a molecule.)

Production Beads is v1.3.1. Nothing here may depend on an unreleased or
forked Beads.

## Decision

1. **Additive only.** The v1 request and receipt gain optional fields and new
   enum variants. Existing operations, stages, fields, codes and bd argv keep
   their names and meaning. `schema` stays `sc-compose/beads/v1`. Registry
   pour by name is unchanged and remains the way to use Beads-native formula
   features.
2. **sc-compose constructs; Beads validates and stores.** For the graph
   operations below, the rendered formula is final: every value is resolved
   and every loop and condition is already expanded by the sc-compose
   template. No `--var` is passed to bd and the formula may declare no
   `vars`, so Beads' substitution never runs.
3. **One graph engine, two modes.** A rendered formula is read through bd's
   own parser (`bd cook <path> --json`, which writes nothing), turned into a
   graph plan, and written with one `bd create --graph` call:
   - **pour mode** (`preview-pour` / `pour` of a formula outside the active
     registry, #615): a new root bead of type `molecule` (the type `bd mol
     pour` gives its roots) with the steps as its children;
   - **attach mode** (new `preview-attach` / `attach`, #613): the steps as
     children of an existing `parent`, with stable ids.
4. **Flat formulas.** The engine creates exactly the steps in bd's parse of
   the formula. Constructs that remain in that parse (variables, composition,
   advice, conditions, gates, nested children) are refused, never ignored.
   `extends`, `loop` and `expand` are resolved by bd while it parses, before
   sc-compose sees the formula; what they produce is held to the same rules
   (see "Formula grammar"). The sc-compose equivalents are template loops,
   conditionals and includes.
5. **Identity and safe repetition (attach).** Ids are `<parent>.<ref>-<step>`.
   Every created bead carries provenance metadata. A bead whose provenance
   matches is left untouched; anything else refuses the whole request before
   any write. sc-compose never edits, closes, reopens or deletes a bead.
6. **Atomic apply.** Everything a request creates is written by one `bd create
   --graph` transaction, so a request leaves either all of its missing beads
   and edges or none of them.
7. **Edges.** `needs` / `depends_on` become `blocks`; `relations[]` adds edges
   of any well-known bd dependency type except `parent-child` (reserved for the
   hierarchy) between steps and existing beads.
8. **No new dependency, no fork.** `sc-composer-beads` keeps its dependency set
   (CLAUDE.md Boundary Rule 11): it parses bd's JSON output with `serde_json`
   and never parses TOML; it hashes only through the `sc_composer::
   calculate_hash` re-export (ADR-0018). Everything uses bd v1.3.1 commands;
   no capability probe, fork build or proto is involved.

The sections below are the normative contract.

## Use cases

Each use case names the issue case it closes. The examples are generic; any
Beads workspace can use them.

### UC-1: Pour a formula rendered anywhere in the workspace (#615)

A team keeps formula templates in `workflows/` and renders them to `build/`.
The workspace was created with `bd init` and has no `formulas/` directory.

```sh
sc-compose bead render       --request render.json --json   # writes build/release-1.4.formula.toml
sc-compose bead preview-pour --request pour.json   --json   # writes nothing to bd
sc-compose bead pour         --request pour.json   --json   # one bd create --graph
```

`render` succeeds without a registry directory. `preview-pour` shows the root
and every step and edge; `pour` creates them in one transaction. The receipt
reports `pour_mode: "graph"` and maps each step to its new bead id. A formula
placed in the active registry still pours by name exactly as before
(`pour_mode: "registry"`).

### UC-2: Attach a workflow under an existing parent (#613 gap 1)

A project has epic `proj-42`. The release workflow (build -> verify ->
publish) is attached under it:

```json
{"schema":"sc-compose/beads/v1","operation":"attach","working_directory":"<repo>",
 "template":"workflows/release.formula.toml.j2","rendered_formula":"<repo>/build/release-1.4.formula.toml",
 "formula_name":"release","compose_variables":{"version":"1.4.0"},
 "parent":"proj-42","ref":"release",
 "pour_authorization":"CreatePersistentBeads"}
```

This creates `proj-42.release-build`, `proj-42.release-verify` and
`proj-42.release-publish` as children of `proj-42`, with `blocks` edges
verify -> build and publish -> verify. The children sit directly under the
parent (no intermediate container); `bd children proj-42` lists them, and
Beads' parent-child closure rules apply as for any child.

### UC-3: Preview before writing (#613 required behaviour)

`preview-attach` with the same request (no authorization needed) returns the
receipt's `graph` block with every node marked `create` or `existing` and
every edge `add` or `existing`, after bd has validated the plan with `bd
create --graph --dry-run`. Nothing is written to bd.

### UC-4: Re-run is a no-op (#613 gap 2)

Running UC-2 again finds three beads whose provenance matches; all nodes and
edges are `existing`, no bd write is issued, and the receipt succeeds. A bead
that was claimed, closed, annotated or edited since keeps all of that:
status, notes, assignee and fields are never compared and never written.

### UC-5: Resume after an interruption (#613 required behaviour)

The apply is one transaction, so an interrupted `attach` left either nothing
or everything. Re-running the same request creates whatever is missing (all of
it, or none). If a matching bead was deleted by hand, a re-run recreates just
that bead and its edges, again in one transaction.

### UC-6: A changed formula or scope is refused (#613 required behaviour)

After `proj-42.release-*` exist, attaching a re-rendered formula with
different content under the same `parent` and `ref` meets beads whose
provenance names another revision and is refused as `BEADS_GRAPH_CONFLICT`
before any write. A new attachment uses a new `ref`. A `parent` or `ref` in
`compose_variables` that differs from the top-level field is refused as
`BEADS_GRAPH_SCOPE_MISMATCH`.

### UC-7: Link to beads that already exist (#613 gap 1)

`relations[]` adds edges between a step and an existing bead, in either
direction, of any well-known bd dependency type except `parent-child`:

```json
"relations":[
  {"from":"step:publish","to":"bead:proj-17","type":"blocks"},
  {"from":"bead:proj-90","to":"step:verify","type":"validates"},
  {"from":"step:build","to":"bead:proj-3","type":"related"}]
```

An edge from an existing bead adds a dependency row only; no field or status
of the existing bead changes.

### UC-8: Generate many steps with a template loop

A batch of items, each blocked by the one before it. Bead templates use
`{{{ ... }}}` for sc-compose values (ADR-0021) and ordinary `{% ... %}` blocks;
`count` comes from `compose_variables` (`{"count": 10}`):

```jinja
formula = "batch"
version = 1
type = "workflow"
{% for i in range(1, count + 1) %}
[[steps]]
id = "item_{{{ i }}}"
title = "Process item {{{ i }}} of {{{ count }}}"
{% if i > 1 %}needs = ["item_{{{ i - 1 }}}"]
{% endif %}
{% endfor %}
```

Rendered with `count: 10`, this is a flat formula of ten steps. Attached under
`proj-42` with `ref: "batch"`, it creates `proj-42.batch-item_1` ...
`proj-42.batch-item_10` in one transaction. (Step ids use `_`, never `-`: see
"Id rule".) The same pattern loops over a list
from a var file (`items: [api, cli, docs]` -> one review chain per item), and
conditionals include or skip steps; Beads' own `loop` / `expand` are not
needed.

### UC-9: The same workflow under many parents (#613 phase-wide iteration)

A caller loops over parents, one request per parent, each with its own
`parent` and the same `ref`: `lab-d-1.chain-dev`, `lab-d-2.chain-dev`, ....
Each request is independent and idempotent, so a loop interrupted half-way is
simply re-run.

### UC-10: Plan output for other tools

Every graph receipt maps each step id to its bead id (`graph.ids`), so a
planner can validate and dispatch the created work without re-querying bd.

## Contract

### Public contract (Rust, additive)
```rust
pub const PROVENANCE_KEY: &str = "sc_compose_graph";

// validating newtypes; each is #[serde(try_from = "String", into = "String")], so the JSON is a plain string
pub struct BeadId(String);        // an existing or planned bead id (non-empty, no whitespace)
pub struct GraphRef(String);      // ^[A-Za-z0-9_-]{1,32}$  (may contain '-')
pub struct StepId(String);        // ^[A-Za-z0-9_]{1,64}$   (never '-')
pub struct Sha256Digest(String);  // "sha256:" + 64 lowercase hex

// existing enums gain variants (serde: snake_case, as today)
pub enum BeadOperation { Render, Validate, PreviewPour, Pour, PreviewAttach, Attach }
pub enum BeadStage { Render, Validate, ResolveActiveRegistry, PreviewPour, Pour, PreviewAttach, Attach }
// BeadStageOutcome { Succeeded, Skipped, Failed { code } } is unchanged.

// BeadComposeRequest gains (all serde-default, so Phase R requests parse unchanged):
pub parent: Option<BeadId>,                        // attach ops: required, an existing bead id; others: must be absent
#[serde(rename = "ref")] pub ref_: Option<GraphRef>,   // attach ops: required; others: must be absent
pub relations: Vec<BeadRelation>,                  // attach ops and graph-mode pour only

// BeadComposeReceipt gains (skip_serializing_if = "Option::is_none"):
pub pour_mode: Option<BeadPourMode>,               // pour ops only
pub graph: Option<BeadGraph>,                      // graph-mode pour and attach ops

#[serde(rename_all = "snake_case")] pub enum BeadPourMode { Registry, Graph }
#[serde(rename_all = "snake_case")] pub enum BeadGraphMode { Pour, Attach }

// every well-known bd v1.3.1 dependency type except parent-child (reserved for the hierarchy)
#[serde(rename_all = "kebab-case")]
pub enum BeadDependencyType {
    Blocks, ConditionalBlocks, WaitsFor, Related, DiscoveredFrom, RepliesTo, RelatesTo,
    Duplicates, Supersedes, AuthoredBy, AssignedTo, ApprovedBy, Attests, Tracks,
    Until, CausedBy, Validates, DelegatedFrom,
}

pub enum BeadEndpoint { Step(StepId), Bead(BeadId) }  // JSON string "step:<step id>" | "bead:<bead id>"
pub struct BeadRelation { pub from: BeadEndpoint, pub to: BeadEndpoint, #[serde(rename = "type")] pub kind: BeadDependencyType }

pub struct BeadGraph {
    pub mode: BeadGraphMode,
    pub parent: Option<BeadId>,                    // attach: the parent; pour: the new root once created
    #[serde(rename = "ref")] pub ref_: Option<GraphRef>,  // attach only
    pub formula: String,                           // formula_name
    pub revision: Sha256Digest,                    // see Revision
    pub plan_path: Option<PathBuf>,                // the bd graph plan, when this request had beads to create
    pub ids: BTreeMap<StepId, BeadId>,             // step id -> bead id (pour preview: empty, ids are assigned by bd)
    pub nodes: Vec<BeadGraphNode>,
    pub edges: Vec<BeadGraphEdge>,
}
pub struct BeadGraphNode { pub step: Option<StepId> /* None = pour root */, pub id: Option<BeadId>, pub action: BeadNodeAction }
pub struct BeadGraphEdge { pub from: String, pub to: String /* bead id, or "step:<id>" before bd assigns ids */, #[serde(rename = "type")] pub kind: String, pub action: BeadEdgeAction }
#[serde(rename_all = "snake_case")] pub enum BeadNodeAction { Create, Created, Existing }
#[serde(rename_all = "snake_case")] pub enum BeadEdgeAction { Add, Added, Existing }
pub struct MissingEdge { pub from: BeadId, pub to: BeadId, #[serde(rename = "type")] pub kind: String }

// bead metadata[PROVENANCE_KEY] on every bead the graph engine creates
pub struct BeadGraphProvenance {
    pub v: u32,                                    // provenance format, always 1
    pub mode: BeadGraphMode,
    pub formula: String,
    pub revision: Sha256Digest,
    pub inputs: Sha256Digest,                      // see Inputs
    pub parent: Option<BeadId>,                    // attach only
    #[serde(rename = "ref")] pub ref_: Option<GraphRef>,  // attach only
    pub step: Option<StepId>,                      // None on a pour root
}
```

A planned edge between two beads that both already exist, absent in bd
(someone removed it), is refused as `GraphEdgeMissing`: bd cannot apply a plan
that adds only edges, and reporting success would leave the graph incomplete.

### Errors (`BeadComposeError`, additive)
| Variant | Code | Outcome / exit | Recovery |
|---|---|---|---|
| `GraphParentNotFound { parent }` | `BEADS_GRAPH_PARENT_NOT_FOUND` | refused / 2 | create the parent or name an existing one |
| `GraphIdInvalid { field, value }` | `BEADS_GRAPH_ID_INVALID` | refused / 2 (validate) | correct the `ref` or step id |
| `GraphScopeMismatch { field, value }` | `BEADS_GRAPH_SCOPE_MISMATCH` | refused / 2 (validate) | make `compose_variables` agree with `parent` / `ref` |
| `GraphFormulaUnsupported { reason }` | `BEADS_GRAPH_FORMULA_UNSUPPORTED` | refused / 2 (validate) | express the construct in the template, or use registry pour |
| `GraphRelationInvalid { index, reason }` | `BEADS_GRAPH_RELATION_INVALID` | refused / 2 (validate or plan) | correct the relation |
| `GraphConflict { id, reason }` | `BEADS_GRAPH_CONFLICT` | refused / 2 (plan) | inspect the named bead (sc-compose never edits it) or use a new `ref` |
| `GraphEdgeConflict { from, to, existing, requested }` | `BEADS_GRAPH_EDGE_CONFLICT` | refused / 2 (plan) | inspect the existing edge, or change the relation |
| `GraphEdgeMissing { edges }` | `BEADS_GRAPH_EDGE_MISSING` | refused / 2 (plan) | run the `bd dep add` command the message gives for each edge, then re-run |
| `GraphReadFailed { command, status }` | `BEADS_GRAPH_READ_FAILED` | failed / 2 (plan) | fix the bd failure and re-run; nothing was written |
| `GraphApplyFailed { command, status }` | `BEADS_GRAPH_APPLY_FAILED` | failed / 2 (pour or attach) | fix the bd failure and re-run; nothing was written |

Field types: `command` is the bd argv as `Vec<String>`; `status` is
`Option<i32>` (None when bd was killed by a signal); `edges` is a non-empty
`Vec<MissingEdge>` in plan order, and the message lists, for each, the exact
command `bd dep add <from> <to> --type <type>`. Each `reason` is a closed
`snake_case` enum per variant (`GraphConflictReason`,
`GraphRelationInvalidReason`, `GraphFormulaUnsupportedReason`) whose values are
exactly the rows of the Conflict rules, Relation validation and Formula
grammar tables; the CLI prints its text form.

Request errors (`Err(BeadComposeError)`: Rust `Err`, Python exception, CLI exit
3) are only request-shape problems found before any stage: serde failures;
`parent` / `ref` missing on an attach op or present on another op;
non-empty `relations` on render or validate; non-empty `bead_variables` on an
attach op (`RequestDeserializationFailed`); and the existing Phase R request
codes. Everything in the table is a receipt (Rust `Ok`, Python receipt, CLI
exit 2).

A pour learns its mode only after resolving the active registry (see
"Stages"), so its two mode-dependent misuses are refused receipts, before any
write: non-empty
`bead_variables` in graph mode is `GraphFormulaUnsupported`
(`bead_variables_set`), and non-empty `relations` in registry mode is
`GraphRelationInvalid` (`registry_pour`).

Scope: the top-level `parent` and `ref` are authoritative. A template may also
receive them through `compose_variables` to bake them into text; any such value
must equal the top-level field, else `GraphScopeMismatch` in the validate
stage, before any bd call. sc-compose never injects `parent` or `ref` into
`compose_variables`.

Authorization and confinement: `attach` and `pour` require `pour_authorization:
CreatePersistentBeads` (existing `BEADS_POUR_AUTH_REQUIRED` /
`BEADS_POUR_AUTH_INVALID`, exit 3, before any bd call). `preview-attach` and
`preview-pour` need none and issue no mutating bd command. `rendered_formula`
and the plan file are confined to `working_directory` with the existing
symlink and UTF-8 checks.

### Pour mode selection
`preview-pour` / `pour` keep ADR-0021's registry path when the rendered
formula's parent directory is the active registry (`bd where` then
`formulas/`): same stages, same argv, `pour_mode: "registry"`. Any other
rendered formula inside `working_directory` uses the graph engine in pour mode
(`pour_mode: "graph"`), and `render` no longer requires the registry directory
to exist. A rendered formula outside `working_directory` is refused as before.

### Id rule
`ref` matches `^[A-Za-z0-9_-]{1,32}$` (no `.`; `-` allowed) and every step id
matches `^[A-Za-z0-9_]{1,64}$` (no `.`: bd's hierarchy separator; no `-`), else
`GraphIdInvalid`. In attach mode the bead id is `<parent>.<ref>-<step>`.
Because a step id has no `-`, the LAST `-` after `<parent>.` separates ref
from step, so distinct (ref, step) pairs under one parent always give distinct
ids, and a ref such as `qa1-f1-r1` (a finding group, round 1) works unchanged:
`<sprint>.qa1-f1-r1-fix`. The ids start with the parent's prefix, which bd's
explicit-id prefix check accepts; sc-compose never passes `--force`. In pour
mode bd assigns every id and the receipt reports them; step ids follow the
same rule.

### Revision
The sha256 of the newline-normalized UTF-8 text of the rendered formula:
`sc_composer::calculate_hash` (ADR-0018 re-export; strict UTF-8, CRLF/CR
normalized to LF), written `sha256:<hex>`. Line-ending-only differences are
the same revision. A rendered formula that is not valid UTF-8 is
`GraphFormulaUnsupported` (`not_utf8`).

The revision depends only on sc-compose's own output, never on the format of
bd's parse, so a bd upgrade that changes `bd cook` JSON does not change any
revision and does not make earlier attachments conflict. The cost: a changed
`extends` base (a file sc-compose did not render) is not detected while the
rendered text is unchanged; a re-run then creates nothing new and leaves the
earlier beads as they are. Attach a changed base under a new `ref`.

### Inputs
`inputs` is `sha256:<hex>` of the canonical JSON (sorted keys, compact) of
`{"mode": <mode>, "relations": <request.relations sorted by (from, to, type)>}`.
`revision` already covers every rendered value (`compose_variables` shape the
rendered bytes, and the formula declares no `vars`). A re-run with changed
relations meets beads whose provenance differs and is refused as
`GraphConflict` before any write.

### Formula grammar
The formula is read from bd's own parse, `bd cook <path> --json` (compile
mode: no `--var`, no `--persist`, no `--mode`, no `--search-path`; bd v1.3.1
prints the parsed and resolved formula as JSON and writes nothing), and
deserialized with `serde_json` into a crate-private struct mirroring bd
v1.3.1's `Formula` / `Step` JSON. TOML and JSON formulas are handled alike. A
non-zero `bd cook` exit is the existing `BEADS_COOK_FAILED`.

bd resolves three constructs while it parses, so they never reach sc-compose
as constructs; the engine sees only their output (verified against bd 1.3.1
during planning):

| Construct in the rendered formula | What `bd cook` does | Result |
|---|---|---|
| `extends = [...]` | merges the base formula's steps, found in bd's own formula search paths; the key is gone from the parse | the merged steps are checked like any other. The revision does not cover the base (see "Revision"). Base not found: `BEADS_COOK_FAILED` |
| step `loop` | replaces the step with its body, once per iteration, with ids `<step>.iter<n>.<body id>` | always refused: `GraphIdInvalid` (`field: "step"`), because the ids contain `.` |
| step `expand` | replaces the step with the expansion formula's `template` steps, ids from that template (conventionally `<step>.<id>`) | ids containing `.` or `-`: `GraphIdInvalid`; otherwise checked like any other step. Expansion formula missing or not `type = "expansion"`: `BEADS_COOK_FAILED` |

Do not rely on these in a graph formula: write the structure in the sc-compose
template, where a preview shows it. Everything else is checked on the parse:

| Formula content (in bd's parse) | Result (`GraphFormulaUnsupported` reason) |
|---|---|
| allowed top-level keys: `formula`, `description`, `version`, `type` = `workflow`, `steps`, informational `source` / `phase` / `pour` / `intent`, and bd's output envelope key `schema_version` | accepted |
| non-empty `vars` | `vars_declared` (resolve values in the template) |
| non-empty request `bead_variables` on a graph-mode pour | `bead_variables_set` |
| non-empty `template`, `compose`, `advice` or `pointcuts`; another `type` | `composition` (bd keeps these keys in its parse; `advice` is refused even though bd has already applied it) |
| unknown top-level key | `unknown_key` |
| allowed step keys: `id`, `title`, `description`, `notes`, `type`, `priority`, `labels`, `metadata`, `assignee`, `needs`, `depends_on` | accepted |
| non-empty `children`, `expand_vars`, `condition`, `gate`, `on_complete` or `waits_for` | `step_construct` (use template loops, conditionals and includes) |
| unknown step key | `unknown_key` |
| step `metadata` containing `PROVENANCE_KEY` | `reserved_metadata` |
| a label containing a comma | `label_comma` |
| no steps, a duplicate step id, or `needs` / `depends_on` naming no step | `step_graph` |

Step fields map to bd graph-plan node fields of the same name; `type` and
`priority` keep bd's defaults when absent. Text is passed literally: a `{{...}}`
left in rendered text stays as written, because no bd substitution runs.

### Graph plan
When the request has at least one bead to create, the engine writes one bd
graph plan (bd v1.3.1 `GraphApplyPlan` JSON) to
`<rendered_formula>.graph.json` beside the rendered formula, in preview and
apply alike, and reports it as `graph.plan_path`. It is the file bd reads;
sc-compose does not delete it. When nothing is missing, no plan file is
written and `plan_path` is absent. The plan contains only what is missing:

- **Attach mode:** one node per missing step, with `key` = step id, explicit
  `id` = `<parent>.<ref>-<step>`, `parent_id` = `parent`, the step's fields,
  and `metadata` = the step's metadata plus `PROVENANCE_KEY`.
- **Pour mode:** a root node (`key` = `_root`, `type` = `molecule`, `title` =
  `formula`, `description` = the formula description) and one node per step
  with `parent_key` = `_root`; every node carries provenance.
- **Edges:** for each step, one `blocks` edge per `needs` / `depends_on` entry
  (`from` = the waiting step, `to` = the step it waits on), then one edge per
  relation (`step:` endpoints resolve to plan keys, `bead:` endpoints to
  `from_id` / `to_id`). Only edges with at least one endpoint being created in
  this request are written.

Preview runs `bd create --graph <plan> --dry-run --json` and apply runs `bd
create --graph <plan> --json`. bd validates the whole plan (types, priority,
ids, cycles, blocking paths through the hierarchy) before writing and applies
it in one transaction, returning `{"ids": {key: id}}`. A failure of either
command is `GraphApplyFailed`; bd has written nothing. When every planned node
and edge already exists with matching provenance, no file and no bd write is
issued.

### Conflict rules (plan stage, before any write)
The plan stage reads the parent, each planned id and each `bead:` relation
endpoint with one `bd show <id>... --json`, and the edges of each existing
planned bead with `bd dep list <id> --json`. A bead is absent only on bd's
not-found response: the id is missing from a list that `bd show` returned with
exit 0, or `bd show` exits 1 with JSON `error` equal to `no issues found
matching the provided IDs` (every id absent). Any other failure of a read
(another exit, unparseable output, a killed process) is `GraphReadFailed` with
nothing written; it is never read as "absent".

| Case | Result (`GraphConflict` reason, or other code) |
|---|---|
| attach `parent` does not exist | `GraphParentNotFound` |
| a bead at a planned id whose `PROVENANCE_KEY` equals the planned provenance (all fields) | `existing`, untouched; status, notes, assignee and fields are not compared |
| a bead at a planned id with no provenance | `not_owned` |
| a bead at a planned id whose provenance differs (formula, revision, inputs, parent, ref or step) | `provenance_differs` |
| an existing edge between two planned endpoints with another type | `GraphEdgeConflict` |
| an existing edge between two planned endpoints with the same type | `existing`, untouched |
| a planned edge between two existing beads, absent in bd | `GraphEdgeMissing`, listing every such edge with its `bd dep add` command |

A race in which another writer creates a planned id between the plan stage and
the apply makes bd refuse the whole plan (explicit id exists):
`GraphApplyFailed` with nothing written; a re-run then classifies that bead by
the rules above.

### Relation validation
Decided in the validate stage from the request and bd's parse, before any `bd
show`; existence of `bead:` endpoints is checked in the plan stage.

| Case | Result |
|---|---|
| endpoint string without `step:` / `bead:` prefix, or unknown `type` | `RequestDeserializationFailed` (exit 3) |
| `step:` endpoint naming no step in the formula | `GraphRelationInvalid` (`unknown_step`) |
| `from` equals `to` | `GraphRelationInvalid` (`self_edge`) |
| both endpoints `bead:` | `GraphRelationInvalid` (`no_step`) |
| two relations, or a relation and a `needs` edge, on the same ordered pair | `GraphRelationInvalid` (`duplicate`) |
| a relation between a step and `bead:<parent>` in either direction | `GraphRelationInvalid` (`parent_pair`) |
| `bead:` endpoint naming a bead that does not exist (plan stage) | `GraphRelationInvalid` (`bead_not_found`) |
| non-empty `relations` on a registry-mode pour | `GraphRelationInvalid` (`registry_pour`) |

### Stages
| Operation | Stages | bd commands, in order |
|---|---|---|
| preview-pour / pour, registry | render, validate, resolve_active_registry, preview_pour / pour (unchanged) | unchanged from ADR-0021 |
| preview-pour / pour, graph | render, validate, resolve_active_registry, preview_pour / pour | validate: `bd cook <path> --dry-run` (unchanged; the mode is not known yet); resolve_active_registry: `bd where --json`, which selects graph mode; preview_pour / pour: `bd cook <path> --json`, `bd show` (only for `bead:` relation endpoints), then `bd create --graph <plan> --dry-run --json` / `bd create --graph <plan> --json` |
| preview-attach / attach | render, validate, preview_attach / attach | validate: `bd cook <path> --json`; preview_attach / attach: `bd show`, `bd dep list`, then `bd create --graph <plan> --dry-run --json` / `bd create --graph <plan> --json` (none when nothing is missing) |

A pour reaches `resolve_active_registry` in both modes; the receipt reports
that stage as succeeded and `pour_mode` says which mode it selected. In a
graph-mode pour, the formula checks this ADR places in the validate stage
(Formula grammar, Id rule, Relation validation, `bead_variables_set`,
`registry_pour`) run at the start of `preview_pour` / `pour` and fail that
stage. The plan-stage reads run inside the `preview_*` / `pour` / `attach`
stage. Exit codes follow the existing policy: 0 succeeded, 2 refused or failed
receipt, 3 request or usage error.

### Write rule
The only mutating bd command sc-compose issues for these operations is one
`bd create --graph` per request. It never issues `bd update`, `bd close`, `bd
reopen`, `bd delete`, `bd dep add`, `bd mol pour` or `bd mol bond` for them,
and never `bd cook --persist`, so no existing bead changes and no proto is
created.

## Issue coverage

| Issue case | Covered by |
|---|---|
| #615: `render` fails when `formulas/` does not exist | Pour mode selection; UC-1 |
| #615: a formula outside the registry cannot be previewed or poured | Pour mode selection, Graph plan; UC-1 |
| #615: preview must not write (the `--persist` caveat) | Graph plan (`--dry-run`), Write rule |
| #615: registry requirement and same-name TOML/JSON ambiguity | not consulted in graph mode; registry pour unchanged |
| #613 gap 1: pour creates a parentless molecule | attach mode; UC-2 |
| #613 gap 1: a dependency on an external bead fails | Relations; UC-7 |
| #613 gap 2: repeated pour duplicates, repeated bond resets children | Identity, Conflict rules, Write rule; UC-4 |
| #613: preview shows hierarchy and blocking edges before mutation | UC-3 |
| #613: explicit persistent-write authorization | Authorization |
| #613: same request is a no-op or a clear conflict; never reset status, notes, claims, evidence | Conflict rules; UC-4, UC-6 |
| #613: recover from partial execution | Atomic apply; UC-5 |
| #613: reject a conflicting revision or scope | Revision, Scope; UC-6 |
| #613: receipt mapping parent and generated ids | `BeadGraph`; UC-10 |
| #613: children directly under the parent or under a container | directly under the parent; UC-2 |
| #613: phase-wide iteration optional | UC-9; template loops UC-8 |
| #613: keep formula rendering, metadata baking and `needs` edges | Graph plan field mapping |
| #613 note: attach takes the rendered formula by path, no registry placement | Formula grammar (`bd cook <path>`) |

## Consequences

- Phase R callers see no change: same requests, same argv for registry
  formulas, same stages and codes.
- A formula rendered anywhere in the workspace pours on production bd v1.3.1,
  with no registry directory, no proto and no Beads fork.
- Attach is safe to re-run and to resume; a request is all-or-nothing.
- Workflow structure (loops, per-item chains, optional steps) is written once,
  in the sc-compose template language, and Beads' substitution and expansion
  engine is not on the path.
- Beads-native formula features (`loop`, `expand`, `gate`, `vars`) remain
  available through registry pour. The graph engine refuses the ones that
  survive bd's parse, and documents exactly what happens to the ones bd
  resolves while parsing (`extends`, `loop`, `expand`).
- Beads created by other tools carry no provenance under `PROVENANCE_KEY`, so
  attach refuses them as `not_owned` rather than adopting them.

## Rejected alternatives

### Repeat `bd mol pour` / `bd mol bond` under the parent
Not idempotent: pour creates a new molecule each time, and repeated bond was
observed to reopen closed children.

### By-path `bd mol pour <path>` behind a capability probe
Depends on an unreleased Beads change (fork, upstream release unknown) and
still routes values and structure through Beads' substitution and expansion.

### `bd cook --persist` to a temporary proto, pour it, delete it
Needs namespacing, ownership checks and cleanup to keep protos from
accumulating, can leave a proto behind on failure, and still uses Beads'
substitution. One `bd create --graph` writes the same beads directly.

### One `bd create` / `bd dep add` per bead and edge
Not atomic: an interruption leaves part of a workflow and forces partial-apply
recovery rules. `bd create --graph` is one transaction.

### Use Beads `vars`, `loop` and `expand` for structure
A second, more limited template language next to sc-compose's, with its own
defects; sc-compose templates already express these constructs.

### Parse TOML formulas in `sc-composer-beads`
Adds a dependency Boundary Rule 11 does not allow and duplicates bd's parser.

### Copy the rendered formula into `formulas/`
Writes outside the requested path, may not exist in a fresh workspace, and
leaves stale, request-specific formulas in a shared by-name registry.

## References

- [ADR-0018: sc-sha Hash Ownership and Boundary](0018-sc-sha-hash-ownership.md)
- [ADR-0021: Beads Formula Composition Host-Neutral Integration](0021-beads-formula-composition-integration.md)
- GitHub issues #613 (attach) and #615 (by-path pour)
- Beads v1.3.1 source: `cmd/bd/graph_apply.go` (`bd create --graph`),
  `cmd/bd/cook.go`, `cmd/bd/template.go`, `internal/formula/parser.go`,
  `internal/types/types.go` (dependency types)
