//! Graph protocol and fail-closed behavior with an injectable bd runner.
use sc_composer_beads::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
const COOKED: &str =
    r#"{"formula":"sample","type":"workflow","steps":[{"id":"build","title":"Build"}]}"#;

fn public_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        return PathBuf::from(value.strip_prefix("\\\\?\\").unwrap_or(value.as_ref()));
    }
    #[cfg(not(windows))]
    path.to_path_buf()
}
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
                formula_name: Some(
                    sc_composer_beads::FormulaName::new("sample").expect("formula name"),
                ),
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

    fn with_results(outputs: impl IntoIterator<Item = io::Result<ProcessOutput>>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into_iter().collect()),
            calls: Mutex::new(Vec::new()),
        }
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
fn ok(s: &str) -> ProcessOutput {
    out(Some(0), s)
}
fn parent() -> ProcessOutput {
    ok(r#"[{"id":"proj-1"}]"#)
}
fn refused(r: &BeadComposeReceipt, code: &str, stage: BeadStage) {
    assert_eq!(
        r.outcome,
        BeadOutcome::Refused { code: code.into() },
        "{r:#?}"
    );
    assert_eq!(r.stages.last().expect("stage").stage, stage);
}
fn refused_with_reason(r: &BeadComposeReceipt, code: &str, stage: BeadStage, reason: &str) {
    refused(r, code, stage);
    let evidence = &r.stages.last().expect("stage").stderr_excerpt;
    assert!(
        evidence.contains(&format!(": {reason};")),
        "expected rejection reason `{reason}` in stage evidence: {evidence}"
    );
}
fn refused_with_stage_text(r: &BeadComposeReceipt, code: &str, stage: BeadStage, text: &str) {
    refused(r, code, stage);
    assert_eq!(
        r.stages.last().expect("stage").stderr_excerpt,
        text,
        "stage evidence"
    );
}
fn failed(r: &BeadComposeReceipt, code: &str, stage: BeadStage) {
    assert_eq!(
        r.outcome,
        BeadOutcome::Failed { code: code.into() },
        "{r:#?}"
    );
    assert_eq!(r.stages.last().expect("stage").stage, stage);
}
fn assert_read_only(runner: &FakeRunner) {
    assert!(
        runner
            .calls()
            .iter()
            .all(|c| matches!(c.args[0].as_str(), "cook" | "show" | "dep" | "where"))
    );
}

#[test]
fn attach_preview_and_apply_have_only_the_authorized_argv() {
    let mut w = Workspace::new();
    for (operation, preview) in [
        (BeadOperation::PreviewAttach, true),
        (BeadOperation::Attach, false),
    ] {
        w.req.operation = operation;
        let runner = FakeRunner::new([
            ok(COOKED),
            parent(),
            ok(if preview {
                "{}"
            } else {
                r#"{"ids":{"build":"proj-1.chain-build"}}"#
            }),
        ]);
        let r = w.run(&runner);
        assert_eq!(r.outcome, BeadOutcome::Succeeded, "{r:#?}");
        let calls = runner.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(
            calls[0].args,
            vec!["cook".into(), calls[0].args[1].clone(), "--json".into()]
        );
        assert_eq!(
            calls[1].args,
            ["show", "--json", "--", "proj-1", "proj-1.chain-build"]
        );
        let mut expected = vec!["create".into(), "--graph".into(), calls[2].args[2].clone()];
        if preview {
            expected.push("--dry-run".into());
        }
        expected.push("--json".into());
        assert_eq!(calls[2].args, expected);
        for (input, suffix) in [(&calls[0].args[1], "toml"), (&calls[2].args[2], "json")] {
            let input = std::path::Path::new(input);
            assert_eq!(input.parent().map(public_path), Some(public_path(&w.root)));
            assert_eq!(input.extension().and_then(|ext| ext.to_str()), Some(suffix));
            assert!(
                input
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".sc-compose-input-")
            );
            assert!(
                !input.exists(),
                "private inputs must be removed or published"
            );
        }
        assert!(
            calls[0].args[1].ends_with(".formula.toml"),
            "bd's TOML parser requires the full suffix"
        );
        assert_ne!(calls[0].args[1], w.req.rendered_formula.to_string_lossy());
        assert_ne!(
            calls[2].args[2],
            format!("{}.graph.json", w.req.rendered_formula.display())
        );
        let plan = w.plan();
        assert_eq!(plan["nodes"][0]["parent_id"], "proj-1");
        assert_eq!(
            plan["nodes"][0]["metadata"][PROVENANCE_KEY]["step"],
            "build"
        );
    }
}
#[test]
fn request_shape_and_authorization_fail_before_render_or_bd() {
    for case in 0..7 {
        let mut w = Workspace::new();
        match case {
            0 => w.req.parent = None,
            1 => w.req.ref_ = None,
            2 => {
                w.req.bead_variables.insert("var".into(), "value".into());
            }
            3 => w.req.operation = BeadOperation::Render,
            4 => {
                w.req.operation = BeadOperation::Pour;
                w.req.parent = None;
            }
            5 | 6 => {
                w.req.operation = if case == 5 {
                    BeadOperation::Render
                } else {
                    BeadOperation::Validate
                };
                w.req.parent = None;
                w.req.ref_ = None;
                w.req.relations = serde_json::from_value(
                    json!([{"from":"step:build","to":"bead:proj-2","type":"related"}]),
                )
                .expect("relation");
            }
            _ => unreachable!(),
        }
        let runner = FakeRunner::new([]);
        let err = execute_bead_request_with_runner(&w.req, &runner).expect_err("shape refused");
        assert_eq!(err.code(), "BEADS_REQUEST_DESERIALIZATION_FAILED");
        let expected_message = match case {
            0 => "attach requires parent",
            1 => "attach requires ref",
            2 => "attach forbids bead_variables",
            3 => "non-attach operations forbid parent",
            4 => "non-attach operations forbid ref",
            5 | 6 => "render and validate forbid relations",
            _ => unreachable!(),
        };
        assert!(err.to_string().contains(expected_message), "{err}");
        assert!(runner.calls().is_empty());
        assert!(!w.req.rendered_formula.exists());
    }
    let mut w = Workspace::new();
    w.req.operation = BeadOperation::Attach;
    w.req.pour_authorization = None;
    assert_eq!(
        execute_bead_request_with_runner(&w.req, &FakeRunner::new([]))
            .expect_err("authorization")
            .code(),
        "BEADS_POUR_AUTH_REQUIRED"
    );
}

