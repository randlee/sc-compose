//! Repour against a stateful in-memory bd: convergence, refusals and interruption.
//!
//! The fake implements exactly the bd commands the library issues. Any other
//! command panics, so a delete or a fresh pour cannot happen unnoticed. An
//! interruption is injected by failing the Nth mutating call, which is
//! deterministic: there are no timers, threads or sleeps.
use sc_composer_beads::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, PartialEq)]
struct Store {
    rows: BTreeMap<String, Value>,
    parents: BTreeMap<String, String>,
    deps: Vec<(String, String, String)>,
    next: u32,
}

struct FakeBd {
    store: Mutex<Store>,
    cooked: Mutex<String>,
    mutations: Mutex<Vec<Vec<String>>>,
    fail_at: Mutex<Option<usize>>,
}

fn reply(status: i32, stdout: &str, stderr: &str) -> ProcessOutput {
    ProcessOutput {
        exit_status: Some(status),
        stdout: stdout.into(),
        stderr: stderr.into(),
        elapsed: Duration::ZERO,
    }
}

impl FakeBd {
    fn new() -> Self {
        Self {
            store: Mutex::new(Store::default()),
            cooked: Mutex::new(String::new()),
            mutations: Mutex::new(Vec::new()),
            fail_at: Mutex::new(None),
        }
    }

    fn cook(&self, steps: &Value) {
        *self.cooked.lock().unwrap() =
            json!({"formula":"plan","type":"workflow","steps":steps}).to_string();
    }

    fn snapshot(&self) -> Store {
        self.store.lock().unwrap().clone()
    }

    fn mutation_count(&self) -> usize {
        self.mutations.lock().unwrap().len()
    }

    fn fail_nth_mutation(&self, n: Option<usize>) {
        *self.mutations.lock().unwrap() = Vec::new();
        *self.fail_at.lock().unwrap() = n;
    }

    /// Runtime state a worker sets on a bead; plan fields are never touched.
    fn set_runtime(&self, id: &str, status: &str, notes: &str, assignee: &str) {
        let mut store = self.store.lock().unwrap();
        let row = store.rows.get_mut(id).unwrap();
        row["status"] = json!(status);
        row["notes"] = json!(notes);
        row["assignee"] = json!(assignee);
    }

    fn edit_row(&self, id: &str, edit: impl FnOnce(&mut Value)) {
        edit(self.store.lock().unwrap().rows.get_mut(id).unwrap());
    }

    fn row(&self, id: &str) -> Value {
        self.store.lock().unwrap().rows[id].clone()
    }

    fn ids_by_step(&self) -> BTreeMap<String, String> {
        let store = self.store.lock().unwrap();
        store
            .rows
            .iter()
            .filter_map(|(id, row)| {
                let step = row["metadata"]["sc_compose_graph"]["step"].as_str()?;
                Some((step.to_owned(), id.clone()))
            })
            .collect()
    }

    fn root(&self) -> String {
        let store = self.store.lock().unwrap();
        store
            .rows
            .iter()
            .find(|(_, row)| row["metadata"]["sc_compose_graph"]["step"].is_null())
            .map(|(id, _)| id.clone())
            .expect("a pour root")
    }

    /// Returns true when this mutating call must fail.
    fn begin_mutation(&self, args: &[String]) -> bool {
        let mut mutations = self.mutations.lock().unwrap();
        mutations.push(args.to_vec());
        *self.fail_at.lock().unwrap() == Some(mutations.len())
    }

