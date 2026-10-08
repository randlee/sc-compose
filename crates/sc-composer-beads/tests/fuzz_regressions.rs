//! Regression tests promoted from the Phase T adversarial fuzz campaign.

use sc_compose_test_support as shell_literal;

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

#[cfg(windows)]
fn public_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix("\\\\?\\UNC\\") {
        return format!("\\\\{unc}");
    }
    value
        .strip_prefix("\\\\?\\")
        .unwrap_or(value.as_ref())
        .to_owned()
}

#[cfg(not(windows))]
fn public_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
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

    #[cfg(unix)]
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

// TMPL5-02: every bead route refuses an undefined composition variable in a
// TOML or JSON formula template before writing the formula or calling bd.
#[test]
fn tmpl5_02_every_route_refuses_an_undefined_variable_without_frontmatter() {
    for (template, body) in [
        ("sample.formula.toml.j2", "formula = \"{{{ title }}}\"\n"),
        (
            "sample.formula.json.j2",
            "{ \"formula\": \"{{{ title }}}\" }",
        ),
    ] {
        for operation in [
            BeadOperation::Render,
            BeadOperation::Validate,
            BeadOperation::PreviewPour,
            BeadOperation::Pour,
            BeadOperation::PreviewAttach,
            BeadOperation::Attach,
        ] {
            let mut w = Workspace::new();
            w.req.operation = operation;
            if !matches!(
                operation,
                BeadOperation::PreviewAttach | BeadOperation::Attach
            ) {
                w.req.parent = None;
                w.req.ref_ = None;
            }
            if !matches!(operation, BeadOperation::Pour | BeadOperation::Attach) {
                w.req.pour_authorization = None;
            }
            w.req.template = w.root.join(template);
            w.req.rendered_formula = w.root.join(template.trim_end_matches(".j2"));
            fs::write(&w.req.template, body).expect("template");
            let runner = FakeRunner::new([]);
            let receipt = w.run(&runner);
            failed(&receipt, "BEADS_RENDER_FAILED", BeadStage::Render);
            assert!(
                receipt
                    .stages
                    .last()
                    .expect("stage")
                    .stderr_excerpt
                    .contains("title"),
                "{operation:?} {template}: {receipt:#?}"
            );
            assert!(runner.calls().is_empty(), "{operation:?} {template}");
            assert!(
                !w.req.rendered_formula.exists(),
                "{operation:?} {template}: nothing is written"
            );
        }
    }
}

// FUZZ-013: Phase R requests parse unchanged (ADR-0023 Decision 1).
#[test]
fn fuzz_013_phase_r_operations_keep_accepting_legacy_formula_names() {
    for operation in ["render", "validate", "preview_pour", "pour"] {
        for name in ["café", "re g0", "a+b"] {
            let request = json!({
                "schema": BEADS_SCHEMA_V1,
                "operation": operation,
                "working_directory": "/work",
                "template": "f.formula.toml.j2",
                "rendered_formula": "/work/build/f.formula.toml",
                "formula_name": name,
                "compose_variables": {},
                "bead_variables": {}
            });
            let parsed = parse_request(&request.to_string());
            assert!(parsed.is_ok(), "{operation} {name}: {:?}", parsed.err());
        }
    }
}

// pe-f5: adapters share the JSON parser's operation-aware formula-name boundary.
#[test]
fn formula_name_boundary_matches_json_parser_for_every_operation() {
    for operation in [
        "render",
        "validate",
        "preview_pour",
        "pour",
        "preview_attach",
        "attach",
    ] {
        let wire = serde_json::from_value::<BeadOperation>(json!(operation)).expect("operation");
        for name in ["café", "re g0", "a+b", "workflow", ""] {
            let attach = matches!(operation, "attach" | "preview_attach");
            let mut request = json!({
                "schema": BEADS_SCHEMA_V1,
                "operation": operation,
                "working_directory": "/work",
                "template": "f.formula.toml.j2",
                "rendered_formula": "/work/build/f.formula.toml",
                "formula_name": name,
                "compose_variables": {},
                "bead_variables": {}
            });
            if attach {
                request["parent"] = json!("proj-1");
                request["ref"] = json!("valid");
            }
            let shared = FormulaName::for_operation(wire, name.to_owned());
            let parsed = parse_request(&request.to_string());
            match (shared, parsed) {
                (Ok(expected), Ok(request)) => {
                    assert_eq!(request.formula_name, expected, "{operation} {name:?}");
                }
                (Err(_), Err(_)) => {}
                (shared, parsed) => panic!("{operation} {name:?}: {shared:?} vs {parsed:?}"),
            }
        }
    }
}

