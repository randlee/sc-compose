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
#[ignore = "FUZZ-013"]
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
#[ignore = "FUZZ-014"]
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

// FUZZ-015: a 40-step attach re-run is not capped by existing-bead output.
#[cfg(unix)]
#[test]
#[ignore = "FUZZ-015"]
fn fuzz_015_rerun_of_a_40_step_attach_is_not_capped_by_the_output_limit() {
    use std::os::unix::fs::PermissionsExt;

    let mut w = Workspace::new();
    let steps: Vec<Value> = (1..=40)
        .map(|index| json!({"id": format!("item_{index}"), "title": format!("Item {index}")}))
        .collect();
    let cooked = w.root.join("cooked.json");
    let shown = w.root.join("show.json");
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
            "#!/bin/sh\ncase \"$1\" in\n  cook) cat '{}' ;;\n  show) cat '{}' ;;\n  dep) printf '%s' '[{{\"id\":\"proj-1\",\"dependency_type\":\"parent-child\"}}]' ;;\n  *) printf '{{}}' ;;\nesac\n",
            cooked.display(),
            shown.display()
        ),
    )
    .expect("fake bd");
    fs::set_permissions(&bd, fs::Permissions::from_mode(0o755)).expect("chmod");
    w.req.bd_executable = Some(bd);
    let first = execute_bead_request(&w.req).expect("first preview");
    assert_eq!(first.outcome, BeadOutcome::Succeeded, "{first:#?}");
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
    let rerun = execute_bead_request(&w.req).expect("re-run must yield a receipt");
    assert_eq!(rerun.outcome, BeadOutcome::Succeeded, "{rerun:#?}");
    let graph = rerun.graph.expect("graph");
    assert_eq!(graph.nodes.len(), 40);
    assert!(
        graph
            .nodes
            .iter()
            .all(|node| node.action == BeadNodeAction::Existing)
    );
}

// FUZZ-016: option-like bead IDs reach bd as IDs, not options.
#[test]
#[ignore = "FUZZ-016"]
fn fuzz_016_option_like_bead_ids_are_never_parsed_as_bd_options() {
    let mut w = Workspace::new();
    w.req.parent = Some(BeadId::new("--db=/elsewhere").expect("valid bead id"));
    let runner = FakeRunner::new([
        ok(COOKED),
        out(
            Some(1),
            r#"{"error":"no issues found matching the provided IDs"}"#,
        ),
    ]);
    let receipt = execute_bead_request_with_runner(&w.req, &runner);
    for call in runner.calls() {
        let separator = call.args.iter().position(|arg| arg == "--");
        for (index, arg) in call.args.iter().enumerate() {
            if arg.starts_with("--db=") {
                assert!(
                    separator.is_some_and(|position| position < index),
                    "{:?}",
                    call.args
                );
            }
        }
    }
    if let Ok(receipt) = receipt {
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Refused {
                code: "BEADS_GRAPH_PARENT_NOT_FOUND".into()
            }
        );
    }
}

// FUZZ-020: apply failures preserve bd's error text.
#[test]
#[ignore = "FUZZ-020"]
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
        evidence.contains("graph apply failed") && evidence.contains("blocking dependency cycle")
    );
}