    fn create_graph(&self, path: &str) -> ProcessOutput {
        let plan: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let mut store = self.store.lock().unwrap();
        let mut keys = BTreeMap::new();
        for node in plan["nodes"].as_array().unwrap() {
            let id = if let Some(id) = node.get("id").and_then(Value::as_str) {
                id.to_owned()
            } else {
                store.next += 1;
                format!("bd-{}", store.next)
            };
            keys.insert(node["key"].as_str().unwrap().to_owned(), id.clone());
            let mut row = node.clone();
            row["id"] = json!(id);
            row["status"] = json!("open");
            row["notes"] = json!("");
            row["assignee"] = json!("");
            store.rows.insert(id, row);
        }
        for node in plan["nodes"].as_array().unwrap() {
            let id = keys[node["key"].as_str().unwrap()].clone();
            let parent = node
                .get("parent_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| keys.get(node.get("parent_key")?.as_str()?).cloned());
            if let Some(parent) = parent {
                store.parents.insert(id, parent);
            }
        }
        let endpoint = |edge: &Value, side: &str| -> String {
            edge.get(format!("{side}_id"))
                .and_then(Value::as_str)
                .map_or_else(
                    || keys[edge[format!("{side}_key")].as_str().unwrap()].clone(),
                    str::to_owned,
                )
        };
        for edge in plan["edges"].as_array().cloned().unwrap_or_default() {
            let dep = (
                endpoint(&edge, "from"),
                endpoint(&edge, "to"),
                edge["type"].as_str().unwrap().to_owned(),
            );
            store.deps.push(dep);
        }
        let ids: BTreeMap<_, _> = keys.into_iter().collect();
        reply(0, &json!({"ids": ids}).to_string(), "")
    }

    fn update(&self, args: &[String]) -> ProcessOutput {
        let split = args.iter().position(|arg| arg == "--").unwrap();
        let id = &args[split + 1];
        let mut store = self.store.lock().unwrap();
        let row = store.rows.get_mut(id).expect("update of a known bead");
        for flag in &args[1..split] {
            let (name, value) = flag.split_once('=').expect("--flag=value form");
            match name {
                "--title" => row["title"] = json!(value),
                "--description" => row["description"] = json!(value),
                "--design" => row["design"] = json!(value),
                "--acceptance" => row["acceptance_criteria"] = json!(value),
                "--priority" => row["priority"] = json!(value.parse::<i64>().unwrap()),
                "--add-label" => {
                    let mut labels = row["labels"].as_array().cloned().unwrap_or_default();
                    labels.push(json!(value));
                    row["labels"] = Value::Array(labels);
                }
                "--metadata" => row["metadata"] = serde_json::from_str(value).unwrap(),
                other => panic!("repour must not write {other}"),
            }
        }
        reply(0, "{}", "")
    }
}

impl ProcessRunner for FakeBd {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        Ok(self.dispatch(spec))
    }
}

impl FakeBd {
    fn dispatch(&self, spec: &CommandSpec) -> ProcessOutput {
        let args = &spec.args;
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        match words.as_slice() {
            ["where", "--json"] => reply(
                0,
                &json!({"path": spec.working_directory.join(".beads")}).to_string(),
                "",
            ),
            ["cook", ..] => reply(0, &self.cooked.lock().unwrap(), ""),
            ["show", "--json", "--", ids @ ..] => {
                let store = self.store.lock().unwrap();
                let rows: Vec<Value> = ids
                    .iter()
                    .filter_map(|id| store.rows.get(*id).cloned())
                    .collect();
                if rows.is_empty() {
                    return reply(
                        1,
                        r#"{"error":"no issues found matching the provided IDs"}"#,
                        "",
                    );
                }
                reply(0, &Value::Array(rows).to_string(), "")
            }
            ["children", "--json", "--", root] => {
                let store = self.store.lock().unwrap();
                let rows: Vec<Value> = store
                    .parents
                    .iter()
                    .filter(|(_, parent)| parent.as_str() == *root)
                    .map(|(id, _)| store.rows[id].clone())
                    .collect();
                reply(0, &Value::Array(rows).to_string(), "")
            }
            ["dep", "list", "--json", "--", id] => {
                let store = self.store.lock().unwrap();
                let mut rows: Vec<Value> = store
                    .deps
                    .iter()
                    .filter(|(from, _, _)| from == id)
                    .map(|(_, to, kind)| json!({"id": to, "dependency_type": kind}))
                    .collect();
                if let Some(parent) = store.parents.get(*id) {
                    rows.push(json!({"id": parent, "dependency_type": "parent-child"}));
                }
                reply(0, &Value::Array(rows).to_string(), "")
            }
            ["create", "--graph", path, ..] => {
                if words.contains(&"--dry-run") {
                    return reply(0, "{}", "");
                }
                if self.begin_mutation(args) {
                    return reply(1, "", "injected failure");
                }
                self.create_graph(path)
            }
            ["update", ..] => {
                if self.begin_mutation(args) {
                    return reply(1, "", "injected failure");
                }
                self.update(args)
            }
            ["dep", "add", "--type", kind, "--", from, to] => {
                if self.begin_mutation(args) {
                    return reply(1, "", "injected failure");
                }
                self.store.lock().unwrap().deps.push((
                    (*from).to_owned(),
                    (*to).to_owned(),
                    (*kind).to_owned(),
                ));
                reply(0, "{}", "")
            }
            other => panic!("repour issued an unexpected bd command: {other:?}"),
        }
    }
}