#[test]
fn cook_parse_failure_keeps_the_parser_cause() {
    let w = Workspace::new();
    let runner = FakeRunner::new([ok("not-json")]);

    let receipt = w.run(&runner);

    failed(&receipt, "BEADS_COOK_FAILED", BeadStage::Validate);
    assert!(
        receipt
            .stages
            .last()
            .unwrap()
            .stderr_excerpt
            .contains("expected ident"),
        "{receipt:#?}"
    );
}

#[test]
fn graph_read_parse_failure_keeps_the_parser_cause() {
    let w = Workspace::new();
    let runner = FakeRunner::new([ok(COOKED), ok("not-json")]);

    let receipt = w.run(&runner);

    failed(
        &receipt,
        "BEADS_GRAPH_READ_FAILED",
        BeadStage::PreviewAttach,
    );
    assert!(
        receipt
            .stages
            .last()
            .unwrap()
            .stderr_excerpt
            .contains("expected ident"),
        "{receipt:#?}"
    );
}

#[test]
fn graph_apply_parse_failure_keeps_the_parser_cause() {
    let mut w = Workspace::new();
    w.req.operation = BeadOperation::Attach;
    let runner = FakeRunner::new([ok(COOKED), parent(), ok("not-json")]);

    let receipt = w.run(&runner);

    failed(&receipt, "BEADS_GRAPH_APPLY_FAILED", BeadStage::Attach);
    assert!(
        receipt
            .stages
            .last()
            .unwrap()
            .stderr_excerpt
            .contains("expected ident"),
        "{receipt:#?}"
    );
}
#[test]
fn scope_mismatch_precedes_every_bd_call() {
    for field in [
        sc_composer_beads::GraphIdField::Parent,
        sc_composer_beads::GraphIdField::Ref,
    ] {
        let mut w = Workspace::new();
        w.req
            .compose_variables
            .insert(field.to_string(), json!("other"));
        let runner = FakeRunner::new([]);
        let receipt = w.run(&runner);
        refused(&receipt, "BEADS_GRAPH_SCOPE_MISMATCH", BeadStage::Validate);
        assert!(
            receipt
                .stages
                .last()
                .expect("scope refusal stage")
                .stderr_excerpt
                .contains(&format!("graph scope {field} disagrees"))
        );
        assert!(runner.calls().is_empty());
    }
}
#[test]
fn parent_absence_is_distinct_from_read_failure() {
    for missing in [
        ok("[]"),
        out(
            Some(1),
            r#"{"error":"no issues found matching the provided IDs"}"#,
        ),
    ] {
        let w = Workspace::new();
        let runner = FakeRunner::new([ok(COOKED), missing]);
        refused(
            &w.run(&runner),
            "BEADS_GRAPH_PARENT_NOT_FOUND",
            BeadStage::PreviewAttach,
        );
        assert_read_only(&runner);
    }
    for read in [
        ok("{}"),
        ok("not JSON"),
        out(Some(1), r#"{"error":"permission denied"}"#),
        out(
            Some(2),
            r#"{"error":"no issues found matching the provided IDs"}"#,
        ),
        out(None, ""),
    ] {
        let w = Workspace::new();
        let runner = FakeRunner::new([ok(COOKED), read]);
        failed(
            &w.run(&runner),
            "BEADS_GRAPH_READ_FAILED",
            BeadStage::PreviewAttach,
        );
        assert_read_only(&runner);
    }
}
#[test]
fn ownership_conflicts_refuse_without_a_plan_write() {
    for (rows, reason) in [
        (
            r#"[{"id":"proj-1"},{"id":"proj-1.chain-build","metadata":{}}]"#,
            "not_owned",
        ),
        (
            r#"[{"id":"proj-1"},{"id":"proj-1.chain-build","metadata":{"sc_compose_graph":{}}}]"#,
            "provenance_differs",
        ),
    ] {
        let w = Workspace::new();
        let runner = FakeRunner::new([ok(COOKED), ok(rows)]);
        refused_with_reason(
            &w.run(&runner),
            "BEADS_GRAPH_CONFLICT",
            BeadStage::PreviewAttach,
            reason,
        );
        assert_read_only(&runner);
        assert!(
            !w.req
                .rendered_formula
                .with_extension("toml.graph.json")
                .exists()
        );
    }
}
fn existing(w: &Workspace) -> String {
    let preview = FakeRunner::new([ok(COOKED), parent(), ok("{}")]);
    assert_eq!(w.run(&preview).outcome, BeadOutcome::Succeeded);
    let metadata = w.plan()["nodes"][0]["metadata"].clone();
    json!([{"id":"proj-1"},{"id":"proj-1.chain-build","status":"closed","notes":"keep","metadata":metadata}]).to_string()
}
#[test]
fn edge_conflict_missing_edge_and_noop_preserve_the_prior_plan() {
    for (deps, code) in [
        (
            r#"[{"id":"proj-1","dependency_type":"blocks"}]"#,
            Some("BEADS_GRAPH_EDGE_CONFLICT"),
        ),
        ("[]", Some("BEADS_GRAPH_EDGE_MISSING")),
        (
            r#"[{"id":"proj-1","dependency_type":"parent-child"}]"#,
            None,
        ),
    ] {
        let w = Workspace::new();
        let rows = existing(&w);
        let before = w.plan();
        let mut dependencies = ok(deps);
        dependencies.stderr = "unrelated bd dep add forged source --type blocks".into();
        let runner = FakeRunner::new([ok(COOKED), ok(&rows), dependencies]);
        let r = w.run(&runner);
        if code == Some("BEADS_GRAPH_EDGE_MISSING") {
            assert_eq!(
                r.missing_edges,
                vec![MissingEdge {
                    from: BeadId::new("proj-1.chain-build").expect("child"),
                    to: BeadId::new("proj-1").expect("parent"),
                    kind: GraphDependencyType::ParentChild,
                }]
            );
        } else {
            assert!(r.missing_edges.is_empty());
        }
        if let Some(code) = code {
            refused(&r, code, BeadStage::PreviewAttach);
        } else {
            assert_eq!(r.outcome, BeadOutcome::Succeeded);
            assert!(r.graph.expect("graph").plan_path.is_none());
        }
        assert_read_only(&runner);
        assert_eq!(w.plan(), before);
        assert_eq!(
            runner.calls()[2].args,
            ["dep", "list", "--json", "--", "proj-1.chain-build"]
        );
    }
}
#[test]
fn apply_failure_is_failed_at_the_apply_stage() {
    let mut w = Workspace::new();
    w.req.operation = BeadOperation::Attach;
    let runner = FakeRunner::new([ok(COOKED), parent(), out(Some(1), "")]);
    failed(
        &w.run(&runner),
        "BEADS_GRAPH_APPLY_FAILED",
        BeadStage::Attach,
    );
    assert_eq!(
        runner
            .calls()
            .iter()
            .filter(|c| c.args[0] == "create")
            .count(),
        1
    );
}

#[test]
fn unavailable_bd_on_attach_returns_the_phase_r_error() {
    let w = Workspace::new();
    let runner =
        FakeRunner::with_results([Err(io::Error::new(io::ErrorKind::NotFound, "bd not found"))]);

    let error = execute_bead_request_with_runner(&w.req, &runner)
        .expect_err("unavailable bd must be a typed error");

    assert!(matches!(
        error,
        BeadComposeError::BdUnavailable { ref executable } if executable == &PathBuf::from("fake-bd")
    ));
    assert_eq!(runner.calls().len(), 1);
}

#[test]
fn read_and_apply_launch_errors_preserve_invoke_classification() {
    let w = Workspace::new();
    let read_runner = FakeRunner::with_results([
        Ok(ok(COOKED)),
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "argument cannot be represented",
        )),
    ]);
    let read_error = execute_bead_request_with_runner(&w.req, &read_runner)
        .expect_err("read launch error must be a typed error");
    assert!(matches!(
        read_error,
        BeadComposeError::ProcessArgumentInvalid { ref executable, ref message }
            if executable == &PathBuf::from("fake-bd")
                && message == "argument cannot be represented"
    ));

    let mut w = Workspace::new();
    w.req.operation = BeadOperation::Attach;
    let apply_runner = FakeRunner::with_results([
        Ok(ok(COOKED)),
        Ok(parent()),
        Err(io::Error::other(
            "sc-composer-beads process output limit exceeded",
        )),
    ]);
    let apply_error = execute_bead_request_with_runner(&w.req, &apply_runner)
        .expect_err("apply launch error must be a typed error");
    assert!(matches!(
        apply_error,
        BeadComposeError::ProcessOutputLimitExceeded {
            stage: BeadStage::Attach,
            limit_bytes: sc_composer_beads::runner::PROCESS_OUTPUT_LIMIT_BYTES,
        }
    ));
}
#[test]
fn invalid_relations_refuse_before_reading_beads() {
    for (relation, reason) in [
        (
            json!({"from":"step:absent","to":"step:build","type":"blocks"}),
            "unknown_step",
        ),
        (
            json!({"from":"step:build","to":"step:build","type":"blocks"}),
            "self_edge",
        ),
        (
            json!({"from":"bead:proj-2","to":"bead:proj-3","type":"related"}),
            "no_step",
        ),
        (
            json!({"from":"bead:proj-1","to":"bead:proj-2","type":"related"}),
            "no_step",
        ),
        (
            json!({"from":"step:build","to":"bead:proj-1","type":"related"}),
            "parent_pair",
        ),
    ] {
        let mut w = Workspace::new();
        w.req.relations = serde_json::from_value(json!([relation])).expect("relation");
        let runner = FakeRunner::new([ok(COOKED)]);
        refused_with_reason(
            &w.run(&runner),
            "BEADS_GRAPH_RELATION_INVALID",
            BeadStage::Validate,
            reason,
        );
        assert_eq!(runner.calls().len(), 1);
    }
    let mut w = Workspace::new();
    w.req.relations = serde_json::from_value(json!([
        {"from":"step:build","to":"bead:proj-2","type":"related"},
        {"from":"step:build","to":"bead:proj-2","type":"related"}
    ]))
    .expect("duplicate relations");
    let runner = FakeRunner::new([ok(COOKED)]);
    refused_with_reason(
        &w.run(&runner),
        "BEADS_GRAPH_RELATION_INVALID",
        BeadStage::Validate,
        "duplicate",
    );
    assert_eq!(runner.calls().len(), 1);
    let mut w = Workspace::new();
    w.req.relations =
        serde_json::from_value(json!([{"from":"step:build","to":"bead:proj-2","type":"related"}]))
            .expect("relation");
    let runner = FakeRunner::new([ok(COOKED), parent()]);
    refused_with_reason(
        &w.run(&runner),
        "BEADS_GRAPH_RELATION_INVALID",
        BeadStage::PreviewAttach,
        "bead_not_found",
    );
}
#[test]
fn each_captured_grammar_row_is_checked_at_validate_with_its_reason() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/beads/graph");
    let index: Value =
        serde_json::from_slice(&fs::read(root.join("captures.json")).expect("index"))
            .expect("JSON");
    for case in index["cases"].as_array().expect("cases") {
        let mut w = Workspace::new();
        let name = case["name"].as_str().expect("name");
        if name == "bead_variables_set" {
            continue;
        } // attach's request-shape refusal tested separately; graph pour below.
        let capture: Value = serde_json::from_slice(
            &fs::read(root.join(case["capture"].as_str().expect("capture"))).expect("capture file"),
        )
        .expect("JSON");
        let mut cooked = capture["stdout"].as_str().expect("stdout").to_owned();
        // Unknown keys are dropped by bd; inject a future parsed field explicitly.
        if name == "unknown_top" || name == "unknown_step" {
            let mut v: Value = serde_json::from_str(&cooked).expect("cooked");
            if name == "unknown_top" {
                v["future"] = json!(true);
            } else {
                v["steps"][0]["future"] = json!(true);
            }
            cooked = v.to_string();
        }
        let status = i32::try_from(capture["exit_status"].as_i64().expect("status")).expect("i32");
        let accepted = matches!(
            name,
            "allowed_top" | "allowed_step" | "needs_depends_on" | "extends"
        );
        w.req.formula_name = Some(sc_composer_beads::FormulaName::new(name).expect("formula name"));
        let mut outputs = vec![out(Some(status), &cooked)];
        if accepted {
            outputs.extend([parent(), ok("{}")]);
        }
        let runner = FakeRunner::new(outputs);
        let r = w.run(&runner);
        if status != 0 {
            failed(&r, "BEADS_COOK_FAILED", BeadStage::Validate);
        } else if accepted {
            assert_eq!(r.outcome, BeadOutcome::Succeeded, "{name}: {r:#?}");
        } else {
            if matches!(name, "loop" | "expand") {
                refused(&r, "BEADS_GRAPH_ID_INVALID", BeadStage::Validate);
            } else {
                let reason = match name {
                    "vars_declared" => "vars_declared",
                    "template" | "compose" | "advice" | "pointcuts" | "other_type" => "composition",
                    "unknown_top" | "unknown_step" => "unknown_key",
                    "children" | "expand_vars" | "condition" | "gate" | "on_complete"
                    | "waits_for" => "step_construct",
                    "reserved_metadata" => "reserved_metadata",
                    "label_comma" => "label_comma",
                    "no_steps" => "step_graph",
                    _ => unreachable!("unexpected successful capture: {name}"),
                };
                refused_with_reason(
                    &r,
                    "BEADS_GRAPH_FORMULA_UNSUPPORTED",
                    BeadStage::Validate,
                    reason,
                );
            }
            assert_read_only(&runner);
        }
    }
}
#[test]
fn graph_pour_forbids_runtime_variables_and_registry_pour_forbids_relations() {
    let mut w = Workspace::new();
    fs::create_dir_all(w.root.join(".beads/formulas")).expect("registry");
    w.req.operation = BeadOperation::PreviewPour;
    w.req.parent = None;
    w.req.ref_ = None;
    w.req
        .bead_variables
        .insert("runtime".into(), "value".into());
    let where_output = json!({"path":w.root.join(".beads")}).to_string();
    let runner = FakeRunner::new([ok("{}"), ok(&where_output), ok(COOKED)]);
    refused_with_reason(
        &w.run(&runner),
        "BEADS_GRAPH_FORMULA_UNSUPPORTED",
        BeadStage::PreviewPour,
        "bead_variables_set",
    );
    assert_read_only(&runner);
    w.req.bead_variables.clear();
    w.req.rendered_formula = w.root.join(".beads/formulas/sample.formula.toml");
    w.req.relations =
        serde_json::from_value(json!([{"from":"step:build","to":"bead:proj-2","type":"related"}]))
            .expect("relation");
    let runner = FakeRunner::new([ok("{}"), ok(&where_output)]);
    refused_with_stage_text(
        &w.run(&runner),
        "BEADS_GRAPH_RELATION_INVALID",
        BeadStage::PreviewPour,
        "relations are unavailable for registry pour",
    );
    assert_read_only(&runner);
}
#[test]
fn graph_plan_symlink_is_refused_without_touching_target() {
    #[cfg(unix)]
    {
        let w = Workspace::new();
        let target = w.root.join("keep");
        fs::write(&target, "unchanged").expect("target");
        std::os::unix::fs::symlink(
            &target,
            w.req.rendered_formula.with_extension("toml.graph.json"),
        )
        .expect("symlink");
        let runner = FakeRunner::new([ok(COOKED), parent()]);
        refused(
            &w.run(&runner),
            "BEADS_OUTPUT_PATH_SYMLINK",
            BeadStage::PreviewAttach,
        );
        assert_eq!(fs::read_to_string(target).expect("target"), "unchanged");
        assert_read_only(&runner);
    }
}

