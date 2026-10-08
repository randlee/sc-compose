//! Regression tests promoted from the Phase T adversarial fuzz campaign.

use sc_composer_beads::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
const COOKED: &str =
    r#"{"formula":"sample","type":"workflow","steps":[{"id":"build","title":"Build"}]}"#;

struct Workspace {
    root: PathBuf,
    req: BeadComposeRequest,
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Workspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "sc-graph-fake-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("root");
        let root = fs::canonicalize(root).expect("canonical root");
        let template = root.join("sample.formula.toml.j2");
        fs::write(&template, "formula = \"sample\"\n").expect("template");
        Self {
            req: BeadComposeRequest {
                schema: BEADS_SCHEMA_V1.into(),
                operation: BeadOperation::PreviewAttach,
                working_directory: root.clone(),
                template,
                rendered_formula: root.join("sample.formula.toml"),
                compose_variables: serde_json::Map::new(),
                formula_name: Some(FormulaName::new("sample").expect("formula name")),
                bead_variables: BTreeMap::new(),
                bd_executable: Some(PathBuf::from("fake-bd")),
                pour_authorization: Some(PourAuthorization::CreatePersistentBeads),
                parent: Some(BeadId::new("proj-1").expect("parent")),
                ref_: Some(GraphRef::new("chain").expect("ref")),
                relations: Vec::new(),
            },
            root,
        }
    }

    fn run(&self, runner: &FakeRunner) -> BeadComposeReceipt {
        execute_bead_request_with_runner(&self.req, runner).expect("request")
    }

    fn plan(&self) -> Value {
        serde_json::from_slice(
            &fs::read(self.req.rendered_formula.with_extension("toml.graph.json"))
                .expect("plan file"),
        )
        .expect("plan JSON")
    }
}

struct FakeRunner {
    outputs: Mutex<VecDeque<io::Result<ProcessOutput>>>,
    calls: Mutex<Vec<CommandSpec>>,
}

impl FakeRunner {
    fn new(outputs: impl IntoIterator<Item = ProcessOutput>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into_iter().map(Ok).collect()),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<CommandSpec> {
        self.calls.lock().expect("calls").clone()
    }
}

impl ProcessRunner for FakeRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        self.calls.lock().expect("calls").push(spec.clone());
        self.outputs
            .lock()
            .expect("outputs")
            .pop_front()
            .expect("unexpected bd call")
    }
}

fn out(status: Option<i32>, stdout: &str) -> ProcessOutput {
    ProcessOutput {
        exit_status: status,
        stdout: stdout.into(),
        stderr: String::new(),
        elapsed: Duration::ZERO,
    }
}

fn ok(stdout: &str) -> ProcessOutput {
    out(Some(0), stdout)
}

fn parent() -> ProcessOutput {
    ok(r#"[{"id":"proj-1"}]"#)
}

fn failed(receipt: &BeadComposeReceipt, code: &str, stage: BeadStage) {
    assert_eq!(
        receipt.outcome,
        BeadOutcome::Failed { code: code.into() },
        "{receipt:#?}"
    );
    assert_eq!(receipt.stages.last().expect("stage").stage, stage);
}

/// Rewrites the on-disk formula just before `bd cook` would read it.
struct RewritingRunner {
    inner: FakeRunner,
    rendered_formula: PathBuf,
}

impl ProcessRunner for RewritingRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        if spec.args[0] != "cook" {
            return self.inner.run(spec);
        }
        self.inner.calls.lock().expect("calls").push(spec.clone());
        fs::write(&self.rendered_formula, "formula = \"other\"\n").expect("rewrite");
        let parsed = fs::read_to_string(&spec.args[1]).expect("cooked path");
        Ok(ok(if parsed.contains("other") {
            r#"{"formula":"sample","type":"workflow","steps":[{"id":"other","title":"Other"}]}"#
        } else {
            COOKED
        }))
    }
}

// FUZZ-013: Phase R requests parse unchanged (ADR-0023 Decision 1).
#[test]
fn fuzz_013_phase_r_render_request_keeps_accepting_its_formula_name() {
    for name in ["café", "re g0"] {
        let request = json!({
            "schema": BEADS_SCHEMA_V1,
            "operation": "render",
            "working_directory": "/work",
            "template": "f.formula.toml.j2",
            "rendered_formula": "/work/build/f.formula.toml",
            "formula_name": name,
            "compose_variables": {},
            "bead_variables": {}
        });
        let parsed = parse_request(&request.to_string());
        assert!(parsed.is_ok(), "{name}: {:?}", parsed.err());
    }
}