struct Workspace {
    root: PathBuf,
    request: BeadComposeRequest,
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Workspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "sc-repour-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join(".beads")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let template = root.join("plan.formula.toml.j2");
        fs::write(&template, "formula = \"plan\"\n").unwrap();
        Self {
            request: BeadComposeRequest {
                schema: BEADS_SCHEMA_V1.into(),
                operation: BeadOperation::Pour,
                working_directory: root.clone(),
                template,
                rendered_formula: root.join("plan.formula.toml"),
                compose_variables: serde_json::Map::new(),
                formula_name: Some(FormulaName::new("plan").unwrap()),
                bead_variables: BTreeMap::new(),
                bd_executable: Some(PathBuf::from("fake-bd")),
                pour_authorization: Some(PourAuthorization::CreatePersistentBeads),
                parent: None,
                ref_: None,
                relations: Vec::new(),
            },
            root,
        }
    }

    fn pour(&mut self, bd: &FakeBd) -> BeadComposeReceipt {
        self.request.operation = BeadOperation::Pour;
        self.request.parent = None;
        execute_bead_request_with_runner(&self.request, bd).unwrap()
    }

    fn repour(&mut self, bd: &FakeBd, root: &str) -> BeadComposeReceipt {
        self.request.operation = BeadOperation::Repour;
        self.request.parent = Some(BeadId::new(root).unwrap());
        execute_bead_request_with_runner(&self.request, bd).unwrap()
    }

    /// Names of leftover temporary input snapshot files in the working directory.
    fn leftovers(&self) -> Vec<String> {
        fs::read_dir(&self.root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".sc-compose-input"))
            .collect()
    }
}

fn plan_v1() -> Value {
    json!([
        {"id":"build","title":"Build","description":"compile"},
        {"id":"verify","title":"Verify","needs":["build"]}
    ])
}

fn plan_v2() -> Value {
    json!([
        {"id":"build","title":"Build release","description":"compile everything"},
        {"id":"verify","title":"Verify","needs":["build"]}
    ])
}

fn plan_v3() -> Value {
    json!([
        {"id":"build","title":"Build release","description":"compile everything"},
        {"id":"verify","title":"Verify","needs":["build"]},
        {"id":"ship","title":"Ship","needs":["verify"]}
    ])
}

fn plan_v4() -> Value {
    json!([
        {"id":"build","title":"Build release","description":"compile everything"},
        {"id":"verify","title":"Verify","needs":["build"]},
        {"id":"ship","title":"Ship","needs":["verify","build"]}
    ])
}

fn poured(plan: &Value) -> (Workspace, FakeBd, String) {
    let mut w = Workspace::new();
    let bd = FakeBd::new();
    bd.cook(plan);
    let receipt = w.pour(&bd);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let root = bd.root();
    (w, bd, root)
}

fn dep(from: &str, to: &str, kind: &str) -> (String, String, String) {
    (from.into(), to.into(), kind.into())
}