// pe-f12 (DIFF5-01): request JSON errors report the caller's line and column,
// never a location in an internal compact re-serialization.
#[test]
fn request_json_errors_report_source_line_and_column() {
    let request = json!({
        "schema": BEADS_SCHEMA_V1,
        "operation": "render",
        "working_directory": "/work",
        "template": "f.formula.toml.j2",
        "rendered_formula": "/work/build/f.formula.toml",
        "compose_variables": [],
        "bead_variables": {}
    });
    let pretty = serde_json::to_string_pretty(&request).unwrap();
    let line = pretty
        .lines()
        .position(|line| line.contains("compose_variables"))
        .map(|index| index + 1)
        .unwrap();
    assert!(line > 1, "the request must span several lines");
    let error = parse_request(&pretty)
        .expect_err("array compose_variables")
        .to_string();
    assert!(
        error.contains(&format!("line {line} ")),
        "expected source line {line}: {error}"
    );
    assert!(!error.contains("line 1 "), "{error}");
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

// FUZZ-015 round 3: bd emits one diagnostic per missing planned id.
#[cfg(unix)]
#[test]
fn fuzz_015_preview_of_500_missing_steps_batches_not_found_diagnostics() {
    scalable_attach_roundtrip(500, false);
}

#[test]
fn fuzz_015_later_issue_batch_failure_stops_before_graph_apply() {
    let w = Workspace::new();
    let steps: Vec<Value> = (0..500)
        .map(|i| json!({"id":format!("item_{i}"), "title":"Item"}))
        .collect();
    let cooked = json!({"formula":"sample", "type":"workflow", "steps":steps}).to_string();
    for failure in [
        out(Some(2), r#"{"error":"read failed"}"#),
        ok(r#"[{"id":"proj-1"}]"#), // A parent row belongs to the first batch, not the second.
    ] {
        let runner = FakeRunner::new([ok(&cooked), parent(), failure]);
        let receipt = w.run(&runner);
        failed(
            &receipt,
            "BEADS_GRAPH_READ_FAILED",
            BeadStage::PreviewAttach,
        );
        assert!(runner.calls().iter().all(|call| call.args[0] != "create"));
    }
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
    write_scalable_fake_bd(&bd, &cooked, &created, &shown);
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

#[cfg(unix)]
fn write_scalable_fake_bd(
    bd: &std::path::Path,
    cooked: &std::path::Path,
    created: &std::path::Path,
    shown: &std::path::Path,
) {
    let script = r#"#!/usr/bin/env python3
import json, sys
command = sys.argv[1]
if command in ('cook', 'create'):
    with open(COOKED_PATH if command == 'cook' else CREATED_PATH) as source:
        sys.stdout.write(source.read())
elif command == 'show':
    requested = sys.argv[sys.argv.index('--') + 1:]
    with open(SHOWN_PATH) as source:
        existing = {row['id']: row for row in json.load(source)}
    found = [existing[id] for id in requested if id in existing]
    for id in requested:
        if id not in existing:
            sys.stderr.write('Issue ' + id + ' not found: ' + 'diagnostic ' * 20 + '\n')
    print(json.dumps(found if found else {'error': 'no issues found matching the provided IDs'}))
    sys.exit(0 if found else 1)
elif command == 'dep':
    print('[{"id":"proj-1","dependency_type":"parent-child"}]')
else:
    print('{}')
"#
    .replace(
        "COOKED_PATH",
        &serde_json::to_string(cooked).expect("UTF-8 fixture path"),
    )
    .replace(
        "CREATED_PATH",
        &serde_json::to_string(created).expect("UTF-8 fixture path"),
    )
    .replace(
        "SHOWN_PATH",
        &serde_json::to_string(shown).expect("UTF-8 fixture path"),
    );
    fs::write(bd, script).expect("fake bd");
}

const OPTION_LIKE_IDS: [&str; 3] = ["--db=/elsewhere", "--json", "-q"];

// FUZZ-040: recovery command arguments cannot execute shell syntax or emit controls.
#[test]
fn fuzz_040_recovery_arguments_are_shell_quoted_and_control_escaped() {
    let from = "source'\\\u{7}\u{7f}\u{80}\u{202e}\u{200b}\u{00ad}$(literal)$HOME`literal`";
    let to = "target\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}\u{200e}\u{200f}\u{feff}";
    let error = sc_composer_beads::BeadComposeError::GraphEdgeMissing {
        edges: vec![sc_composer_beads::MissingEdge {
            from: sc_composer_beads::BeadId::new(from).unwrap(),
            to: sc_composer_beads::BeadId::new(to).unwrap(),
            kind: sc_composer_beads::GraphDependencyType::try_from("blocks".to_owned()).unwrap(),
        }],
    };
    let message = error.to_string(); // Drives missing_edge_commands, not just shell_quote.
    assert!(!message.chars().any(char::is_control), "{message:?}");
    for raw in ['\u{200b}', '\u{feff}', '\u{00ad}', '\u{202e}'] {
        assert!(!message.contains(raw), "{message:?}");
    }
    for escaped in [
        "\\xE2\\x80\\x8B",
        "\\xEF\\xBB\\xBF",
        "\\xC2\\xAD",
        "\\xE2\\x80\\xAE",
    ] {
        assert!(message.contains(escaped), "{message:?}");
    }
    let command = message
        .strip_prefix("graph edges missing; repair then retry: ")
        .unwrap();
    let arguments = command
        .strip_prefix("bd dep add ")
        .unwrap()
        .strip_suffix(" --type 'blocks'")
        .unwrap();
    shell_literal::assert_round_trip(arguments, &[from, to]);
    #[cfg(unix)]
    assert_bash_round_trip(arguments, format!("{from}\0{to}\0").as_bytes());

    let separators = "line\u{2028}paragraph\u{2029}end";
    let escaped_separators = sc_composer_beads::error::shell_quote(separators);
    assert!(escaped_separators.contains("\\xE2\\x80\\xA8"));
    assert!(escaped_separators.contains("\\xE2\\x80\\xA9"));
    shell_literal::assert_round_trip(&escaped_separators, &[separators]);
    #[cfg(unix)]
    assert_bash_round_trip(&escaped_separators, format!("{separators}\0").as_bytes());
    assert_eq!(sc_composer_beads::error::shell_quote("a'b"), "'a'\"'\"'b'");
    for value in [
        "",
        " ",
        "trailing\\",
        "a'b",
        "$(literal);$HOME`literal`*?[]",
    ] {
        shell_literal::assert_round_trip(&sc_composer_beads::error::shell_quote(value), &[value]);
    }
}

#[cfg(unix)]
fn assert_bash_round_trip(arguments: &str, expected: &[u8]) {
    let mut tested_shell = false;
    for shell in ["/bin/bash", "bash"] {
        let output = std::process::Command::new(shell)
            .args(["-c", &format!("printf '%s\\0' {arguments}")])
            .env("LC_ALL", "C.UTF-8")
            .output();
        let Ok(output) = output else {
            continue;
        };
        tested_shell = true;
        assert!(output.status.success(), "{shell}: {output:?}");
        assert_eq!(output.stdout, expected, "{shell}");
    }
    assert!(
        tested_shell,
        "neither /bin/bash nor bash from PATH is available"
    );
}

fn option_id_argument_errors(calls: &[CommandSpec], ids: &[&str]) -> Vec<String> {
    let mut checked = 0;
    let mut errors = Vec::new();
    for call in calls.iter().filter(|call| call.args[0] != "cook") {
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

// FUZZ-041: a relation from the attach parent to itself is a self edge.
#[test]
fn fuzz_041_parent_self_relation_reports_self_edge() {
    let mut w = Workspace::new();
    let parent = BeadId::new("proj-1").expect("parent");
    w.req.relations.push(BeadRelation {
        from: BeadEndpoint::Bead(parent.clone()),
        to: BeadEndpoint::Bead(parent),
        kind: BeadDependencyType::Blocks,
    });
    let receipt = w.run(&FakeRunner::new([ok(COOKED)]));

    assert_eq!(
        receipt.outcome,
        BeadOutcome::Refused {
            code: "BEADS_GRAPH_RELATION_INVALID".into()
        },
        "{receipt:#?}"
    );
    assert_eq!(
        receipt.stages.last().expect("stage").stage,
        BeadStage::Validate
    );
    let diagnostic = &receipt.stages.last().expect("stage").stderr_excerpt;
    assert!(diagnostic.contains("self_edge"), "{diagnostic}");
    assert!(!diagnostic.contains("parent_pair"), "{diagnostic}");
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
                "bead_variables": {}, "parent": "proj-1", "ref": reference,
                "pour_authorization": "CreatePersistentBeads"
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
            assert_eq!(
                receipt.rendered_formula,
                PathBuf::from(public_path(&w.req.rendered_formula))
            );
            assert_eq!(
                graph.plan_path.as_ref().expect("public plan"),
                &PathBuf::from(public_path(
                    &w.req
                        .rendered_formula
                        .with_extension(format!("{suffix}.graph.json")),
                ))
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
                "bead_variables": {}, "parent": invalid, "ref": "valid",
                "pour_authorization": "CreatePersistentBeads"
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
                    public_path(Path::new(&calls[0].args[1])),
                    public_path(Path::new(&calls[2].args[1])),
                    "both cooks must use the same public snapshot path"
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
                assert_eq!(
                    receipt.rendered_formula,
                    PathBuf::from(public_path(&w.req.rendered_formula))
                );
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
// FUZZ-042: a missing rendered_formula parent is an output-path error.
#[test]
fn fuzz_042_missing_rendered_formula_directory_is_typed() {
    let mut w = Workspace::new();
    let missing_directory = w.root.join("missing_dir");
    w.req.rendered_formula = missing_directory.join("m.formula.toml");
    let runner = FakeRunner::new([]);

    let error = execute_bead_request_with_runner(&w.req, &runner)
        .expect_err("missing output directory must be rejected");
    assert_eq!(error.code(), "BEADS_OUTPUT_PATH_INVALID");
    assert!(error.to_string().contains("m.formula.toml"));
    assert!(error.to_string().contains("missing_dir"));
    let details =
        serde_json::to_value(&error).expect("serialize output-path diagnostic")["details"].clone();
    assert_eq!(details["field"], "rendered_formula");
    assert_eq!(details["value"], public_path(&w.req.rendered_formula));
    assert!(
        details["rule"]
            .as_str()
            .expect("rule")
            .contains(&public_path(&missing_directory))
    );
    assert!(
        details["rule"]
            .as_str()
            .expect("rule")
            .contains("must exist")
    );
    assert!(runner.calls().is_empty(), "{:#?}", runner.calls());
}

// FUZZ-050: an unusable public output must refuse before any graph transaction.
#[test]
fn fuzz_050_directory_outputs_refuse_attach_before_bd_create() {
    for operation in [BeadOperation::PreviewAttach, BeadOperation::Attach] {
        for graph_plan in [true, false] {
            let mut w = Workspace::new();
            w.req.operation = operation;
            let destination = if graph_plan {
                w.req.rendered_formula.with_extension("toml.graph.json")
            } else {
                w.req.rendered_formula.clone()
            };
            fs::create_dir(&destination).expect("unusable output directory");
            let runner = FakeRunner::new([
                ok(COOKED),
                parent(),
                ok(if operation == BeadOperation::Attach {
                    r#"{"ids":{"build":"proj-1.chain-build"}}"#
                } else {
                    "{}"
                }),
            ]);
            let result = execute_bead_request_with_runner(&w.req, &runner);
            let calls = runner.calls();
            assert!(
                !calls
                    .iter()
                    .any(|call| call.args.first().is_some_and(|arg| arg == "create")),
                "{operation:?}: local publication failure must happen before bd create: {calls:#?}"
            );
            match result {
                Ok(receipt) => {
                    assert_eq!(
                        receipt.outcome,
                        BeadOutcome::Refused {
                            code: "BEADS_OUTPUT_PATH_INVALID".into()
                        },
                        "{receipt:#?}"
                    );
                    assert!(receipt.graph.is_none());
                    let diagnostic = &receipt.stages.last().expect("stage").stderr_excerpt;
                    assert!(
                        diagnostic.contains(&public_path(&destination)),
                        "{diagnostic}"
                    );
                }
                Err(error) => {
                    assert_eq!(error.code(), "BEADS_OUTPUT_PATH_INVALID", "{error}");
                    assert!(
                        matches!(&error,BeadComposeError::OutputPathInvalid{path,..} if path.to_string_lossy()==public_path(&destination))
                    );
                }
            }
            assert!(
                destination.is_dir(),
                "must not replace the unusable destination"
            );
            assert!(
                !fs::read_dir(&w.root).expect("directory").any(|entry| entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sc-compose")),
                "private input leak"
            );
        }
    }
}

/// Makes further publication impossible after bd has consumed its own inputs.
struct PublishedBeforeCreateRunner {
    inner: FakeRunner,
    formula: PathBuf,
    plan: PathBuf,
}

impl ProcessRunner for PublishedBeforeCreateRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        if spec.args[0] == "cook" {
            assert_eq!(fs::read(&self.formula)?, fs::read(&spec.args[1])?);
        }
        if spec.args[0] == "create" {
            let input = spec
                .args
                .iter()
                .position(|arg| arg == "--graph")
                .expect("graph")
                + 1;
            assert_eq!(fs::read(&self.plan)?, fs::read(&spec.args[input])?);
            // Another owner can replace public paths after publication. This
            // request must need no further local output writes after bd runs.
            for path in [&self.formula, &self.plan] {
                fs::remove_file(path)?;
                fs::create_dir(path)?;
            }
        }
        self.inner.run(spec)
    }
}

#[test]
fn fuzz_050_attach_publishes_all_outputs_before_create_and_never_after() {
    for operation in [BeadOperation::PreviewAttach, BeadOperation::Attach] {
        let mut w = Workspace::new();
        w.req.operation = operation;
        let runner = PublishedBeforeCreateRunner {
            inner: FakeRunner::new([
                ok(COOKED),
                parent(),
                ok(if operation == BeadOperation::Attach {
                    r#"{"ids":{"build":"proj-1.chain-build"}}"#
                } else {
                    "{}"
                }),
            ]),
            formula: w.req.rendered_formula.clone(),
            plan: w.req.rendered_formula.with_extension("toml.graph.json"),
        };
        let receipt = execute_bead_request_with_runner(&w.req, &runner).expect("request");
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
        assert!(receipt.graph.is_some());
        assert!(runner.formula.is_dir());
        assert!(runner.plan.is_dir());
        assert!(fs::read_dir(&w.root).expect("files").all(|entry| {
            !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".sc-compose-input-")
        }));
    }
}

#[test]
fn fuzz_021_refused_parse_preserves_native_details_and_legacy_consumers() {
    for operation in ["attach", "preview_attach"] {
        let base = json!({"schema":BEADS_SCHEMA_V1,"operation":operation,
            "working_directory":"relative-work", "template":"missing.toml.j2",
            "rendered_formula":"out.toml", "compose_variables":{},"bead_variables":{},
            "parent":"proj-1","ref":"valid","pour_authorization":"CreatePersistentBeads"});
        assert!(matches!(
            parse_request_with_outcome(&base.to_string()).unwrap(),
            RequestParseOutcome::Ready(_)
        ));
        for (field, value, endpoint) in [
            ("bead", "bad id", None),
            ("step", "bad-step", Some("step:bad-step")),
            ("bead", "bad id", Some("bead:bad id")),
            ("ref", "a.b", None),
        ] {
            let mut input = base.clone();
            if let Some(endpoint) = endpoint {
                input["relations"] = json!([{"from":endpoint,"to":"bead:proj-1","type":"blocks"}]);
            } else {
                input[if field == "bead" { "parent" } else { "ref" }] = json!(value);
            }
            let error = parse_request(&input.to_string()).unwrap_err();
            let RequestParseOutcome::Refused(refused) =
                parse_request_with_outcome(&input.to_string()).unwrap()
            else {
                panic!("expected refused receipt")
            };
            assert_eq!(
                refused.error,
                BeadDiagnostic::graph_id_invalid(&error).unwrap()
            );
            assert_eq!(refused.error.details.as_ref().unwrap()["field"], field);
            assert_eq!(refused.error.details.as_ref().unwrap()["value"], value);
            assert!(refused.error.details.as_ref().unwrap()["rule"].is_string());
            assert!(refused.receipt.rendered_formula.is_absolute());
            assert_eq!(refused.receipt.stages[0].stage, BeadStage::Validate);
            let wire = serde_json::to_value(&refused).unwrap();
            let legacy: BeadComposeReceipt = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(legacy, refused.receipt);
            assert_eq!(
                serde_json::from_value::<RefusedBeadComposeReceipt>(wire).unwrap(),
                refused
            );
        }
        let mut invalid = base.clone();
        invalid["ref"] = json!("x".repeat(100_000));
        let RequestParseOutcome::Refused(receipt) =
            parse_request_with_outcome(&invalid.to_string()).unwrap()
        else {
            panic!("refused")
        };
        assert!(receipt.receipt.stages[0].stderr_excerpt.chars().count() <= 16 * 1024);
        invalid["operation"] = json!("unknown");
        assert_eq!(
            parse_request_with_outcome(&invalid.to_string())
                .unwrap_err()
                .code(),
            "BEADS_REQUEST_DESERIALIZATION_FAILED"
        );
        if operation == "attach" {
            invalid["operation"] = json!(operation);
            invalid
                .as_object_mut()
                .unwrap()
                .remove("pour_authorization");
            assert_eq!(
                parse_request_with_outcome(&invalid.to_string())
                    .unwrap_err()
                    .code(),
                "BEADS_POUR_AUTH_REQUIRED"
            );
        }
    }
}

// FUZZ-049: parser fallback preserves typed errors for malformed fields and duplicate keys.
#[test]
fn fuzz_049_request_fallback_preserves_typed_errors() {
    for operation in ["render", "validate", "preview_pour", "pour"] {
        for name in [serde_json::json!(5), serde_json::json!({"bad": true})] {
            let request = serde_json::json!({"schema":BEADS_SCHEMA_V1,"operation":operation,"working_directory":"/work","template":"f.formula.toml.j2","rendered_formula":"/work/f.formula.toml","formula_name":name,"compose_variables":{},"bead_variables":{}});
            let error = parse_request(&request.to_string())
                .expect_err("non-string formula_name must remain a typed parse error");
            assert_eq!(error.code(), "BEADS_REQUEST_DESERIALIZATION_FAILED");
            assert!(error.to_string().contains("formula_name"), "{error}");
        }

        for duplicate in [
            format!(
                r#"{{"schema":"{BEADS_SCHEMA_V1}","operation":"{operation}","working_directory":"/work","template":"a","template":"b","rendered_formula":"/work/f.formula.toml","formula_name":"re g0","compose_variables":{{}},"bead_variables":{{}}}}"#
            ),
            format!(
                r#"{{"schema":"{BEADS_SCHEMA_V1}","operation":"{operation}","working_directory":"/work","formula_name":"re g0","template":"a","template":"b","rendered_formula":"/work/f.formula.toml","compose_variables":{{}},"bead_variables":{{}}}}"#
            ),
        ] {
            let error = parse_request(&duplicate)
                .expect_err("duplicate top-level template must remain a typed parse error");
            assert_eq!(error.code(), "BEADS_REQUEST_DESERIALIZATION_FAILED");
            assert!(
                error.to_string().contains("duplicate field `template`"),
                "{error}"
            );
        }
    }
}

// FUZZ-042: invalid output parents name the requested rendered formula.
#[test]
fn fuzz_042_parent_file_and_relative_output_are_typed() {
    let mut w = Workspace::new();
    let parent_file = w.root.join("not-a-directory");
    fs::write(&parent_file, "file").expect("parent file");
    w.req.rendered_formula = parent_file.join("out.formula.toml");
    let error = execute_bead_request_with_runner(&w.req, &FakeRunner::new([]))
        .expect_err("file parent must be rejected as an output path");
    assert_eq!(error.code(), "BEADS_OUTPUT_PATH_INVALID");
    assert!(
        error
            .to_string()
            .contains(&public_path(&w.req.rendered_formula)),
        "{error}"
    );

    let mut w = Workspace::new();
    w.req.rendered_formula = PathBuf::from("missing-dir/out.formula.toml");
    let error = execute_bead_request_with_runner(&w.req, &FakeRunner::new([]))
        .expect_err("relative output with no parent must be rejected");
    assert_eq!(error.code(), "BEADS_OUTPUT_PATH_INVALID");
    assert!(!error.to_string().contains("``"), "{error}");
    assert!(
        error.to_string().contains(&public_path(
            &w.root.join("missing-dir").join("out.formula.toml")
        )),
        "{error}"
    );
}
// FUZZ-012: relative rendered outputs are rooted at working_directory for Phase R operations.
#[test]
fn fuzz_012_relative_rendered_formula_is_rooted_at_working_directory() {
    for operation in [BeadOperation::Render, BeadOperation::PreviewPour] {
        let mut w = Workspace::new();
        w.req.operation = operation;
        w.req.parent = None;
        w.req.ref_ = None;
        w.req.rendered_formula = PathBuf::from("nested/out.formula.toml");
        fs::create_dir_all(w.root.join("nested")).expect("output directory");
        let runner = if operation == BeadOperation::PreviewPour {
            FakeRunner::new([
                ok(COOKED),
                ok(&format!(
                    r#"{{"path":"{}"}}"#,
                    w.root.join(".beads").display()
                )),
                ok(""),
            ])
        } else {
            FakeRunner::new([])
        };
        let receipt = w.run(&runner);
        assert_eq!(
            receipt.rendered_formula,
            PathBuf::from(public_path(&w.root.join("nested/out.formula.toml")))
        );
        assert!(receipt.rendered_formula.is_file());
    }
}

struct CookFailureSourceRunner {
    inner: FakeRunner,
    fail_cook: usize,
    cooks: Mutex<usize>,
}

impl ProcessRunner for CookFailureSourceRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        if spec.args[0] == "cook" {
            let mut cooks = self.cooks.lock().expect("cooks");
            *cooks += 1;
            assert!(spec.args[1].contains(".sc-compose-input-"), "{spec:#?}");
            assert!(PathBuf::from(&spec.args[1]).is_file(), "live private input");
            if *cooks == self.fail_cook {
                self.inner.calls.lock().expect("calls").push(spec.clone());
                return Ok(ProcessOutput {
                    stderr: format!("cannot cook source {}: invalid formula", spec.args[1]),
                    ..out(Some(7), "")
                });
            }
        }
        self.inner.run(spec)
    }
}

#[test]
fn fuzz_055_real_registry_pour_cook_failure_receipt_names_public_source() {
    for operation in [BeadOperation::PreviewPour, BeadOperation::Pour] {
        let mut w = Workspace::new();
        w.req.operation = operation;
        w.req.parent = None;
        w.req.ref_ = None;
        let formulas = w.root.join(".beads/formulas");
        fs::create_dir_all(&formulas).expect("registry formulas");
        w.req.rendered_formula = formulas.join("sample.formula.toml");
        let runner = CookFailureSourceRunner {
            inner: FakeRunner::new([]),
            fail_cook: 1,
            cooks: Mutex::new(0),
        };
        assert_fuzz_055_cook_failure(&w, &runner, BeadStage::Validate, 1);
    }
}

#[test]
fn fuzz_055_real_graph_routes_cook_failure_receipt_names_public_source() {
    for operation in [
        BeadOperation::PreviewPour,
        BeadOperation::Pour,
        BeadOperation::PreviewAttach,
        BeadOperation::Attach,
    ] {
        let mut w = Workspace::new();
        w.req.operation = operation;
        let by_path = matches!(operation, BeadOperation::PreviewPour | BeadOperation::Pour);
        if by_path {
            w.req.parent = None;
            w.req.ref_ = None;
        }
        let registry = w.root.join(".beads");
        fs::create_dir(&registry).expect("registry");
        let runner = CookFailureSourceRunner {
            inner: FakeRunner::new(if by_path {
                vec![ok(COOKED), ok(&json!({"path":registry}).to_string())]
            } else {
                vec![]
            }),
            fail_cook: if by_path { 2 } else { 1 },
            cooks: Mutex::new(0),
        };
        let stage = match operation {
            BeadOperation::PreviewPour => BeadStage::PreviewPour,
            BeadOperation::Pour => BeadStage::Pour,
            _ => BeadStage::Validate,
        };
        assert_fuzz_055_cook_failure(&w, &runner, stage, if by_path { 2 } else { 1 });
    }
}

fn assert_fuzz_055_cook_failure(
    w: &Workspace,
    runner: &CookFailureSourceRunner,
    stage: BeadStage,
    cook_count: usize,
) {
    let receipt = execute_bead_request_with_runner(&w.req, runner).expect("actual execute receipt");
    failed(&receipt, "BEADS_COOK_FAILED", stage);
    assert_eq!(
        receipt.rendered_formula,
        PathBuf::from(public_path(&w.req.rendered_formula))
    );
    let evidence = serde_json::to_string(&receipt).expect("receipt JSON");
    let source = public_path(&w.req.rendered_formula);
    let encoded_source = serde_json::to_string(&source).expect("encoded source");
    assert!(
        evidence.contains(encoded_source.trim_matches('"')),
        "{evidence}"
    );
    assert!(!evidence.contains(".sc-compose-input-"), "{evidence}");
    let diagnostic = &receipt
        .stages
        .last()
        .expect("failed cook stage")
        .stderr_excerpt;
    assert!(diagnostic.contains(source.as_str()), "{diagnostic}");
    assert!(diagnostic.contains("cannot cook source"), "{diagnostic}");
    assert!(!diagnostic.contains(".sc-compose-input-"), "{diagnostic}");
    let calls = runner.inner.calls();
    assert_eq!(
        calls.iter().filter(|call| call.args[0] == "cook").count(),
        cook_count
    );
    for call in calls.iter().filter(|call| call.args[0] == "cook") {
        assert!(call.args[1].contains(".sc-compose-input-"));
        assert!(
            !PathBuf::from(&call.args[1]).exists(),
            "private input cleaned up"
        );
    }
    assert!(
        !calls
            .iter()
            .any(|call| matches!(call.args[0].as_str(), "create" | "mol"))
    );
}

struct GraphPlanSourceRunner {
    inner: FakeRunner,
    fail_create: bool,
}

impl ProcessRunner for GraphPlanSourceRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        if spec.args[0] == "create" {
            let index = spec
                .args
                .iter()
                .position(|arg| arg == "--graph")
                .expect("graph")
                + 1;
            let private = &spec.args[index];
            assert!(private.contains(".sc-compose-input-"), "{spec:#?}");
            let _: Value = serde_json::from_slice(&fs::read(private)?).expect("own live plan");
            self.inner.calls.lock().expect("calls").push(spec.clone());
            return Ok(ProcessOutput {
                stderr: if self.fail_create {
                    format!("cannot apply graph {private}")
                } else {
                    String::new()
                },
                ..out(
                    Some(if self.fail_create { 7 } else { 0 }),
                    &format!("graph input {private}"),
                )
            });
        }
        self.inner.run(spec)
    }
}

#[test]
fn fuzz_055_r3_create_failures_and_preview_receipts_name_public_graph_plan() {
    for operation in [
        BeadOperation::PreviewPour,
        BeadOperation::Pour,
        BeadOperation::PreviewAttach,
        BeadOperation::Attach,
    ] {
        for fail_create in [true, false] {
            if !fail_create && matches!(operation, BeadOperation::Pour | BeadOperation::Attach) {
                continue;
            }
            let mut w = Workspace::new();
            w.req.operation = operation;
            let by_path = matches!(operation, BeadOperation::PreviewPour | BeadOperation::Pour);
            let registry = w.root.join(".beads");
            fs::create_dir(&registry).expect("registry");
            let outputs = if by_path {
                w.req.parent = None;
                w.req.ref_ = None;
                vec![
                    ok(COOKED),
                    ok(&json!({"path":registry}).to_string()),
                    ok(COOKED),
                ]
            } else {
                vec![ok(COOKED), parent()]
            };
            let runner = GraphPlanSourceRunner {
                inner: FakeRunner::new(outputs),
                fail_create,
            };
            let receipt =
                execute_bead_request_with_runner(&w.req, &runner).expect("actual execution");
            if fail_create {
                assert_eq!(
                    receipt.outcome,
                    BeadOutcome::Failed {
                        code: "BEADS_GRAPH_APPLY_FAILED".into()
                    }
                );
                let message = &receipt.stages.last().expect("create stage").stderr_excerpt;
                assert!(message.contains("cannot apply graph"), "{message}");
            } else {
                assert_eq!(receipt.outcome, BeadOutcome::Succeeded);
            }
            let public = w.req.rendered_formula.with_extension("toml.graph.json");
            let public_display = public_path(&public);
            let wire = serde_json::to_string(&receipt).expect("receipt");
            let encoded_public = serde_json::to_string(&public_display).expect("encoded path");
            assert!(wire.contains(encoded_public.trim_matches('"')), "{wire}");
            assert!(!wire.contains(".sc-compose-input-"), "{wire}");
            let create = receipt
                .stages
                .iter()
                .find(|stage| stage.argv.iter().any(|arg| arg == "--graph"))
                .expect("create");
            assert!(create.argv.iter().any(|arg| arg == &public_display));
            let calls = runner.inner.calls();
            let actual = calls
                .iter()
                .find(|call| call.args[0] == "create")
                .expect("actual create");
            assert!(
                !PathBuf::from(&actual.args[2]).exists(),
                "private plan cleaned"
            );
        }
    }
}