// FUZZ-014: graph planning uses this request's rendered text.
#[test]
fn fuzz_014_graph_is_built_from_this_requests_rendered_text() {
    let w = Workspace::new();
    let clean = w.run(&FakeRunner::new([ok(COOKED), parent(), ok("{}")]));
    assert_eq!(clean.outcome, BeadOutcome::Succeeded, "{clean:#?}");
    let raced = execute_bead_request_with_runner(
        &w.req,
        &RewritingRunner {
            inner: FakeRunner::new([parent(), ok("{}")]),
            rendered_formula: w.req.rendered_formula.clone(),
        },
    )
    .expect("request");
    let ids = |receipt: &BeadComposeReceipt| receipt.graph.as_ref().map(|graph| graph.ids.clone());
    assert_eq!(ids(&raced), ids(&clean), "{raced:#?}");
}

/// Models another request replacing the legacy shared plan before bd opens its argv path.
/// Overwrite only the legacy path so a future per-request path can pass this regression.
struct PlanRewritingRunner {
    inner: FakeRunner,
    shared_plan: PathBuf,
    plans: Mutex<Option<(Value, Value)>>,
}

impl ProcessRunner for PlanRewritingRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        if spec.args.first().is_some_and(|arg| arg == "create") {
            let graph_index = spec
                .args
                .iter()
                .position(|arg| arg == "--graph")
                .expect("graph argument");
            let actual_path = &spec.args[graph_index + 1];
            let expected: Value =
                serde_json::from_slice(&fs::read(actual_path).expect("original plan"))
                    .expect("plan JSON");
            let mut competing = expected.clone();
            competing["nodes"][0]["id"] = json!("proj-1.other-build");
            fs::write(
                &self.shared_plan,
                serde_json::to_vec(&competing).expect("competing JSON"),
            )
            .expect("overwrite shared plan");
            let consumed =
                serde_json::from_slice(&fs::read(actual_path).expect("bd reads argv path"))
                    .expect("consumed plan JSON");
            *self.plans.lock().expect("plans") = Some((expected, consumed));
        }
        self.inner.run(spec)
    }
}

// FUZZ-014: bd must consume this request's graph plan despite another request's overwrite.
#[test]
fn fuzz_014_graph_create_reads_this_requests_plan() {
    let w = Workspace::new();
    let runner = PlanRewritingRunner {
        inner: FakeRunner::new([ok(COOKED), parent(), ok("{}")]),
        shared_plan: w.req.rendered_formula.with_extension("toml.graph.json"),
        plans: Mutex::new(None),
    };
    let receipt =
        execute_bead_request_with_runner(&w.req, &runner).expect("request must yield receipt");
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let plans = runner.plans.lock().expect("plans");
    let (expected, consumed) = plans.as_ref().expect("bd create must read a graph plan");
    assert_eq!(expected["nodes"][0]["id"], "proj-1.chain-build");
    let graph = receipt.graph.expect("graph receipt");
    assert_eq!(
        graph.ids[&StepId::new("build").expect("step")].as_str(),
        "proj-1.chain-build"
    );
    assert_eq!(
        consumed, expected,
        "bd consumed another request's replacement at the shared plan path"
    );
}

// FUZZ-015: a 40-step attach re-run is not capped by existing-bead output.
#[cfg(unix)]
#[test]
fn fuzz_015_rerun_of_a_40_step_attach_is_not_capped_by_the_output_limit() {
    scalable_attach_roundtrip(40, false);
}

#[cfg(unix)]
#[test]
fn fuzz_015_first_attach_and_rerun_of_500_steps_have_bounded_graph_output() {
    scalable_attach_roundtrip(500, true);
}