#[test]
fn three_repours_keep_ids_and_runtime_state_and_create_nothing_fresh() {
    let (mut w, bd, root) = poured(&plan_v1());
    let ids = bd.ids_by_step();
    let (build, verify) = (ids["build"].clone(), ids["verify"].clone());
    bd.set_runtime(&build, "closed", "built on tuesday", "alice");
    bd.set_runtime(&verify, "in_progress", "half done", "bob");
    let beads_before = bd.snapshot().rows.len();

    // 1. change a field
    bd.cook(&plan_v2());
    bd.fail_nth_mutation(None);
    let receipt = w.repour(&bd, &root);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let graph = receipt.graph.unwrap();
    assert!(graph.nodes.iter().any(|node| {
        node.step.as_ref().is_some_and(|s| s.as_str() == "build")
            && node.action == BeadNodeAction::Updated
    }));
    assert_eq!(bd.ids_by_step()["build"], build);
    assert_eq!(bd.row(&build)["title"], "Build release");
    assert_eq!(bd.row(&build)["description"], "compile everything");
    assert_eq!(bd.row(&build)["status"], "closed");
    assert_eq!(bd.row(&build)["notes"], "built on tuesday");
    assert_eq!(bd.row(&build)["assignee"], "alice");
    assert_eq!(bd.row(&verify)["status"], "in_progress");
    assert_eq!(bd.snapshot().rows.len(), beads_before);

    // 2. add a node
    bd.cook(&plan_v3());
    let receipt = w.repour(&bd, &root);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let ids = bd.ids_by_step();
    assert_eq!((&ids["build"], &ids["verify"]), (&build, &verify));
    let ship = ids["ship"].clone();
    assert_eq!(bd.snapshot().rows.len(), beads_before + 1);
    assert!(bd.snapshot().deps.contains(&dep(&ship, &verify, "blocks")));
    assert_eq!(bd.snapshot().parents[&ship], root);

    // 3. add an edge between existing beads
    bd.cook(&plan_v4());
    let receipt = w.repour(&bd, &root);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let graph = receipt.graph.unwrap();
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.action == BeadEdgeAction::Added)
    );
    let store = bd.snapshot();
    assert!(store.deps.contains(&dep(&ship, &build, "blocks")));
    assert_eq!(store.rows.len(), beads_before + 1, "nothing poured fresh");
    assert_eq!(bd.ids_by_step()["ship"], ship);
    assert_eq!(bd.row(&build)["status"], "closed");
    assert_eq!(bd.row(&verify)["assignee"], "bob");
    assert_eq!(w.leftovers(), Vec::<String>::new());

    // a replay is a no-op
    bd.fail_nth_mutation(None);
    let before = bd.snapshot();
    let replay = w.repour(&bd, &root);
    assert_eq!(replay.outcome, BeadOutcome::Succeeded, "{replay:#?}");
    assert_eq!(bd.snapshot(), before);
    assert_eq!(bd.mutation_count(), 0);
}

#[test]
fn removed_nodes_and_edges_are_kept_and_reported() {
    let (mut w, bd, root) = poured(&plan_v3());
    let ids = bd.ids_by_step();
    bd.cook(&json!([
        {"id":"build","title":"Build","description":"compile"},
        {"id":"verify","title":"Verify"}
    ]));
    bd.fail_nth_mutation(None);
    let before = bd.snapshot();
    let receipt = w.repour(&bd, &root);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let graph = receipt.graph.unwrap();
    assert!(graph.nodes.iter().any(|node| {
        node.step.as_ref().is_some_and(|s| s.as_str() == "ship")
            && node.action == BeadNodeAction::Retained
    }));
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.action == BeadEdgeAction::Retained)
    );
    let after = bd.snapshot();
    assert_eq!(after.rows.len(), before.rows.len());
    assert_eq!(after.deps, before.deps, "no dependency is removed");
    assert_eq!(bd.ids_by_step(), ids);
}

#[test]
fn an_interruption_at_every_apply_boundary_completes_on_rerun() {
    // Reference: one uninterrupted repour of the same edit.
    let (mut reference, reference_bd, reference_root) = poured(&plan_v1());
    reference_bd.cook(&plan_v4());
    reference_bd.fail_nth_mutation(None);
    let done = reference.repour(&reference_bd, &reference_root);
    assert_eq!(done.outcome, BeadOutcome::Succeeded, "{done:#?}");
    let total = reference_bd.mutation_count();
    assert!(total >= 3, "create, updates and edges: {total}");
    let expected = reference_bd.snapshot();

    for boundary in 1..=total {
        let (mut w, bd, root) = poured(&plan_v1());
        bd.cook(&plan_v4());
        bd.fail_nth_mutation(Some(boundary));
        let interrupted = w.repour(&bd, &root);
        assert!(
            matches!(interrupted.outcome, BeadOutcome::Failed { .. }),
            "boundary {boundary}: {interrupted:#?}"
        );
        assert_eq!(w.leftovers(), Vec::<String>::new(), "boundary {boundary}");

        bd.fail_nth_mutation(None);
        let resumed = w.repour(&bd, &root);
        assert_eq!(
            resumed.outcome,
            BeadOutcome::Succeeded,
            "boundary {boundary}: {resumed:#?}"
        );
        assert_eq!(bd.snapshot(), expected, "boundary {boundary}");

        let settled = bd.snapshot();
        bd.fail_nth_mutation(None);
        let again = w.repour(&bd, &root);
        assert_eq!(again.outcome, BeadOutcome::Succeeded, "{again:#?}");
        assert_eq!(bd.snapshot(), settled, "boundary {boundary}");
        assert_eq!(bd.mutation_count(), 0, "boundary {boundary}: replay writes");
        assert_eq!(w.leftovers(), Vec::<String>::new(), "boundary {boundary}");
    }
}