#[test]
fn applied_pour_keeps_external_ids_distinct_from_step_and_root_names() {
    let mut w = Workspace::new();
    w.req.operation = BeadOperation::Pour;
    w.req.parent = None;
    w.req.ref_ = None;
    w.req.relations = serde_json::from_value(json!([
        {"from":"step:build","to":"bead:_root","type":"related"},
        {"from":"bead:step:build","to":"step:build","type":"validates"}
    ]))
    .expect("relations");
    let registry = w.root.join(".beads");
    fs::create_dir(&registry).expect("registry");
    let runner = FakeRunner::new([
        ok("{}"),
        ok(&json!({"path":registry}).to_string()),
        ok(COOKED),
        ok(r#"[{"id":"_root"},{"id":"step:build"}]"#),
        ok("[]"),
        ok(r#"{"ids":{"_root":"proj-root","step:build":"proj-child"}}"#),
    ]);
    let receipt = w.run(&runner);
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
    let graph = receipt.graph.expect("graph");
    assert_eq!(
        (
            graph.edges[0].from.to_string().as_str(),
            graph.edges[0].to.to_string().as_str()
        ),
        ("proj-child", "_root")
    );
    assert_eq!(
        (
            graph.edges[1].from.to_string().as_str(),
            graph.edges[1].to.to_string().as_str()
        ),
        ("step:build", "proj-child")
    );
    assert_eq!(
        (
            graph.edges[2].from.to_string().as_str(),
            graph.edges[2].to.to_string().as_str()
        ),
        ("proj-child", "proj-root")
    );
    assert_eq!(runner.calls().len(), 6);
}

#[test]
fn provenance_normalizes_newlines_and_relation_order() {
    let mut w = Workspace::new();
    w.req.relations = serde_json::from_value(json!([
        {"from":"step:build","to":"bead:proj-a","type":"related"},
        {"from":"step:build","to":"bead:proj-b","type":"validates"}
    ]))
    .expect("relations");
    let mut provenance = Vec::new();
    for newline in ["\n", "\r\n", "\r"] {
        fs::write(
            &w.req.template,
            format!("formula = \"sample\"{newline}version = 1{newline}"),
        )
        .expect("template");
        w.req.relations.reverse();
        let runner = FakeRunner::new([
            ok(COOKED),
            ok(r#"[{"id":"proj-1"},{"id":"proj-a"},{"id":"proj-b"}]"#),
            ok("{}"),
        ]);
        assert_eq!(w.run(&runner).outcome, BeadOutcome::Succeeded);
        provenance.push(w.plan()["nodes"][0]["metadata"][PROVENANCE_KEY].clone());
    }
    assert_eq!(provenance[0], provenance[1]);
    assert_eq!(provenance[0], provenance[2]);
}

#[test]
fn dependency_read_rejects_unsafe_type_tokens_before_planning() {
    let w = Workspace::new();
    let rows = existing(&w);
    let before = w.plan();
    let runner = FakeRunner::new([
        ok(COOKED),
        ok(&rows),
        ok(r#"[{"id":"proj-1","dependency_type":"parent-child; echo injected"}]"#),
    ]);
    failed(
        &w.run(&runner),
        "BEADS_GRAPH_READ_FAILED",
        BeadStage::PreviewAttach,
    );
    assert_read_only(&runner);
    assert_eq!(w.plan(), before);
}

#[test]
fn missing_relation_retains_typed_reason_with_nonempty_bd_stderr() {
    for operation in [BeadOperation::PreviewAttach, BeadOperation::Attach] {
        let mut workspace = Workspace::new();
        workspace.req.operation = operation;
        workspace.req.relations = serde_json::from_value(json!([
            {"from": "step:build", "to": "bead:proj-2", "type": "related"}
        ]))
        .expect("relation");
        let stderr = "Issue proj-2 not found\n";
        let mut missing_relation = parent();
        missing_relation.stderr = stderr.into();
        let runner = FakeRunner::new([ok(COOKED), missing_relation]);

        let receipt = workspace.run(&runner);
        let stage = if operation == BeadOperation::Attach {
            BeadStage::Attach
        } else {
            BeadStage::PreviewAttach
        };
        refused_with_reason(
            &receipt,
            "BEADS_GRAPH_RELATION_INVALID",
            stage,
            "bead_not_found",
        );
        let failed_stage = receipt.stages.last().expect("failed stage");
        assert_eq!(
            failed_stage.outcome,
            BeadStageOutcome::Failed {
                code: "BEADS_GRAPH_RELATION_INVALID".into()
            }
        );
        assert!(failed_stage.stderr_excerpt.contains(stderr));
        assert_eq!(failed_stage.stdout_excerpt, "[{\"id\":\"proj-1\"}]");
        assert_eq!(failed_stage.exit_status, Some(0));
        assert_eq!(runner.calls().len(), 2);
        assert_read_only(&runner);
    }
}