#[cfg(unix)]
fn scalable_attach_roundtrip(count: usize, apply: bool) {
    use std::os::unix::fs::PermissionsExt;

    let mut w = Workspace::new();
    if apply {
        w.req.operation = BeadOperation::Attach;
    }
    let steps: Vec<Value> = (1..=count)
        .map(|index| json!({"id": format!("item_{index}"), "title": format!("Item {index}"), "description": "x".repeat(1900)}))
        .collect();
    let cooked = w.root.join("cooked.json");
    let shown = w.root.join("show.json");
    let created = w.root.join("created.json");
    let ids: serde_json::Map<String, Value> = (1..=count)
        .map(|index| {
            (
                format!("item_{index}"),
                json!(format!("proj-1.chain-item_{index}")),
            )
        })
        .collect();
    fs::write(
        &created,
        json!({"ids": ids, "diagnostic": "x".repeat(70_000)}).to_string(),
    )
    .unwrap();
    fs::write(
        &cooked,
        json!({"formula": "sample", "type": "workflow", "steps": steps}).to_string(),
    )
    .expect("cooked");
    fs::write(&shown, r#"[{"id":"proj-1"}]"#).expect("show");
    let bd = w.root.join("fake-bd");
    fs::write(
        &bd,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n  cook) cat '{}' ;;\n  show) cat '{}' ;;\n  dep) printf '%s' '[{{\"id\":\"proj-1\",\"dependency_type\":\"parent-child\"}}]' ;;\n  create) cat '{}' ;;\n  *) printf '{{}}' ;;\nesac\n",
            cooked.display(),
            shown.display(),
            created.display()
        ),
    )
    .expect("fake bd");
    fs::set_permissions(&bd, fs::Permissions::from_mode(0o755)).expect("chmod");
    w.req.bd_executable = Some(bd);
    let first = execute_bead_request(&w.req).expect("first preview");
    assert_eq!(first.outcome, BeadOutcome::Succeeded, "{first:#?}");
    assert_eq!(first.graph.as_ref().unwrap().nodes.len(), count);
    if apply {
        assert!(
            first
                .graph
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .all(|node| node.action == BeadNodeAction::Created)
        );
        assert!(fs::metadata(&cooked).unwrap().len() > 64 * 1024);
        assert!(fs::metadata(&created).unwrap().len() > 64 * 1024);
    }
    let mut rows = vec![json!({"id": "proj-1"})];
    for node in w.plan()["nodes"].as_array().expect("plan nodes") {
        rows.push(json!({
            "id": node["id"],
            "title": node["title"],
            "description": "x".repeat(1900),
            "metadata": node["metadata"],
        }));
    }
    fs::write(&shown, Value::Array(rows).to_string()).expect("existing rows");
    let shown_bytes = fs::metadata(&shown).unwrap().len();
    assert!(shown_bytes > 64 * 1024);
    assert!(shown_bytes < sc_composer_beads::runner::GRAPH_OUTPUT_LIMIT_BYTES as u64);
    let rerun = execute_bead_request(&w.req).expect("re-run must yield a receipt");
    assert_eq!(rerun.outcome, BeadOutcome::Succeeded, "{rerun:#?}");
    let graph = rerun.graph.expect("graph");
    assert_eq!(graph.nodes.len(), count);
    assert!(
        graph
            .nodes
            .iter()
            .all(|node| node.action == BeadNodeAction::Existing)
    );
}

const OPTION_LIKE_IDS: [&str; 3] = ["--db=/elsewhere", "--json", "-q"];

fn option_id_argument_errors(calls: &[CommandSpec], ids: &[&str]) -> Vec<String> {
    let mut checked = 0;
    let mut errors = Vec::new();
    for call in calls {
        for id in ids {
            if call.args.iter().any(|arg| arg == id) {
                checked += 1;
                // --json can also be a real option before the separator;
                // require its distinct positional occurrence after the separator.
                let safely_positional =
                    call.args
                        .iter()
                        .position(|arg| arg == "--")
                        .is_some_and(|separator| {
                            call.args[separator + 1..].iter().any(|arg| arg == id)
                        });
                if !safely_positional {
                    errors.push(format!(
                        "option-like ID {id:?} must follow -- in {:?}",
                        call.args
                    ));
                }
            }
        }
    }
    assert!(checked > 0, "no option-like ID reached bd: {calls:?}");
    errors
}

// FUZZ-016: option-like parent IDs reach bd show as IDs, not options.
#[test]
#[ignore = "FUZZ-016"]
fn fuzz_016_option_like_bead_ids_are_never_parsed_as_bd_options() {
    let mut argument_errors = Vec::new();
    for id in OPTION_LIKE_IDS {
        let mut w = Workspace::new();
        w.req.parent = Some(BeadId::new(id).expect("valid bead id"));
        let runner = FakeRunner::new([
            ok(COOKED),
            out(
                Some(1),
                r#"{"error":"no issues found matching the provided IDs"}"#,
            ),
        ]);
        let receipt = execute_bead_request_with_runner(&w.req, &runner)
            .expect("option-like parent must yield a receipt");
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Refused {
                code: "BEADS_GRAPH_PARENT_NOT_FOUND".into()
            },
            "{id}: {receipt:#?}"
        );
        let calls = runner.calls();
        assert_eq!(calls.len(), 2, "parent refusal must stop after cook/show");
        assert_eq!(calls[1].args[0], "show");
        let child = format!("{id}.chain-build");
        argument_errors.extend(option_id_argument_errors(&calls, &[id, &child]));
    }
    assert!(argument_errors.is_empty(), "{}", argument_errors.join("\n"));
}