fn assert_refused_without_writes(w: &mut Workspace, bd: &FakeBd, root: &str, code: &str) {
    let before = bd.snapshot();
    bd.fail_nth_mutation(None);
    let receipt = w.repour(bd, root);
    assert_eq!(
        receipt.outcome,
        BeadOutcome::Refused { code: code.into() },
        "{receipt:#?}"
    );
    assert_eq!(bd.mutation_count(), 0, "a refusal must not write");
    assert_eq!(bd.snapshot(), before);
    assert_eq!(w.leftovers(), Vec::<String>::new());
}

#[test]
fn a_dangling_reference_refuses_with_zero_writes() {
    let (mut w, bd, root) = poured(&plan_v1());
    bd.cook(&json!([
        {"id":"build","title":"Build changed"},
        {"id":"verify","title":"Verify","needs":["ghost"]}
    ]));
    let before = bd.snapshot();
    bd.fail_nth_mutation(None);
    let receipt = w.repour(&bd, &root);
    assert_eq!(
        receipt.outcome,
        BeadOutcome::Refused {
            code: "BEADS_GRAPH_FORMULA_UNSUPPORTED".into()
        },
        "{receipt:#?}"
    );
    assert_eq!(bd.mutation_count(), 0);
    assert_eq!(bd.snapshot(), before);
    assert_eq!(w.leftovers(), Vec::<String>::new());
}

#[test]
fn a_malformed_plan_refuses_with_zero_writes() {
    let (mut w, bd, root) = poured(&plan_v1());
    // A step without an id is not a plan.
    bd.cook(&json!([{"title":"Anonymous"}]));
    let before = bd.snapshot();
    bd.fail_nth_mutation(None);
    let receipt = w.repour(&bd, &root);
    assert_eq!(
        receipt.outcome,
        BeadOutcome::Failed {
            code: "BEADS_COOK_FAILED".into()
        },
        "{receipt:#?}"
    );
    assert_eq!(bd.mutation_count(), 0);
    assert_eq!(bd.snapshot(), before);
    assert_eq!(w.leftovers(), Vec::<String>::new());
}

#[test]
fn a_conflicting_edge_type_refuses_with_zero_writes() {
    let (mut w, bd, root) = poured(&plan_v1());
    let ids = bd.ids_by_step();
    {
        let mut store = bd.store.lock().unwrap();
        for dep in &mut store.deps {
            if dep.0 == ids["verify"] && dep.1 == ids["build"] {
                dep.2 = "related".into();
            }
        }
    }
    bd.cook(&plan_v2());
    assert_refused_without_writes(&mut w, &bd, &root, "BEADS_GRAPH_EDGE_CONFLICT");
}

#[test]
fn a_missing_or_foreign_target_refuses_before_any_write() {
    let (mut w, bd, root) = poured(&plan_v1());
    bd.cook(&plan_v2());
    assert_refused_without_writes(&mut w, &bd, "bd-999", "BEADS_GRAPH_PARENT_NOT_FOUND");

    // A step bead is not a pour root: it carries a step.
    let build = bd.ids_by_step()["build"].clone();
    assert_refused_without_writes(&mut w, &bd, &build, "BEADS_GRAPH_CONFLICT");

    // A bead with no provenance at all is not owned.
    bd.add_parent("plain-1");
    assert_refused_without_writes(&mut w, &bd, "plain-1", "BEADS_GRAPH_CONFLICT");

    // A root of a different formula.
    bd.edit_row(&root, |row| {
        row["metadata"]["sc_compose_graph"]["formula"] = json!("other");
    });
    assert_refused_without_writes(&mut w, &bd, &root, "BEADS_GRAPH_CONFLICT");
}