// FUZZ-016: existing option-like relation sources reach both show and dep list safely.
#[test]
#[ignore = "FUZZ-016"]
fn fuzz_016_option_like_relation_sources_are_not_dep_list_options() {
    let mut argument_errors = Vec::new();
    for id in OPTION_LIKE_IDS {
        let mut w = Workspace::new();
        w.req.relations.push(BeadRelation {
            from: BeadEndpoint::Bead(BeadId::new(id).expect("source")),
            to: BeadEndpoint::Step(StepId::new("build").expect("step")),
            kind: BeadDependencyType::Blocks,
        });
        let runner = FakeRunner::new([
            ok(COOKED),
            ok(&json!([{"id":"proj-1"}, {"id":id}]).to_string()),
            ok("[]"),
            ok("{}"),
        ]);
        let receipt = execute_bead_request_with_runner(&w.req, &runner)
            .expect("option-like source must yield a receipt");
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Succeeded,
            "{id}: {receipt:#?}"
        );
        let calls = runner.calls();
        assert_eq!(calls.len(), 4, "cook/show/dep list/create are required");
        assert_eq!(calls[1].args[0], "show");
        assert_eq!(&calls[2].args[..2], ["dep", "list"]);
        assert!(
            calls[2].args.iter().any(|arg| arg == id),
            "dep list must inspect the option-like source"
        );
        argument_errors.extend(option_id_argument_errors(&calls[2..3], &[id]));
        argument_errors.extend(option_id_argument_errors(&calls[1..2], &[id]));
    }
    assert!(argument_errors.is_empty(), "{}", argument_errors.join("\n"));
}

// FUZZ-020: apply failures preserve bd's error text.
#[test]
fn fuzz_020_apply_failure_cause_is_bds_error_text() {
    let w = Workspace::new();
    let runner = FakeRunner::new([
        ok(COOKED),
        parent(),
        out(
            Some(1),
            "{\n  \"error\": \"graph contains a blocking dependency cycle involving node \\\"build\\\"\"\n}",
        ),
    ]);
    let receipt = w.run(&runner);
    failed(
        &receipt,
        "BEADS_GRAPH_APPLY_FAILED",
        BeadStage::PreviewAttach,
    );
    let evidence = &receipt.stages.last().expect("stage").stderr_excerpt;
    assert!(
        evidence.contains("graph apply failed")
            && evidence
                .contains("graph contains a blocking dependency cycle involving node \"build\""),
        "{evidence}"
    );
}

#[cfg(unix)]
#[test]
fn graph_capture_overflow_reports_the_actual_stream_limit() {
    use sc_composer_beads::runner::{GRAPH_OUTPUT_LIMIT_BYTES, PROCESS_OUTPUT_LIMIT_BYTES};
    use std::os::unix::fs::PermissionsExt;
    for (limit, redirect) in [
        (GRAPH_OUTPUT_LIMIT_BYTES, ""),
        (PROCESS_OUTPUT_LIMIT_BYTES, " >&2"),
    ] {
        let mut w = Workspace::new();
        let output = w.root.join("oversized-output");
        fs::write(&output, vec![b'x'; limit + 1]).unwrap();
        let bd = w.root.join("fake-bd");
        fs::write(
            &bd,
            format!(
                "#!/bin/sh\ncat '{}'{redirect}\nsleep 60\n",
                output.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&bd, fs::Permissions::from_mode(0o755)).unwrap();
        w.req.bd_executable = Some(bd);
        let error = execute_bead_request(&w.req).unwrap_err();
        assert_eq!(error.code(), "BEADS_PROCESS_OUTPUT_LIMIT");
        assert!(
            matches!(error, BeadComposeError::ProcessOutputLimitExceeded { stage: BeadStage::Validate, limit_bytes } if limit_bytes == limit)
        );
        assert!(
            !w.req
                .rendered_formula
                .with_extension("toml.graph.json")
                .exists()
        );
    }
}

// FUZZ-021: semantic graph ids retain typed errors across request parsing.
#[test]
fn fuzz_021_invalid_refs_and_relation_steps_keep_typed_errors() {
    for operation in ["attach", "preview_attach"] {
        for reference in ["a.b", "", &"x".repeat(33)] {
            let request = json!({
                "schema": BEADS_SCHEMA_V1, "operation": operation,
                "working_directory": "/work", "template": "sample.formula.toml.j2",
                "rendered_formula": "/work/sample.formula.toml", "compose_variables": {},
                "bead_variables": {}, "parent": "proj-1", "ref": reference
            });
            let error = parse_request(&request.to_string()).expect_err("invalid ref");
            assert_eq!(error.code(), "BEADS_GRAPH_ID_INVALID");
            assert!(
                matches!(&error, BeadComposeError::GraphIdInvalid { field: GraphIdField::Ref, value } if value == reference)
            );
            let wire = serde_json::to_value(&error).unwrap();
            assert_eq!(wire["details"], json!({"field":"ref", "value":reference}));
            assert!(error.to_string().contains("[A-Za-z0-9_-]"));
            for endpoint_field in ["from", "to"] {
                let mut relation_request = request.clone();
                relation_request["ref"] = json!("valid");
                let mut relation =
                    json!({"from":"step:build", "to":"bead:proj-1", "type":"blocks"});
                relation[endpoint_field] = json!("step:bad.step");
                relation_request["relations"] = json!([relation]);
                let error = parse_request(&relation_request.to_string()).expect_err("invalid step");
                assert!(
                    matches!(error, BeadComposeError::GraphIdInvalid { field: GraphIdField::Step, value } if value == "bad.step")
                );
            }
        }
    }
}

/// Coordinates both real threads at each bd read, with bounded waits on failure.
struct ConcurrentRunner {
    peer: std::sync::mpsc::Sender<&'static str>,
    ready: Mutex<std::sync::mpsc::Receiver<&'static str>>,
    formula: Mutex<Option<Value>>,
    plan: Mutex<Option<Value>>,
}

impl ConcurrentRunner {
    fn rendezvous(&self, stage: &'static str) {
        self.peer.send(stage).expect("peer alive");
        assert_eq!(
            self.ready
                .lock()
                .expect("ready")
                .recv_timeout(Duration::from_secs(10))
                .expect("peer reached read"),
            stage
        );
    }
}

impl ProcessRunner for ConcurrentRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        match spec.args[0].as_str() {
            "cook" => {
                self.rendezvous("cook");
                let path = std::path::Path::new(&spec.args[1]);
                assert!(
                    path.to_string_lossy().ends_with(".formula.toml")
                        || path.to_string_lossy().ends_with(".formula.json"),
                    "bd must recognize the formula format"
                );
                let text = fs::read_to_string(path).expect("own formula input");
                let formula: Value = if path.extension().is_some_and(|ext| ext == "json") {
                    serde_json::from_str(&text).expect("JSON formula")
                } else {
                    serde_json::to_value(
                        toml::from_str::<toml::Value>(&text).expect("TOML formula"),
                    )
                    .expect("formula JSON")
                };
                *self.formula.lock().expect("formula") = Some(formula.clone());
                Ok(ok(&formula.to_string()))
            }
            "show" => Ok(parent()),
            "create" => {
                self.rendezvous("create");
                let plan: Value =
                    serde_json::from_slice(&fs::read(&spec.args[2]).expect("own graph input"))
                        .expect("graph JSON");
                let ids: serde_json::Map<String, Value> = plan["nodes"]
                    .as_array()
                    .expect("nodes")
                    .iter()
                    .map(|node| {
                        (
                            node["key"].as_str().expect("key").into(),
                            node["id"].clone(),
                        )
                    })
                    .collect();
                *self.plan.lock().expect("plan") = Some(plan);
                Ok(ok(&json!({"ids":ids}).to_string()))
            }
            other => panic!("unexpected bd command: {other}"),
        }
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "Keep the coordinated concurrent requests and their complete payload/receipt oracles together."
)]
fn fuzz_014_concurrent_attaches_keep_complete_inputs_and_receipts() {
    for suffix in ["toml", "json"] {
        let mut w = Workspace::new();
        w.req.operation = BeadOperation::Attach;
        w.req.template = w.root.join(format!("sample.formula.{suffix}.j2"));
        w.req.rendered_formula = w.root.join(format!("sample.formula.{suffix}"));
        fs::write(&w.req.template, if suffix == "toml" {
            "formula = \"sample\"\ntype = \"workflow\"\n[[steps]]\nid = \"{{{ step }}}\"\ntitle = \"{{{ title }}}\"\ndescription = \"{{{ description }}}\"\n"
        } else {
            r#"{"formula":"sample","type":"workflow","steps":[{"id":"{{{ step }}}","title":"{{{ title }}}","description":"{{{ description }}}"}]}"#
        }).expect("template");
        let (tx_a, rx_a) = std::sync::mpsc::channel();
        let (tx_b, rx_b) = std::sync::mpsc::channel();
        let runners = [
            ConcurrentRunner {
                peer: tx_b,
                ready: Mutex::new(rx_a),
                formula: Mutex::new(None),
                plan: Mutex::new(None),
            },
            ConcurrentRunner {
                peer: tx_a,
                ready: Mutex::new(rx_b),
                formula: Mutex::new(None),
                plan: Mutex::new(None),
            },
        ];
        let requests: Vec<_> = ["alpha", "beta"]
            .into_iter()
            .map(|step| {
                let mut request = w.req.clone();
                request.ref_ = Some(GraphRef::new(step).expect("ref"));
                request.compose_variables = serde_json::Map::from_iter([
                    ("step".into(), json!(step)),
                    ("title".into(), json!(format!("{step} – own title"))),
                    (
                        "description".into(),
                        json!(format!("{step} {}", step.repeat(100))),
                    ),
                ]);
                request
            })
            .collect();
        let receipts = std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                execute_bead_request_with_runner(&requests[0], &runners[0]).expect("first request")
            });
            let b = scope.spawn(|| {
                execute_bead_request_with_runner(&requests[1], &runners[1]).expect("second request")
            });
            [
                a.join().expect("first thread"),
                b.join().expect("second thread"),
            ]
        });
        for ((request, runner), receipt) in requests.iter().zip(&runners).zip(&receipts) {
            assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
            let step = request.compose_variables["step"].as_str().expect("step");
            let expected_formula = json!({"formula":"sample","type":"workflow","steps":[{
                "id":step,"title":request.compose_variables["title"],"description":request.compose_variables["description"]
            }]});
            assert_eq!(
                runner
                    .formula
                    .lock()
                    .expect("formula")
                    .as_ref()
                    .expect("consumed formula"),
                &expected_formula
            );
            let plan = runner.plan.lock().expect("plan");
            let plan = plan.as_ref().expect("consumed plan");
            let graph = receipt.graph.as_ref().expect("graph");
            let id = format!("proj-1.{step}-{step}");
            assert_eq!(graph.ids[&StepId::new(step).expect("step")].as_str(), id);
            assert_eq!(graph.ref_.as_ref().expect("ref").as_str(), step);
            assert_eq!(graph.nodes[0].action, BeadNodeAction::Created);
            assert_eq!(graph.nodes[0].id.as_ref().expect("node id").as_str(), id);
            let provenance = &plan["nodes"][0]["metadata"][PROVENANCE_KEY];
            assert_eq!(provenance["ref"], step);
            assert_eq!(provenance["step"], step);
            assert_eq!(provenance["revision"], json!(graph.revision));
            assert_eq!(
                plan,
                &json!({"edges":[],"nodes":[{
                    "key":step,"id":id,"parent_id":"proj-1", "title":request.compose_variables["title"],
                    "description":request.compose_variables["description"],"metadata":{PROVENANCE_KEY:provenance}
                }]})
            );
            assert_eq!(receipt.rendered_formula, w.req.rendered_formula);
            assert_eq!(
                graph.plan_path.as_ref().expect("public plan"),
                &w.req
                    .rendered_formula
                    .with_extension(format!("{suffix}.graph.json"))
            );
        }
        let public_formula =
            fs::read_to_string(&w.req.rendered_formula).expect("published formula");
        let public_formula: Value = if suffix == "json" {
            serde_json::from_str(&public_formula).unwrap()
        } else {
            serde_json::to_value(toml::from_str::<toml::Value>(&public_formula).unwrap()).unwrap()
        };
        assert!(
            runners
                .iter()
                .any(|runner| runner.formula.lock().unwrap().as_ref() == Some(&public_formula)),
            "public formula must be one complete request"
        );
        let public_plan: Value = serde_json::from_slice(
            &fs::read(
                w.req
                    .rendered_formula
                    .with_extension(format!("{suffix}.graph.json")),
            )
            .expect("published plan"),
        )
        .expect("plan JSON");
        assert!(
            runners
                .iter()
                .any(|runner| runner.plan.lock().unwrap().as_ref() == Some(&public_plan)),
            "public plan must be one complete request"
        );
        assert!(
            !fs::read_dir(&w.root).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".sc-compose")),
            "private inputs must be cleaned up"
        );
    }
}