#[test]
fn a_dangling_relation_endpoint_refuses_with_its_typed_code_and_zero_writes() {
    let (mut w, bd, root) = poured(&plan_v1());
    bd.cook(&plan_v2());
    w.request.relations = vec![BeadRelation {
        from: BeadEndpoint::Step(StepId::new("build").unwrap()),
        to: BeadEndpoint::Bead(BeadId::new("bd-404").unwrap()),
        kind: BeadDependencyType::Blocks,
    }];
    assert_refused_without_writes(&mut w, &bd, &root, "BEADS_GRAPH_RELATION_INVALID");
}

#[test]
fn repour_requires_authorization_and_a_target() {
    let mut w = Workspace::new();
    let bd = FakeBd::new();
    bd.cook(&plan_v1());
    w.request.operation = BeadOperation::Repour;
    w.request.parent = None;
    let receipt = execute_bead_request_with_runner(&w.request, &bd);
    assert!(
        !matches!(&receipt, Ok(r) if r.outcome == BeadOutcome::Succeeded),
        "{receipt:?}"
    );
    assert_eq!(bd.mutation_count(), 0);
}

impl FakeBd {
    fn add_parent(&self, id: &str) {
        self.store.lock().unwrap().rows.insert(
            id.into(),
            json!({"id": id, "metadata": {}, "status": "open"}),
        );
    }

    fn mutation_log(&self) -> Vec<Vec<String>> {
        self.mutations.lock().unwrap().clone()
    }
}

impl Workspace {
    fn attach(&mut self, bd: &FakeBd, parent: &str) -> BeadComposeReceipt {
        self.request.operation = BeadOperation::Attach;
        self.request.parent = Some(BeadId::new(parent).unwrap());
        self.request.ref_ = Some(GraphRef::new("rel").unwrap());
        execute_bead_request_with_runner(&self.request, bd).unwrap()
    }
}

#[test]
fn repour_by_parent_and_ref_updates_creates_and_adds_edges_in_one_run() {
    let mut w = Workspace::new();
    let bd = FakeBd::new();
    bd.add_parent("proj-1");
    bd.cook(&plan_v1());
    let first = w.attach(&bd, "proj-1");
    assert_eq!(first.outcome, BeadOutcome::Succeeded, "{first:#?}");
    let (build, verify) = ("proj-1.rel-build", "proj-1.rel-verify");
    bd.set_runtime(build, "closed", "done", "alice");
    let beads = bd.snapshot().rows.len();

    // One run: a changed field, a new node and a new edge between existing beads.
    bd.cook(&json!([
        {"id":"build","title":"Build release","description":"compile"},
        {"id":"verify","title":"Verify","needs":["build"]},
        {"id":"ship","title":"Ship","needs":["verify","build"]}
    ]));
    w.request.operation = BeadOperation::Repour;
    bd.fail_nth_mutation(None);
    let receipt = execute_bead_request_with_runner(&w.request, &bd).unwrap();
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let store = bd.snapshot();
    assert_eq!(store.rows.len(), beads + 1);
    assert_eq!(bd.row(build)["title"], "Build release");
    assert_eq!(bd.row(build)["status"], "closed", "closed is not reopened");
    assert_eq!(bd.row(build)["assignee"], "alice");
    assert!(
        store
            .deps
            .contains(&dep("proj-1.rel-ship", verify, "blocks"))
    );
    assert!(
        store
            .deps
            .contains(&dep("proj-1.rel-ship", build, "blocks"))
    );

    // An edge between two existing beads is added with `dep add`.
    bd.cook(&json!([
        {"id":"build","title":"Build release","description":"compile"},
        {"id":"verify","title":"Verify","needs":["build"]},
        {"id":"ship","title":"Ship","needs":["verify","build"]},
        {"id":"docs","title":"Docs"}
    ]));
    let receipt = execute_bead_request_with_runner(&w.request, &bd).unwrap();
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    bd.cook(&json!([
        {"id":"build","title":"Build release","description":"compile"},
        {"id":"verify","title":"Verify","needs":["build"]},
        {"id":"ship","title":"Ship","needs":["verify","build","docs"]},
        {"id":"docs","title":"Docs"}
    ]));
    bd.fail_nth_mutation(None);
    let receipt = execute_bead_request_with_runner(&w.request, &bd).unwrap();
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    assert!(
        bd.mutation_log()
            .iter()
            .any(|call| call[..2] == ["dep", "add"])
    );
    assert!(
        bd.snapshot()
            .deps
            .contains(&dep("proj-1.rel-ship", "proj-1.rel-docs", "blocks"))
    );
    assert_eq!(w.leftovers(), Vec::<String>::new());

    // Replay writes nothing.
    bd.fail_nth_mutation(None);
    let before = bd.snapshot();
    let replay = execute_bead_request_with_runner(&w.request, &bd).unwrap();
    assert_eq!(replay.outcome, BeadOutcome::Succeeded, "{replay:#?}");
    assert_eq!(bd.mutation_count(), 0);
    assert_eq!(bd.snapshot(), before);
}