#[test]
fn graph_private_inputs_are_cleaned_after_failed_reads_and_apply() {
    for failure in ["cook", "apply", "spawn"] {
        let w = Workspace::new();
        let runner = if failure == "cook" {
            FakeRunner::new([out(Some(1), "invalid formula")])
        } else {
            FakeRunner::new([ok(COOKED), parent(), out(Some(1), "apply refused")])
        };
        if failure == "spawn" {
            *runner
                .outputs
                .lock()
                .expect("outputs")
                .back_mut()
                .expect("apply") = Err(io::Error::new(io::ErrorKind::NotFound, "bd unavailable"));
        }
        let result = execute_bead_request_with_runner(&w.req, &runner);
        if failure == "spawn" {
            assert!(matches!(
                result,
                Err(BeadComposeError::BdUnavailable { .. })
            ));
        } else {
            let receipt = result.expect("failure receipt");
            assert!(
                matches!(receipt.outcome, BeadOutcome::Failed { .. }),
                "{receipt:#?}"
            );
        }
        assert_eq!(
            fs::read_to_string(&w.req.rendered_formula).expect("published formula"),
            "formula = \"sample\""
        );
        assert!(
            !fs::read_dir(&w.root).expect("directory").any(|entry| entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".sc-compose")),
            "private input leaked after {failure}"
        );
    }
}