#[test]
fn attach_repour_interrupted_at_every_boundary_completes_on_rerun() {
    let edited = json!([
        {"id":"build","title":"Build release","description":"compile"},
        {"id":"verify","title":"Verify again","needs":["build"]},
        {"id":"ship","title":"Ship","needs":["verify","build"]}
    ]);
    let setup = || {
        let mut w = Workspace::new();
        let bd = FakeBd::new();
        bd.add_parent("proj-1");
        bd.cook(&plan_v1());
        assert_eq!(w.attach(&bd, "proj-1").outcome, BeadOutcome::Succeeded);
        bd.cook(&edited);
        w.request.operation = BeadOperation::Repour;
        (w, bd)
    };
    let (w, reference) = setup();
    reference.fail_nth_mutation(None);
    let done = execute_bead_request_with_runner(&w.request, &reference).unwrap();
    assert_eq!(done.outcome, BeadOutcome::Succeeded, "{done:#?}");
    let total = reference.mutation_count();
    let expected = reference.snapshot();
    assert!(total >= 3, "{total}");
    for boundary in 1..=total {
        let (w, bd) = setup();
        bd.fail_nth_mutation(Some(boundary));
        let cut = execute_bead_request_with_runner(&w.request, &bd).unwrap();
        assert!(
            matches!(cut.outcome, BeadOutcome::Failed { .. }),
            "boundary {boundary}: {cut:#?}"
        );
        assert_eq!(w.leftovers(), Vec::<String>::new());
        bd.fail_nth_mutation(None);
        let resumed = execute_bead_request_with_runner(&w.request, &bd).unwrap();
        assert_eq!(resumed.outcome, BeadOutcome::Succeeded, "{boundary}");
        assert_eq!(bd.snapshot(), expected, "boundary {boundary}");
        bd.fail_nth_mutation(None);
        execute_bead_request_with_runner(&w.request, &bd).unwrap();
        assert_eq!(bd.mutation_count(), 0, "boundary {boundary}: replay writes");
    }
}

#[test]
fn attach_repour_refuses_foreign_and_unowned_planned_ids_before_any_write() {
    let mut w = Workspace::new();
    let bd = FakeBd::new();
    bd.add_parent("proj-1");
    bd.cook(&plan_v1());
    assert_eq!(w.attach(&bd, "proj-1").outcome, BeadOutcome::Succeeded);
    bd.cook(&plan_v2());
    w.request.operation = BeadOperation::Repour;

    // Unowned: the planned id holds a bead with no provenance.
    let saved = bd.row("proj-1.rel-build");
    bd.store.lock().unwrap().rows.insert(
        "proj-1.rel-build".into(),
        json!({"id":"proj-1.rel-build","metadata":{}}),
    );
    bd.fail_nth_mutation(None);
    let before = bd.snapshot();
    let receipt = execute_bead_request_with_runner(&w.request, &bd).unwrap();
    assert!(
        matches!(&receipt.outcome, BeadOutcome::Refused { code } if code == "BEADS_GRAPH_CONFLICT"),
        "{receipt:#?}"
    );
    assert_eq!(bd.mutation_count(), 0);
    assert_eq!(bd.snapshot(), before);

    // Foreign: provenance of another attachment.
    let mut foreign = saved;
    foreign["metadata"]["sc_compose_graph"]["ref"] = json!("elsewhere");
    bd.store
        .lock()
        .unwrap()
        .rows
        .insert("proj-1.rel-build".into(), foreign);
    let receipt = execute_bead_request_with_runner(&w.request, &bd).unwrap();
    assert!(
        matches!(&receipt.outcome, BeadOutcome::Refused { code } if code == "BEADS_GRAPH_CONFLICT"),
        "{receipt:#?}"
    );
    assert_eq!(bd.mutation_count(), 0);
    assert_eq!(w.leftovers(), Vec::<String>::new());
}