#[test]
fn invalid_parent_and_relation_bead_ids_keep_native_typed_errors() {
    for operation in [
        "render",
        "validate",
        "attach",
        "preview_attach",
        "pour",
        "preview_pour",
    ] {
        for invalid in [
            "",
            " ",
            "invalid parent",
            "bad\tparent",
            "bad\nparent",
            "bad\rparent",
        ] {
            let mut request = json!({
                "schema": BEADS_SCHEMA_V1, "operation": operation,
                "working_directory": "/work", "template": "sample.formula.toml.j2",
                "rendered_formula": "/work/sample.formula.toml", "compose_variables": {},
                "bead_variables": {}, "parent": invalid, "ref": "valid"
            });
            let error = parse_request(&request.to_string()).expect_err("invalid parent");
            assert_native_bead_id_error(&error, invalid);
            request["parent"] = json!("proj-1");
            for field in ["from", "to"] {
                let mut relation =
                    json!({"from":"step:build", "to":"bead:proj-1", "type":"blocks"});
                relation[field] = json!(format!("bead:{invalid}"));
                request["relations"] = json!([relation]);
                let error = parse_request(&request.to_string()).expect_err("invalid relation bead");
                assert_native_bead_id_error(&error, invalid);
            }
        }
    }
}

fn assert_native_bead_id_error(error: &BeadComposeError, invalid: &str) {
    assert_eq!(error.code(), "BEADS_GRAPH_ID_INVALID");
    assert!(
        matches!(error, BeadComposeError::GraphIdInvalid { field: GraphIdField::Bead, value } if value == invalid)
    );
    let wire = serde_json::to_value(error).unwrap();
    assert_eq!(wire["details"], json!({"field":"bead", "value":invalid}));
    assert!(
        wire["message"]
            .as_str()
            .unwrap()
            .contains("bead ids are non-empty without whitespace")
    );
}

struct ByPathConcurrentRunner {
    peer: std::sync::mpsc::Sender<&'static str>,
    ready: Mutex<std::sync::mpsc::Receiver<&'static str>>,
    registry: PathBuf,
    label: &'static str,
    cooked: Mutex<Vec<Value>>,
    calls: Mutex<Vec<CommandSpec>>,
}

impl ProcessRunner for ByPathConcurrentRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        self.calls.lock().expect("calls").push(spec.clone());
        match spec.args[0].as_str() {
            "cook" => {
                self.peer.send("cook").expect("peer alive");
                assert_eq!(
                    self.ready
                        .lock()
                        .expect("ready")
                        .recv_timeout(Duration::from_secs(10))
                        .expect("peer cook"),
                    "cook"
                );
                let path = std::path::Path::new(&spec.args[1]);
                let text = fs::read_to_string(path).expect("cook input");
                let formula: Value = if path.extension().is_some_and(|ext| ext == "json") {
                    serde_json::from_str(&text).expect("JSON formula")
                } else {
                    serde_json::to_value(
                        toml::from_str::<toml::Value>(&text).expect("TOML formula"),
                    )
                    .expect("formula JSON")
                };
                self.cooked.lock().expect("cooked").push(formula.clone());
                Ok(ok(&formula.to_string()))
            }
            "where" => Ok(ok(&json!({"path":self.registry}).to_string())),
            "create" => {
                let plan: Value =
                    serde_json::from_slice(&fs::read(&spec.args[2]).expect("graph input"))
                        .expect("plan");
                let ids: serde_json::Map<String, Value> = plan["nodes"]
                    .as_array()
                    .expect("nodes")
                    .iter()
                    .map(|node| {
                        let key = node["key"].as_str().expect("key");
                        let id = if key == "_root" {
                            format!("root-{}", self.label)
                        } else {
                            format!("root-{}.{}", self.label, self.label)
                        };
                        (key.into(), json!(id))
                    })
                    .collect();
                Ok(ok(&json!({"ids":ids}).to_string()))
            }
            other => panic!("unexpected by-path command {other}"),
        }
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "Keep both cook-input and receipt-stage oracles beside the concurrent requests for both pour modes and formats."
)]
fn fuzz_014b_concurrent_bypath_pours_validate_their_own_initial_cook_inputs() {
    for operation in [BeadOperation::PreviewPour, BeadOperation::Pour] {
        for suffix in ["toml", "json"] {
            let mut w = Workspace::new();
            w.req.operation = operation;
            w.req.parent = None;
            w.req.ref_ = None;
            w.req.template = w.root.join(format!("sample.formula.{suffix}.j2"));
            w.req.rendered_formula = w.root.join(format!("sample.formula.{suffix}"));
            fs::write(&w.req.template, if suffix == "toml" {
                "formula = \"sample\"\ntype = \"workflow\"\n[[steps]]\nid = \"{{{ step }}}\"\ntitle = \"{{{ title }}}\"\n"
            } else {
                r#"{"formula":"sample","type":"workflow","steps":[{"id":"{{{ step }}}","title":"{{{ title }}}"}]}"#
            }).expect("template");
            let registry = w.root.join(".beads");
            fs::create_dir(&registry).expect("registry");
            let (tx_a, rx_a) = std::sync::mpsc::channel();
            let (tx_b, rx_b) = std::sync::mpsc::channel();
            let runners = [
                ByPathConcurrentRunner {
                    peer: tx_b,
                    ready: Mutex::new(rx_a),
                    registry: registry.clone(),
                    label: "alpha",
                    cooked: Mutex::new(Vec::new()),
                    calls: Mutex::new(Vec::new()),
                },
                ByPathConcurrentRunner {
                    peer: tx_a,
                    ready: Mutex::new(rx_b),
                    registry,
                    label: "beta",
                    cooked: Mutex::new(Vec::new()),
                    calls: Mutex::new(Vec::new()),
                },
            ];
            let requests: Vec<_> = ["alpha", "beta"]
                .into_iter()
                .map(|step| {
                    let mut request = w.req.clone();
                    request.compose_variables = serde_json::Map::from_iter([
                        ("step".into(), json!(step)),
                        ("title".into(), json!(format!("{step} own title"))),
                    ]);
                    request
                })
                .collect();
            let receipts = std::thread::scope(|scope| {
                let a = scope.spawn(|| {
                    execute_bead_request_with_runner(&requests[0], &runners[0])
                        .expect("first request")
                });
                let b = scope.spawn(|| {
                    execute_bead_request_with_runner(&requests[1], &runners[1])
                        .expect("second request")
                });
                [
                    a.join().expect("first thread"),
                    b.join().expect("second thread"),
                ]
            });
            for (runner, receipt) in runners.iter().zip(&receipts) {
                assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
                let expected = json!({"formula":"sample","type":"workflow","steps":[{"id":runner.label,"title":format!("{} own title",runner.label)}]});
                let cooked = runner.cooked.lock().expect("cooked");
                assert_eq!(cooked.len(), 2, "both initial and graph cooks must run");
                assert_eq!(
                    cooked[0], expected,
                    "initial Phase R cook must validate this request's complete content"
                );
                assert_eq!(cooked[1], expected, "graph cook must use the same content");
                let calls = runner.calls.lock().expect("calls");
                assert_eq!(
                    calls
                        .iter()
                        .map(|call| call.args[0].as_str())
                        .collect::<Vec<_>>(),
                    ["cook", "where", "cook", "create"]
                );
                assert_eq!(&calls[0].args[2..], ["--dry-run", "--json"]);
                assert_eq!(calls[1].args, ["where", "--json"]);
                assert_eq!(&calls[2].args[2..], ["--json"]);
                assert_eq!(
                    calls[0].args[1], calls[2].args[1],
                    "both cooks must use the same private snapshot"
                );
                assert_ne!(calls[0].args[1], w.req.rendered_formula.to_string_lossy());
                let graph_stage = if operation == BeadOperation::Pour {
                    BeadStage::Pour
                } else {
                    BeadStage::PreviewPour
                };
                assert_eq!(
                    receipt
                        .stages
                        .iter()
                        .map(|stage| stage.stage)
                        .collect::<Vec<_>>(),
                    [
                        BeadStage::Render,
                        BeadStage::Validate,
                        BeadStage::ResolveActiveRegistry,
                        graph_stage,
                        graph_stage
                    ]
                );
                assert_eq!(receipt.pour_mode, Some(BeadPourMode::Graph));
                assert_eq!(receipt.rendered_formula, w.req.rendered_formula);
                let graph = receipt.graph.as_ref().expect("graph");
                assert_eq!(graph.nodes.len(), 2);
                assert_eq!(
                    graph.nodes[1].step.as_ref().expect("step").as_str(),
                    runner.label
                );
                if operation == BeadOperation::Pour {
                    assert_eq!(
                        graph.ids[&StepId::new(runner.label).expect("step")].as_str(),
                        format!("root-{}.{}", runner.label, runner.label)
                    );
                }
            }
            assert!(!fs::read_dir(&w.root).expect("directory").any(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sc-compose")
            }));
        }
    }
}