#[test]
fn an_explicitly_empty_plan_field_clears_the_stored_value_and_closed_beads_update() {
    let (mut w, bd, root) = poured(&plan_v1());
    let build = bd.ids_by_step()["build"].clone();
    bd.set_runtime(&build, "closed", "shipped", "alice");
    bd.cook(&json!([
        {"id":"build","title":"Build","description":""},
        {"id":"verify","title":"Verify","needs":["build"]}
    ]));
    bd.fail_nth_mutation(None);
    let receipt = w.repour(&bd, &root);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    assert_eq!(bd.row(&build)["description"], "");
    assert_eq!(bd.row(&build)["status"], "closed");
    assert_eq!(bd.row(&build)["notes"], "shipped");
    let replay = w.repour(&bd, &root);
    assert_eq!(replay.outcome, BeadOutcome::Succeeded);
    assert_eq!(bd.mutation_count(), 2, "only the first run wrote");
}

#[test]
fn metadata_is_merged_and_provenance_is_written_after_the_fields() {
    let (mut w, bd, root) = poured(&plan_v1());
    let build = bd.ids_by_step()["build"].clone();
    bd.edit_row(&build, |row| row["metadata"]["user_key"] = json!("keep me"));
    bd.cook(&plan_v2());
    bd.fail_nth_mutation(None);
    let receipt = w.repour(&bd, &root);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let metadata = &bd.row(&build)["metadata"];
    assert_eq!(metadata["user_key"], "keep me");
    assert!(metadata["sc_compose_graph"]["node_definition"].is_string());

    let mine: Vec<Vec<String>> = bd
        .mutation_log()
        .into_iter()
        .filter(|call| call.last() == Some(&build))
        .collect();
    assert_eq!(mine.len(), 2, "{mine:?}");
    assert!(mine[0].iter().any(|arg| arg.starts_with("--title=")));
    assert!(!mine[0].iter().any(|arg| arg.contains("sc_compose_graph")));
    assert!(mine[1].iter().any(|arg| arg.contains("sc_compose_graph")));
    assert!(!mine[1].iter().any(|arg| arg.starts_with("--title=")));
}

#[test]
fn a_partial_failure_reports_applied_and_pending_work_with_a_rerun_hint() {
    let (mut w, bd, root) = poured(&plan_v1());
    bd.cook(&plan_v4());
    bd.fail_nth_mutation(Some(2));
    let receipt = w.repour(&bd, &root);
    assert_eq!(
        receipt.outcome,
        BeadOutcome::Failed {
            code: "BEADS_GRAPH_APPLY_FAILED".into()
        },
        "{receipt:#?}"
    );
    let evidence = &receipt.stages.last().unwrap().stderr_excerpt;
    assert!(evidence.contains("applied 1 of"), "{evidence}");
    assert!(evidence.contains("pending"), "{evidence}");
    assert!(evidence.contains("rerun"), "{evidence}");
}

#[test]
fn a_blocked_graph_plan_path_refuses_a_repour_before_any_bd_command() {
    let (mut w, bd, root) = poured(&plan_v1());
    bd.cook(&plan_v2());
    let plan = w.request.rendered_formula.with_extension("toml.graph.json");
    // The earlier pour left a plan file; a directory now blocks the path.
    let _ = fs::remove_file(&plan);
    fs::create_dir(&plan).unwrap();
    assert_refused_without_writes(&mut w, &bd, &root, "BEADS_OUTPUT_PATH_INVALID");
    assert!(plan.is_dir());
}

#[test]
fn repour_without_persistent_bead_authorization_is_refused_before_any_bd_command() {
    let (mut w, bd, root) = poured(&plan_v1());
    bd.cook(&plan_v2());
    let before = bd.snapshot();
    bd.fail_nth_mutation(None);
    w.request.operation = BeadOperation::Repour;
    w.request.parent = Some(BeadId::new(&root).unwrap());
    w.request.pour_authorization = None;
    let error = execute_bead_request_with_runner(&w.request, &bd).unwrap_err();
    assert!(
        matches!(error, BeadComposeError::PourAuthorizationRequired),
        "{error:?}"
    );
    assert_eq!(bd.mutation_count(), 0);
    assert_eq!(bd.snapshot(), before);
    assert_eq!(w.leftovers(), Vec::<String>::new());
}
