//! Production bd graph use cases from ADR-0023; opt in with `BD_EXECUTABLE`.
use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeReceipt, BeadComposeRequest, BeadExecutionOptions, BeadGraphMode,
    BeadId, BeadNodeAction, BeadOperation, BeadOutcome, BeadPourMode, BeadRenderBy, BeadStage,
    CommandSpec, GraphRef, PourAuthorization, ProcessOutput, ProcessRunner, StdProcessRunner,
    execute_bead_request_with_options,
};
#[path = "../../test-support/bounded_command.rs"]
mod bounded_command;
use bounded_command::{drain_then_wait, run_command, wait_for_completion};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());
const PIPE_DRAIN_TEST_CHILD_ENV: &str = "SC_COMPOSER_GRAPH_BD_PIPE_DRAIN_CHILD";
const PIPE_DRAIN_TEST_OUTPUT_BYTES: usize = 256 * 1024;
const WORKSPACE_CHILD_ENV: &str = "SC_COMPOSER_GRAPH_BD_WORKSPACE_CHILD";
const FORMULA: &str = "formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"Build {{literal}}\"\n[[steps]]\nid = \"verify\"\ntitle = \"Verify\"\nneeds = [\"build\"]\n[[steps]]\nid = \"publish\"\ntitle = \"Publish\"\nneeds = [\"verify\"]\n";
// Go/Dolt interprets a Windows verbatim prefix as a network path. Rust can
// keep canonical paths internally; only the bd environment needs this spelling.
fn bd_environment_directory(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{unc}"))
    } else if let Some(drive) = text.strip_prefix(r"\\?\") {
        PathBuf::from(drive)
    } else {
        path.to_path_buf()
    }
}

#[test]
fn bd_environment_directory_uses_non_verbatim_fixture_paths() {
    assert_eq!(
        bd_environment_directory(Path::new(r"\\?\C:\temp\fixture\.beads")),
        PathBuf::from(r"C:\temp\fixture\.beads")
    );
    assert_eq!(
        bd_environment_directory(Path::new(r"\\?\UNC\server\share\fixture\.beads")),
        PathBuf::from(r"\\server\share\fixture\.beads")
    );
    assert_eq!(
        bd_environment_directory(Path::new(r"C:\temp\fixture\.beads")),
        PathBuf::from(r"C:\temp\fixture\.beads")
    );
    assert_eq!(
        bd_environment_directory(Path::new("/workspace/.beads")),
        PathBuf::from("/workspace/.beads")
    );
}

struct Workspace {
    root: PathBuf,
    beads_dir: PathBuf,
    bd: PathBuf,
    _temp_dir: Option<tempfile::TempDir>,
}
impl Workspace {
    fn new(binary: &Path) -> Self {
        let temp_dir = tempfile::Builder::new()
            .prefix("sc-graph-bd-")
            .tempdir()
            .expect("workspace tempdir");
        let root = temp_dir.path().to_path_buf();
        fs::create_dir_all(root.join("build")).expect("workspace");
        let root = fs::canonicalize(root).expect("canonical root");
        let beads_dir = root.join(".beads");
        fs::create_dir_all(root.join(".home")).expect("isolated home");
        let ws = Self {
            root,
            beads_dir,
            bd: binary.to_path_buf(),
            _temp_dir: Some(temp_dir),
        };
        let mut init = vec![
            "init".to_owned(),
            "--non-interactive".into(),
            "--quiet".into(),
            "--skip-agents".into(),
            "--skip-hooks".into(),
            "--prefix".into(),
            "graph".into(),
        ];
        // A bd built without embedded Dolt needs a server. Point it at a
        // dedicated throwaway server, never a shared one, with a database
        // name unique to this workspace.
        if let Some(port) = std::env::var_os("SC_COMPOSE_TEST_DOLT_PORT") {
            fs::create_dir_all(&ws.beads_dir).expect("beads dir");
            let workspace_id = ws
                .root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("fixture")
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect::<String>();
            let database = format!("scg_{workspace_id}");
            init.extend([
                "--server".into(),
                "--server-port".into(),
                port.to_string_lossy().into_owned(),
                "--database".into(),
                database,
            ]);
        }
        let init: Vec<&str> = init.iter().map(String::as_str).collect();
        ws.command(&init);
        fs::write(ws.root.join("release.formula.toml.j2"), FORMULA).expect("template");
        ws
    }
    fn command(&self, args: &[&str]) -> String {
        let mut command = Command::new(&self.bd);
        command
            .args(args)
            .current_dir(&self.root)
            .env("BEADS_NO_DAEMON", "1")
            .env("HOME", self.root.join(".home"))
            .env("XDG_CONFIG_HOME", self.root.join(".home/.config"))
            .env("USERPROFILE", self.root.join(".home"))
            .env("BEADS_DIR", bd_environment_directory(&self.beads_dir));
        let out = run_command(&mut command);
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).expect("UTF-8")
    }
    fn json(&self, args: &[&str]) -> Value {
        serde_json::from_str(&self.command(args)).expect("bd JSON")
    }
    fn parent(&self) -> BeadId {
        BeadId::new(
            self.json(&["create", "Parent", "--type", "epic", "--json"])["id"]
                .as_str()
                .expect("created id"),
        )
        .expect("id")
    }
    fn request(&self, operation: BeadOperation, parent: Option<BeadId>) -> BeadComposeRequest {
        BeadComposeRequest {
            schema: BEADS_SCHEMA_V1.into(),
            operation,
            working_directory: self.root.clone(),
            template: self.root.join("release.formula.toml.j2"),
            rendered_formula: self.root.join("build/release.formula.toml"),
            compose_variables: serde_json::Map::new(),
            formula_name: Some(
                sc_composer_beads::FormulaName::new("release").expect("formula name"),
            ),
            bead_variables: BTreeMap::new(),
            bd_executable: Some(self.bd.clone()),
            pour_authorization: Some(PourAuthorization::CreatePersistentBeads),
            ref_: parent
                .as_ref()
                .map(|_| GraphRef::new("release").expect("ref")),
            parent,
            relations: Vec::new(),
        }
    }
    fn run(&self, req: &BeadComposeRequest) -> BeadComposeReceipt {
        assert_eq!(req.working_directory, self.root);
        if req.operation == BeadOperation::Repour {
            let options = sc_composer_beads::GraphRevisionOptions::new(
                sc_composer_beads::ExistingGraphTarget::PourRoot(req.parent.clone().expect("root")),
                sc_composer_beads::UncommittedPolicy::Allow,
            );
            return sc_composer_beads::execute_graph_revision(req, &options)
                .expect("valid revision");
        }
        execute_bead_request_with_options(
            req,
            &BeadExecutionOptions {
                render_by: BeadRenderBy::Bd,
                strict: false,
            },
        )
        .expect("valid request")
    }
    fn success(&self, req: &BeadComposeRequest) -> BeadComposeReceipt {
        let receipt = self.run(req);
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
        receipt
    }
    fn snapshot(&self) -> Value {
        let beads = self.json(&["list", "--all", "-n", "0", "--json"]);
        let mut edges = BTreeMap::new();
        for bead in beads.as_array().expect("beads") {
            let id = bead["id"].as_str().expect("bead id");
            edges.insert(id, self.json(&["dep", "list", id, "--json"]));
        }
        json!({"beads": beads, "edges": edges})
    }
    fn refuse(&self, req: &BeadComposeRequest, code: &str) -> BeadComposeReceipt {
        let before = self.snapshot();
        let result = self.run(req);
        assert_eq!(
            result.outcome,
            BeadOutcome::Refused { code: code.into() },
            "{result:#?}"
        );
        assert_eq!(self.snapshot(), before, "refusal changes no beads or edges");
        result
    }
}

fn bead_id(value: &str) -> BeadId {
    BeadId::new(value).expect("bead id")
}

fn step_id(value: &str) -> sc_composer_beads::StepId {
    sc_composer_beads::StepId::new(value).expect("step id")
}

fn assert_dependency(w: &Workspace, from: &BeadId, to: &BeadId, kind: &str) {
    let dependencies = w.json(&["dep", "list", from.as_str(), "--json"]);
    assert!(
        dependencies
            .as_array()
            .expect("dependency rows")
            .iter()
            .any(|row| { row["id"] == to.as_str() && row["dependency_type"] == kind }),
        "expected {from} -> {to} ({kind}), got {dependencies:#}"
    );
}

fn assert_persisted_graph(w: &Workspace, graph: &sc_composer_beads::BeadGraph) {
    let parent = graph.parent.as_ref().expect("graph parent");
    let children = w.json(&["children", parent.as_str(), "--json"]);
    let actual = children
        .as_array()
        .expect("children")
        .iter()
        .map(|child| child["id"].as_str().expect("child id"))
        .collect::<BTreeSet<_>>();
    let expected = graph
        .ids
        .values()
        .map(BeadId::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "children are exactly the graph steps");
    for id in graph.ids.values() {
        assert_dependency(w, id, parent, "parent-child");
    }
    let build = &graph.ids[&step_id("build")];
    let verify = &graph.ids[&step_id("verify")];
    let publish = &graph.ids[&step_id("publish")];
    assert_dependency(w, verify, build, "blocks");
    assert_dependency(w, publish, verify, "blocks");
}

#[test]
fn child_failure_deadline_uses_controlled_completion_and_expiry() {
    use std::cell::Cell;
    // Already-completed output succeeds even when the guard would be expired.
    let ready = wait_for_completion(
        || Ok(Some(7)),
        || panic!("ready checked first"),
        || panic!("no pause"),
    );
    assert_eq!(ready.unwrap(), Some(7));
    let polls = Cell::new(0);
    let expired = wait_for_completion(
        || {
            polls.set(polls.get() + 1);
            Ok(None::<u8>)
        },
        || true,
        || panic!("expired child must fail immediately"),
    );
    assert_eq!(expired.unwrap(), None);
    assert_eq!(polls.get(), 1);
    let mut results = [None, Some(9)].into_iter();
    let pauses = Cell::new(0);
    let completed = wait_for_completion(
        || Ok(results.next().unwrap()),
        || false,
        || pauses.set(pauses.get() + 1),
    );
    assert_eq!(completed.unwrap(), Some(9));
    assert_eq!(pauses.get(), 1);
    let error = wait_for_completion(
        || Err::<Option<u8>, _>(std::io::Error::other("try_wait failed")),
        || false,
        || panic!("no pause"),
    );
    assert_eq!(error.unwrap_err().to_string(), "try_wait failed");
}

#[test]
fn pipe_drain_child_emits_large_stdout_and_stderr() {
    if std::env::var_os(PIPE_DRAIN_TEST_CHILD_ENV).is_none() {
        return;
    }
    let stdout = vec![b'o'; PIPE_DRAIN_TEST_OUTPUT_BYTES];
    let stderr = vec![b'e'; PIPE_DRAIN_TEST_OUTPUT_BYTES];
    let mut out = std::io::stdout().lock();
    out.write_all(b"<stdout>").expect("write stdout marker");
    out.write_all(&stdout).expect("write test stdout");
    out.write_all(b"</stdout>").expect("write stdout marker");
    std::io::stderr()
        .lock()
        .write_all(&stderr)
        .expect("write test stderr");
}

#[test]
fn bd_command_drains_both_pipes_before_waiting_for_exit() {
    let executable = std::env::current_exe().expect("test executable");
    let mut command = Command::new(executable);
    command
        .args([
            "--exact",
            "pipe_drain_child_emits_large_stdout_and_stderr",
            "--nocapture",
        ])
        .env(PIPE_DRAIN_TEST_CHILD_ENV, "1");

    let output = run_command(&mut command);

    assert!(output.status.success(), "{output:?}");
    let mut expected_stdout = b"<stdout>".to_vec();
    expected_stdout.extend(vec![b'o'; PIPE_DRAIN_TEST_OUTPUT_BYTES]);
    expected_stdout.extend(b"</stdout>");
    assert_eq!(
        output
            .stdout
            .windows(expected_stdout.len())
            .filter(|window| *window == expected_stdout)
            .count(),
        1
    );
    assert_eq!(output.stderr, vec![b'e'; PIPE_DRAIN_TEST_OUTPUT_BYTES]);
}

#[test]
fn both_drains_finish_before_wait() {
    use std::cell::Cell;
    let stdout_done = Cell::new(false);
    let stderr_done = Cell::new(false);
    let (stdout, stderr, status) = drain_then_wait(
        || {
            stdout_done.set(true);
            b"out".to_vec()
        },
        || {
            stderr_done.set(true);
            b"err".to_vec()
        },
        || {
            assert!(
                stdout_done.get() && stderr_done.get(),
                "wait before both drains completed"
            );
            7
        },
    );
    assert_eq!(
        (stdout, stderr, status),
        (b"out".to_vec(), b"err".to_vec(), 7)
    );
}

const BODY_COMPLETED: &str = "SC_COMPOSE_REAL_BD_BODY_COMPLETED";

fn isolated_test_command(name: &str) -> Command {
    let mut child = Command::new(std::env::current_exe().expect("test executable"));
    child.args(["--ignored", "--exact", name, "--nocapture"]);
    child
}

#[test]
#[ignore = "selected by child_reexec_runs_an_ignored_body_without_bd"]
fn ignored_child_execution_probe() {
    println!("\n{BODY_COMPLETED}");
}

#[test]
fn child_reexec_runs_an_ignored_body_without_bd() {
    let output = run_command(&mut isolated_test_command("ignored_child_execution_probe"));
    assert!(output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == BODY_COMPLETED),
        "ignored body was skipped: {output:?}"
    );
}

fn with_workspace(test: impl FnOnce(&Workspace)) {
    let Some(binary) = std::env::var_os("BD_EXECUTABLE") else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    if let Some(root) = std::env::var_os(WORKSPACE_CHILD_ENV) {
        let root = PathBuf::from(root);
        let beads_dir = root.join(".beads");
        assert_eq!(
            std::env::var_os("BEADS_DIR"),
            Some(bd_environment_directory(&beads_dir).into())
        );
        assert_eq!(std::env::var("BEADS_NO_DAEMON").as_deref(), Ok("1"));
        assert_eq!(std::env::var_os("HOME"), Some(root.join(".home").into()));
        test(&Workspace {
            root: fs::canonicalize(root).expect("canonical child root"),
            beads_dir,
            bd: PathBuf::from(binary),
            _temp_dir: None,
        });
        println!("\n{BODY_COMPLETED}");
        return;
    }
    assert!(
        std::env::var_os("BEADS_DIR").is_none(),
        "graph integration refuses ambient BEADS_DIR; unset it before running"
    );
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let workspace = Workspace::new(Path::new(&binary));
    let thread = std::thread::current();
    let name = thread.name().expect("libtest test name");
    let mut child = isolated_test_command(name);
    // The child environment isolates both helper commands and engine-launched bd.
    // Use the original temp path: canonical paths have a verbatim prefix on Windows.
    child
        .env(
            WORKSPACE_CHILD_ENV,
            workspace.beads_dir.parent().expect("root"),
        )
        .env("HOME", workspace.root.join(".home"))
        .env("XDG_CONFIG_HOME", workspace.root.join(".home/.config"))
        .env("USERPROFILE", workspace.root.join(".home"))
        .env("BEADS_DIR", bd_environment_directory(&workspace.beads_dir))
        .env("BEADS_NO_DAEMON", "1");
    let output = run_command(&mut child);
    if String::from_utf8_lossy(&output.stdout).contains("SC_COMPOSE_MODE_UNSUPPORTED:") {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    assert!(
        output.status.success(),
        "isolated {name}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == BODY_COMPLETED),
        "isolated {name} exited without executing its body: {output:?}"
    );
}

#[test]
fn graph_integration_refuses_ambient_beads_dir_before_launching_bd() {
    let mut child = Command::new(std::env::current_exe().expect("test executable"));
    child
        .args([
            "--ignored",
            "--exact",
            "uc1_render_and_pour_outside_registry",
            "--nocapture",
        ])
        .env_remove(WORKSPACE_CHILD_ENV)
        .env("BD_EXECUTABLE", "must-not-be-launched")
        .env("BEADS_DIR", "must-not-be-used");
    let output = run_command(&mut child);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("graph integration refuses ambient BEADS_DIR"),
        "{output:?}"
    );
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc1_render_and_pour_outside_registry() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::Render, None);
        w.success(&r);
        let before_preview = w.snapshot();
        let preview = w.success(&w.request(BeadOperation::PreviewPour, None));
        assert_eq!(preview.pour_mode, Some(BeadPourMode::Graph));
        assert!(preview.graph.expect("graph").ids.is_empty());
        assert_eq!(w.snapshot(), before_preview, "preview-pour must not write");
        let applied = w.success(&w.request(BeadOperation::Pour, None));
        let graph = applied.graph.expect("graph");
        assert_eq!(graph.ids.len(), 3);
        assert_eq!(graph.mode, BeadGraphMode::Pour);
        let root = w.json(&[
            "show",
            graph.parent.as_ref().expect("root").as_str(),
            "--json",
        ]);
        assert_eq!(root[0]["issue_type"], "molecule");
        let persisted = w.json(&["list", "--all", "-n", "0", "--json"]);
        let persisted = persisted.as_array().expect("workspace beads");
        assert_eq!(persisted.len(), graph.ids.len() + 1);
        for id in graph.ids.values() {
            assert!(
                persisted.iter().any(|bead| bead["id"] == id.as_str()),
                "engine-created {id} must land in the workspace database"
            );
        }
        assert_persisted_graph(w, &graph);
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn graph_pour_ignores_same_name_toml_and_json_registry_formulas() {
    with_workspace(|w| {
        let registry = w.root.join(".beads/formulas");
        fs::create_dir_all(&registry).expect("registry");
        fs::write(registry.join("release.formula.toml"), FORMULA).expect("registry TOML");
        fs::write(
            registry.join("release.formula.json"),
            r#"{"formula":"release","version":1,"type":"workflow","steps":[{"id":"build","title":"Build"}]}"#,
        )
        .expect("registry JSON");

        let r = w.request(BeadOperation::Render, None);
        w.success(&r);
        let receipt = w.success(&w.request(BeadOperation::Pour, None));
        assert_eq!(receipt.pour_mode, Some(BeadPourMode::Graph));
        assert_persisted_graph(w, &receipt.graph.expect("graph"));
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc2_attach_children_directly_under_parent() {
    with_workspace(|w| {
        let parent = w.parent();
        let r = w.request(BeadOperation::Attach, Some(parent.clone()));
        let g = w.success(&r).graph.expect("graph");
        assert_eq!(g.ids.len(), 3);
        let children = w.json(&["children", parent.as_str(), "--json"]);
        assert_eq!(children.as_array().expect("children").len(), 3);
        assert_eq!(
            g.ids[&step_id("build")].as_str(),
            format!("{parent}.release-build")
        );
        assert_persisted_graph(w, &g);
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc3_preview_creates_no_beads() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::PreviewAttach, Some(w.parent()));
        let before = w.snapshot();
        let g = w.success(&r).graph.expect("graph");
        assert!(g.nodes.iter().all(|n| n.action == BeadNodeAction::Create));
        assert!(g.plan_path.expect("plan").is_file());
        assert_eq!(w.snapshot(), before);
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc4_rerun_preserves_closed_annotated_beads() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::Attach, Some(w.parent()));
        let g = w.success(&r).graph.expect("graph");
        let id = g.ids.values().next().expect("id");
        w.command(&[
            "update",
            id.as_str(),
            "--notes",
            "retained evidence",
            "--assignee",
            "tester",
        ]);
        w.command(&["close", id.as_str(), "--actor", "tester"]);
        let before = w.snapshot();
        let plan = g.plan_path.expect("original plan");
        let bytes = fs::read(&plan).expect("plan bytes");
        let next = w.success(&r).graph.expect("graph");
        assert!(next.plan_path.is_none());
        assert!(
            next.nodes
                .iter()
                .all(|n| n.action == BeadNodeAction::Existing)
        );
        assert_eq!(w.snapshot(), before);
        assert_eq!(fs::read(plan).expect("unchanged plan"), bytes);
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc5_resume_recreates_only_missing_child() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::Attach, Some(w.parent()));
        let g = w.success(&r).graph.expect("graph");
        let id = g.ids[&step_id("publish")].clone();
        w.command(&["delete", id.as_str(), "--force"]);
        let next = w.success(&r).graph.expect("graph");
        assert_eq!(
            next.nodes
                .iter()
                .filter(|n| n.action == BeadNodeAction::Created)
                .count(),
            1
        );
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc6_changed_revision_scope_and_relations_are_refused() {
    with_workspace(|w| {
        let mut r = w.request(BeadOperation::Attach, Some(w.parent()));
        w.success(&r);
        fs::write(&r.template, FORMULA.replace("Verify", "Verify changed"))
            .expect("changed formula");
        w.refuse(&r, "BEADS_GRAPH_CONFLICT");
        fs::write(&r.template, FORMULA).expect("restore template");
        r.compose_variables.insert("ref".into(), json!("other"));
        w.refuse(&r, "BEADS_GRAPH_SCOPE_MISMATCH");
        r.compose_variables.clear();
        let external = w.parent();
        r.relations = serde_json::from_value(
            json!([{"from":"step:build","to":format!("bead:{external}"),"type":"related"}]),
        )
        .expect("relation");
        w.refuse(&r, "BEADS_GRAPH_CONFLICT");
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc7_relations_in_both_directions_preserve_external_fields() {
    with_workspace(|w| {
        let mut r = w.request(BeadOperation::Attach, Some(w.parent()));
        let external = w.parent();
        r.relations=serde_json::from_value(json!([{"from":"step:build","to":format!("bead:{external}"),"type":"related"},{"from":format!("bead:{external}"),"to":"step:verify","type":"validates"}])).expect("relations");
        w.command(&[
            "update",
            external.as_str(),
            "--notes",
            "unchanged",
            "--assignee",
            "tester",
        ]);
        let before = w.json(&["show", external.as_str(), "--json"]);
        w.success(&r);
        let after = w.json(&["show", external.as_str(), "--json"]);
        for field in ["title", "status", "notes", "assignee"] {
            assert_eq!(before[0][field], after[0][field]);
        }
        w.success(&r);
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc8_template_loop_and_hyphenated_ref() {
    with_workspace(|w| {
        let mut r = w.request(BeadOperation::Attach, Some(w.parent()));
        r.ref_ = Some(GraphRef::new("qa1-f1-r1").expect("ref"));
        fs::write(&r.template,"formula = \"batch\"\nversion = 1\ntype = \"workflow\"\n{% for i in range(1, 11) %}\n[[steps]]\nid = \"item_{{{ i }}}\"\ntitle = \"Item {{{ i }}}\"\n{% if i > 1 %}needs = [\"item_{{{ i - 1 }}}\"]\n{% endif %}{% endfor %}").expect("loop template");
        let g = w.success(&r).graph.expect("graph");
        assert_eq!(g.ids.len(), 10);
        assert!(
            g.ids
                .values()
                .all(|id| id.as_str().contains(".qa1-f1-r1-item_"))
        );
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc9_attach_one_workflow_under_three_parents() {
    with_workspace(|w| {
        let parents = [w.parent(), w.parent(), w.parent()];
        for parent in &parents {
            w.success(&w.request(BeadOperation::Attach, Some(parent.clone())));
        }
        let before = w.snapshot();
        for parent in parents {
            w.success(&w.request(BeadOperation::Attach, Some(parent)));
        }
        assert_eq!(w.snapshot(), before);
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn uc10_receipt_maps_every_generated_id_and_parent() {
    with_workspace(|w| {
        let g = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .expect("graph");
        for (step, id) in &g.ids {
            let row = w.json(&["show", id.as_str(), "--json"]);
            assert_eq!(
                row[0]["metadata"]["sc_compose_graph"]["step"],
                step.as_str()
            );
            assert_eq!(
                row[0]["parent"],
                g.parent.as_ref().expect("parent").as_str()
            );
        }
    });
}
#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn inherited_steps_attach_and_bd_loop_is_refused() {
    with_workspace(|w| {
        fs::create_dir_all(w.root.join(".beads/formulas")).expect("registry");
        fs::write(
            w.root.join(".beads/formulas/base.formula.json"),
            include_str!("fixtures/beads/graph/base.formula.json"),
        )
        .expect("base");
        let r = w.request(BeadOperation::Attach, Some(w.parent()));
        fs::write(&r.template,"formula = \"release\"\nversion = 1\ntype = \"workflow\"\nextends = [\"base\"]\n[[steps]]\nid = \"build\"\ntitle = \"Build\"\n").expect("extends");
        assert_eq!(w.success(&r).graph.expect("graph").ids.len(), 2);
        fs::write(&r.template,"formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"loop\"\ntitle = \"Loop\"\n[steps.loop]\ncount = 2\n[[steps.loop.body]]\nid = \"check\"\ntitle = \"Check\"\n").expect("loop");
        w.refuse(&r, "BEADS_GRAPH_ID_INVALID");
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn refusals_preserve_beads_and_edges() {
    with_workspace(|w| {
        let parent = w.parent();
        let mut r = w.request(BeadOperation::Attach, Some(parent.clone()));
        r.parent = Some(BeadId::new("graph-missing").expect("id"));
        w.refuse(&r, "BEADS_GRAPH_PARENT_NOT_FOUND");
        r.parent = Some(parent);
        r.relations = serde_json::from_value(json!([
            {"from":"step:build","to":"step:build","type":"blocks"}
        ]))
        .expect("relation");
        w.refuse(&r, "BEADS_GRAPH_RELATION_INVALID");
        r.relations = serde_json::from_value(json!([
            {"from":"step:build","to":"bead:graph-missing","type":"related"}
        ]))
        .expect("missing bead relation");
        let receipt = w.refuse(&r, "BEADS_GRAPH_RELATION_INVALID");
        let stage = receipt.stages.last().expect("plan-stage receipt");
        assert_eq!(stage.stage, BeadStage::Attach);
        assert!(
            stage.stderr_excerpt.contains("graph-missing")
                && stage.stderr_excerpt.contains("not found"),
            "missing bead relation must fail at planning: {stage:#?}"
        );
        r.relations.clear();
        fs::write(
            &r.template,
            FORMULA.replace(
                "version = 1",
                "version = 1\n[vars.release]\ndefault = \"v1\"",
            ),
        )
        .expect("unsupported formula");
        w.refuse(&r, "BEADS_GRAPH_FORMULA_UNSUPPORTED");
        fs::write(&r.template, FORMULA).expect("restore formula");
        let graph = w.success(&r).graph.expect("graph");
        let build = graph.ids[&step_id("build")].as_str();
        let verify = graph.ids[&step_id("verify")].as_str();
        w.command(&["dep", "remove", verify, build]);
        w.refuse(&r, "BEADS_GRAPH_EDGE_MISSING");
        w.command(&["dep", "add", verify, build, "--type", "related"]);
        w.refuse(&r, "BEADS_GRAPH_EDGE_CONFLICT");
        w.command(&[
            "update",
            build,
            "--metadata",
            r#"{"sc_compose_graph":null}"#,
        ]);
        w.refuse(&r, "BEADS_GRAPH_CONFLICT");
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn real_bd_advanced_transport_preserves_fields_and_replay_state() {
    with_workspace(|w| {
        let template = w.root.join("release.formula.toml.j2");
        fs::write(&template, "formula = \"release\"\nversion = 1\ntype = \"workflow\"\n{% for item in items %}[[steps]]\nid = \"{{ item }}\"\ntitle = \"{{ item }} {{ literal }}\"\ndescription = \"{{ value | lower }}:{{ value | length }}:{{ value[:3] }}\"\nlabels = [\"label-{{ item }}\"]\n{% endfor %}").unwrap();
        let parent = w.parent();
        let mut request = w.request(BeadOperation::Attach, Some(parent));
        request
            .compose_variables
            .insert("items".into(), json!(["one", "two"]));
        request
            .bead_variables
            .insert("literal".into(), "{{ data }}".into());
        request.bead_variables.insert("value".into(), "A{BC".into());
        let receipt = sc_composer_beads::execute_bead_request(&request).expect("advanced request");
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
        assert!(
            receipt
                .stages
                .iter()
                .all(|stage| !stage.argv.iter().any(|arg| arg == "--var"))
        );
        let graph = receipt.graph.expect("graph");
        for name in ["one", "two"] {
            let id = &graph.ids[&step_id(name)];
            let shown = w.json(&["show", id.as_str(), "--json"]);
            assert_eq!(shown[0]["title"], format!("{name} {}", "{{ data }}"));
            assert_eq!(shown[0]["description"], "a{bc:4:A{B");
            assert!(
                shown[0]["labels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|label| label == &format!("label-{name}"))
            );
        }
        let first = graph.ids[&step_id("one")].as_str();
        w.command(&[
            "update",
            first,
            "--status",
            "in_progress",
            "--notes",
            "keep this",
            "--assignee",
            "reviewer",
        ]);
        let before = w.snapshot();
        let second_staging = w.root.join("second-source-snapshot");
        fs::create_dir(&second_staging).unwrap();
        let second_template = second_staging.join("release.formula.toml.j2");
        fs::copy(&template, &second_template).unwrap();
        request.template = second_template;
        let replay = sc_composer_beads::execute_bead_request(&request).unwrap();
        assert_eq!(replay.outcome, BeadOutcome::Succeeded, "{replay:#?}");
        assert_eq!(
            w.snapshot(),
            before,
            "replay must preserve all stored state"
        );
        let shown = w.json(&["show", first, "--json"]);
        assert_eq!(shown[0]["status"], "in_progress");
        assert_eq!(shown[0]["notes"], "keep this");
        assert_eq!(shown[0]["assignee"], "reviewer");
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn advanced_conditional_nodes_preview_without_pruning_or_unqualified_apply() {
    with_workspace(|w| {
        let template = w.root.join("release.formula.toml.j2");
        fs::write(&template, "formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"Build\"\n{% if enabled %}[[steps]]\nid = \"verify\"\ntitle = \"Verify\"\nneeds = [\"build\"]\n{% endif %}").unwrap();
        let parent = w.parent();
        let mut request = w.request(BeadOperation::Attach, Some(parent));
        request
            .compose_variables
            .insert("enabled".into(), json!(false));
        let initial = sc_composer_beads::execute_bead_request(&request).unwrap();
        assert_eq!(initial.outcome, BeadOutcome::Succeeded);
        assert_eq!(initial.graph.unwrap().ids.len(), 1);
        let before = w.snapshot();
        request
            .compose_variables
            .insert("enabled".into(), json!(true));
        request.operation = BeadOperation::PreviewAttach;
        let preview = sc_composer_beads::execute_bead_request(&request).unwrap();
        assert_eq!(preview.outcome, BeadOutcome::Succeeded, "{preview:#?}");
        assert!(preview.graph.unwrap().nodes.iter().any(|node| {
            node.step
                .as_ref()
                .is_some_and(|step| step.as_str() == "verify")
                && node.action == BeadNodeAction::Create
        }));
        assert_eq!(w.snapshot(), before);
        request.operation = BeadOperation::Attach;
        let applied = sc_composer_beads::execute_bead_request(&request).unwrap();
        assert_eq!(applied.outcome, BeadOutcome::Succeeded, "{applied:#?}");
        assert_eq!(applied.graph.unwrap().ids.len(), 2);
        assert_ne!(w.snapshot(), before, "the optional node is now created");

        let parent = w.parent();
        request.parent = Some(parent);
        let initial_enabled = sc_composer_beads::execute_bead_request(&request).unwrap();
        assert_eq!(initial_enabled.outcome, BeadOutcome::Succeeded);
        assert_eq!(initial_enabled.graph.unwrap().ids.len(), 2);
        let before_disabled = w.snapshot();
        request
            .compose_variables
            .insert("enabled".into(), json!(false));
        let disabled = sc_composer_beads::execute_bead_request(&request).unwrap();
        assert_eq!(disabled.outcome, BeadOutcome::Succeeded, "{disabled:#?}");
        assert_eq!(
            w.snapshot(),
            before_disabled,
            "disabled nodes are never pruned"
        );
    });
}

/// The `release` plan with a configurable build title and extra steps.
fn plan(build_title: &str, extra: &str) -> String {
    format!(
        "formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"{build_title}\"\n[[steps]]\nid = \"verify\"\ntitle = \"Verify\"\nneeds = [\"build\"]\n[[steps]]\nid = \"publish\"\ntitle = \"Publish\"\nneeds = [\"verify\"]\n{extra}"
    )
}

impl Workspace {
    fn set_plan(&self, text: &str) {
        fs::write(self.root.join("release.formula.toml.j2"), text).expect("plan");
    }

    fn repour_request(&self, root: &BeadId) -> BeadComposeRequest {
        let mut request = self.request(BeadOperation::Repour, Some(root.clone()));
        request.ref_ = None;
        request
    }

    fn bead_count(&self) -> usize {
        self.json(&["list", "--all", "-n", "0", "--json"])
            .as_array()
            .expect("beads")
            .len()
    }

    fn leftovers(&self) -> Vec<String> {
        let mut names = Vec::new();
        for dir in [self.root.clone(), self.root.join("build")] {
            for entry in fs::read_dir(dir).expect("dir") {
                let name = entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned();
                if name.contains("backup") || name.contains(".sc-compose-input") {
                    names.push(name);
                }
            }
        }
        names
    }

    fn dependency_exists(&self, from: &BeadId, to: &BeadId) -> bool {
        self.json(&["dep", "list", from.as_str(), "--json"])
            .as_array()
            .expect("deps")
            .iter()
            .any(|dep| dep["id"] == to.as_str() && dep["dependency_type"] == "blocks")
    }
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn repour_converges_three_edits_on_one_store_without_deleting_or_repouring() {
    with_workspace(|w| {
        let poured = w.success(&w.request(BeadOperation::Pour, None));
        let graph = poured.graph.expect("graph");
        let root = graph.parent.clone().expect("root");
        let ids: BTreeMap<String, String> = graph
            .ids
            .iter()
            .map(|(step, id)| (step.to_string(), id.to_string()))
            .collect();
        let (build, verify) = (ids["build"].clone(), ids["verify"].clone());
        w.command(&[
            "update",
            &build,
            "--status",
            "in_progress",
            "--notes",
            "keep this",
            "--assignee",
            "reviewer",
        ]);
        let beads = w.bead_count();
        assert_eq!(beads, 4);

        // 1. Change a field: same beads, same ids, runtime state intact.
        w.set_plan(&plan("Build changed", ""));
        let receipt = w.success(&w.repour_request(&root));
        assert!(receipt.graph.expect("graph").nodes.iter().any(|node| {
            node.step.as_ref().is_some_and(|s| s.as_str() == "build")
                && node.action == BeadNodeAction::Updated
        }));
        assert_eq!(w.bead_count(), beads, "a field-only repour keeps the count");
        let shown = w.json(&["show", &build, "--json"]);
        assert_eq!(shown[0]["title"], "Build changed");
        assert_eq!(shown[0]["status"], "in_progress");
        assert_eq!(shown[0]["notes"], "keep this");
        assert_eq!(shown[0]["assignee"], "reviewer");

        // 2. Add a node.
        w.set_plan(&plan(
            "Build changed",
            "[[steps]]\nid = \"ship\"\ntitle = \"Ship\"\nneeds = [\"publish\"]\n",
        ));
        let receipt = w.success(&w.repour_request(&root));
        let ids_now = receipt.graph.expect("graph").ids;
        assert_eq!(w.bead_count(), beads + 1);
        for (step, id) in &graph.ids {
            assert_eq!(&ids_now[step], id, "existing ids are stable");
        }
        let ship = ids_now[&step_id("ship")].to_string();
        assert!(w.dependency_exists(&bead_id(&ship), &bead_id(&ids["publish"])));

        // 3. Add an edge between two existing beads.
        w.set_plan(&plan(
            "Build changed",
            "[[steps]]\nid = \"ship\"\ntitle = \"Ship\"\nneeds = [\"publish\", \"build\"]\n",
        ));
        assert!(!w.dependency_exists(&bead_id(&ship), &bead_id(&build)));
        let receipt = w.success(&w.repour_request(&root));
        assert!(
            receipt
                .graph
                .expect("graph")
                .edges
                .iter()
                .any(|edge| edge.action == sc_composer_beads::BeadEdgeAction::Added)
        );
        assert!(w.dependency_exists(&bead_id(&ship), &bead_id(&build)));
        assert_eq!(w.bead_count(), beads + 1, "nothing was poured fresh");
        assert!(w.dependency_exists(&bead_id(&verify), &bead_id(&build)));

        // Independent read-back of the final graph and the preserved state.
        let shown = w.json(&["show", &build, "--json"]);
        assert_eq!(shown[0]["status"], "in_progress");
        assert_eq!(shown[0]["notes"], "keep this");
        assert_eq!(shown[0]["assignee"], "reviewer");
        let children = w.json(&["children", root.as_str(), "--json"]);
        assert_eq!(children.as_array().expect("children").len(), 4);
        assert!(w.leftovers().is_empty(), "{:?}", w.leftovers());

        // An unchanged plan is a no-op.
        let before = w.snapshot();
        w.success(&w.repour_request(&root));
        assert_eq!(w.snapshot(), before);
    });
}

/// Fails the Nth mutating bd call; every other call runs against real bd.
struct FailNthMutation {
    // ProcessRunner::run takes &self, so its counter needs interior mutability.
    seen: Mutex<usize>,
    fail_at: usize,
}

impl ProcessRunner for FailNthMutation {
    fn run(&self, spec: &CommandSpec) -> std::io::Result<ProcessOutput> {
        let mutating = matches!(spec.args.first().map(String::as_str), Some("update"))
            || spec.args.starts_with(&["dep".into(), "add".into()])
            || (spec.args.first().map(String::as_str) == Some("create")
                && !spec.args.iter().any(|arg| arg == "--dry-run"));
        if mutating {
            let mut seen = self.seen.lock().expect("counter");
            *seen += 1;
            if *seen == self.fail_at {
                return Ok(ProcessOutput {
                    exit_status: Some(1),
                    stdout: String::new(),
                    stderr: "injected failure".into(),
                    elapsed: std::time::Duration::ZERO,
                });
            }
        }
        StdProcessRunner.run(spec)
    }
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn repour_interrupted_at_a_controlled_boundary_completes_on_rerun() {
    with_workspace(|w| {
        let graph = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .expect("graph");
        let root = graph.parent.expect("root");
        w.set_plan(&plan(
            "Build changed",
            "[[steps]]\nid = \"ship\"\ntitle = \"Ship\"\nneeds = [\"publish\", \"build\"]\n",
        ));
        let request = w.repour_request(&root);
        let options = sc_composer_beads::GraphRevisionOptions::new(
            sc_composer_beads::ExistingGraphTarget::PourRoot(root.clone()),
            sc_composer_beads::UncommittedPolicy::Allow,
        );
        let runner = FailNthMutation {
            seen: Mutex::new(0),
            fail_at: 2,
        };
        let error =
            sc_composer_beads::execute_graph_revision_with_runner(&request, &options, &runner)
                .unwrap_err();
        assert_eq!(error.code(), "BEADS_GRAPH_APPLY_PARTIAL");
        assert_eq!(w.bead_count(), 5, "the create before the cut was applied");
        assert!(w.leftovers().is_empty());

        let resumed = w.success(&request);
        assert!(resumed.graph.is_some());
        assert_eq!(w.bead_count(), 5, "the rerun does not create a second ship");
        let build = graph.ids[&step_id("build")].clone();
        assert_eq!(
            w.json(&["show", build.as_str(), "--json"])[0]["title"],
            "Build changed"
        );
        let after = w.snapshot();
        w.success(&request);
        assert_eq!(w.snapshot(), after, "a second rerun changes nothing");
        assert!(w.leftovers().is_empty());
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn repour_refuses_a_dangling_reference_and_a_conflicting_edge_with_zero_writes() {
    with_workspace(|w| {
        let graph = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .expect("graph");
        let root = graph.parent.clone().expect("root");
        let ids = graph.ids;
        let build = ids[&step_id("build")].to_string();
        let verify = ids[&step_id("verify")].to_string();
        let before = w.snapshot();

        // Dangling reference: a relation to a bead that does not exist.
        w.set_plan(&plan("Build changed", ""));
        let mut dangling = w.repour_request(&root);
        dangling.relations = vec![sc_composer_beads::BeadRelation {
            from: sc_composer_beads::BeadEndpoint::Step(step_id("build")),
            to: sc_composer_beads::BeadEndpoint::Bead(BeadId::new("graph-nope").expect("id")),
            kind: sc_composer_beads::BeadDependencyType::Blocks,
        }];
        let receipt = w.run(&dangling);
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Refused {
                code: "BEADS_GRAPH_RELATION_INVALID".into()
            },
            "{receipt:#?}"
        );
        assert_eq!(w.snapshot(), before);
        assert!(w.leftovers().is_empty());

        // Malformed plan: not a formula bd can cook.
        w.set_plan("formula = \"release\"\n[[steps]\n");
        let receipt = w.run(&w.repour_request(&root));
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Failed {
                code: "BEADS_COOK_FAILED".into()
            },
            "{receipt:#?}"
        );
        assert_eq!(w.snapshot(), before);
        assert!(w.leftovers().is_empty());

        // Conflicting edge type: the stored edge is `related`, the plan says `blocks`.
        w.set_plan(&plan("Build changed", ""));
        w.command(&["dep", "remove", &verify, &build]);
        w.command(&["dep", "add", &verify, &build, "--type", "related"]);
        let conflicted = w.snapshot();
        let receipt = w.run(&w.repour_request(&root));
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Refused {
                code: "BEADS_GRAPH_EDGE_CONFLICT".into()
            },
            "{receipt:#?}"
        );
        assert_eq!(w.snapshot(), conflicted, "refusal writes nothing");
        assert!(w.leftovers().is_empty());
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn repour_preserves_closed_state_and_reports_removed_nodes_and_edges() {
    with_workspace(|w| {
        let graph = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .expect("graph");
        let root = graph.parent.expect("root");
        let build = graph.ids[&step_id("build")].to_string();
        let verify = graph.ids[&step_id("verify")].to_string();
        let publish = graph.ids[&step_id("publish")].to_string();
        w.command(&[
            "update",
            &build,
            "--notes",
            "keep note",
            "--assignee",
            "reviewer",
            "--set-metadata",
            "user_key=keep",
        ]);
        w.command(&["close", &build, "--force"]);
        let before = w.bead_count();
        assert!(w.dependency_exists(&bead_id(&verify), &bead_id(&build)));
        w.set_plan("formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"Build revised\"\n[[steps]]\nid = \"verify\"\ntitle = \"Verify\"\n");
        let receipt = w.success(&w.repour_request(&root));
        let result = receipt.graph.expect("graph");
        assert_eq!(w.bead_count(), before);
        let shown = w.json(&["show", &build, "--json"]);
        assert_eq!(shown[0]["id"], build);
        assert_eq!(shown[0]["title"], "Build revised");
        assert_eq!(shown[0]["status"], "closed");
        assert_eq!(shown[0]["notes"], "keep note");
        assert_eq!(shown[0]["assignee"], "reviewer");
        assert_eq!(shown[0]["metadata"]["user_key"], "keep");
        assert!(!w.dependency_exists(&bead_id(&verify), &bead_id(&build)));
        assert!(result.nodes.iter().any(|node| {
            node.id.as_ref().is_some_and(|id| id.as_str() == publish)
                && node.action == BeadNodeAction::Retained
        }));
        assert!(result.changes.iter().any(|change| matches!(change,
            sc_composer_beads::GraphChange::RemoveInternalEdge { from, to, .. }
            if from.as_str() == verify && to.as_str() == build)));
        assert!(w.leftovers().is_empty());
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn repour_unknown_database_refuses_until_explicit_override() {
    // bd 1.3.1 embedded mode has no `bd sql`. After that probe fails, the
    // database state is unknown: repour reports DIRTY_UNKNOWN until overridden.
    with_workspace(|w| {
        let graph = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .unwrap();
        let root = graph.parent.unwrap();
        w.set_plan(&plan("Build revised", ""));
        let request = w.repour_request(&root);
        let target = sc_composer_beads::ExistingGraphTarget::PourRoot(root);
        let refuse = sc_composer_beads::GraphRevisionOptions::new(
            target.clone(),
            sc_composer_beads::UncommittedPolicy::Refuse,
        );
        let before = w.snapshot();
        let error = sc_composer_beads::execute_graph_revision(&request, &refuse).unwrap_err();
        assert_eq!(error.code(), "BEADS_GRAPH_DIRTY_UNKNOWN", "{error}");
        assert_eq!(w.snapshot(), before);
        let allow = sc_composer_beads::GraphRevisionOptions::new(
            target,
            sc_composer_beads::UncommittedPolicy::Allow,
        );
        let result = sc_composer_beads::execute_graph_revision(&request, &allow).unwrap();
        assert_eq!(result.outcome, BeadOutcome::Succeeded, "{result:#?}");
    });
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn repour_flattens_and_reverses_edges_with_external_edges_intact() {
    use sc_composer_beads::{
        ExistingGraphTarget, GraphChange, GraphRevisionOptions, UncommittedPolicy,
        execute_graph_revision,
    };
    with_workspace(|w| {
        let graph = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .unwrap();
        let root = graph.parent.unwrap();
        let build = graph.ids[&step_id("build")].to_string();
        let verify = graph.ids[&step_id("verify")].to_string();
        let publish = graph.ids[&step_id("publish")].to_string();
        let outside_gate = w.parent().to_string();
        let outside_task = w.parent().to_string();
        w.command(&["dep", "add", &outside_gate, &build]);
        w.command(&["dep", "add", &publish, &outside_task]);
        w.set_plan("formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"Build\"\nneeds = [\"verify\"]\n[[steps]]\nid = \"verify\"\ntitle = \"Verify\"\n[[steps]]\nid = \"publish\"\ntitle = \"Publish\"\nneeds = [\"verify\"]\n");
        let options = GraphRevisionOptions::new(
            ExistingGraphTarget::PourRoot(root.clone()),
            UncommittedPolicy::Allow,
        );
        let mut request = w.repour_request(&root);
        request.operation = BeadOperation::PreviewRepour;
        let before = w.snapshot();
        let preview = execute_graph_revision(&request, &options)
            .unwrap()
            .graph
            .unwrap();
        assert_eq!(w.snapshot(), before);
        assert_eq!(
            preview
                .changes
                .iter()
                .filter(|c| matches!(c, GraphChange::RemoveInternalEdge { .. }))
                .count(),
            1
        );
        assert_eq!(
            preview
                .changes
                .iter()
                .filter(|c| matches!(c, GraphChange::RetainExternalEdge { .. }))
                .count(),
            2
        );
        request.operation = BeadOperation::Repour;
        let applied = execute_graph_revision(&request, &options)
            .unwrap()
            .graph
            .unwrap();
        assert_eq!(preview.changes, applied.changes);
        assert!(!w.dependency_exists(&bead_id(&verify), &bead_id(&build)));
        assert!(w.dependency_exists(&bead_id(&publish), &bead_id(&verify)));
        assert!(w.dependency_exists(&bead_id(&build), &bead_id(&verify)));
        assert!(w.dependency_exists(&bead_id(&outside_gate), &bead_id(&build)));
        assert!(w.dependency_exists(&bead_id(&publish), &bead_id(&outside_task)));
        w.set_plan("formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"Build\"\n[[steps]]\nid = \"verify\"\ntitle = \"Verify\"\n[[steps]]\nid = \"publish\"\ntitle = \"Publish\"\n");
        request.operation = BeadOperation::PreviewRepour;
        let before = w.snapshot();
        let preview = execute_graph_revision(&request, &options)
            .unwrap()
            .graph
            .unwrap();
        assert_eq!(before, w.snapshot());
        request.operation = BeadOperation::Repour;
        let applied = execute_graph_revision(&request, &options)
            .unwrap()
            .graph
            .unwrap();
        assert_eq!(preview.changes, applied.changes);
        assert!(!w.dependency_exists(&bead_id(&build), &bead_id(&verify)));
        assert!(!w.dependency_exists(&bead_id(&publish), &bead_id(&verify)));
        assert!(!w.dependency_exists(&bead_id(&verify), &bead_id(&build)));
        assert!(w.dependency_exists(&bead_id(&outside_gate), &bead_id(&build)));
        assert!(w.dependency_exists(&bead_id(&publish), &bead_id(&outside_task)));
    });
}

#[test]
#[ignore = "separate isolated database-mode qualification only"]
/// A typed unknown is reported as unsupported, not as a qualified mode.
fn repour_clean_dirty_database_mode() {
    use sc_composer_beads::{
        ExistingGraphTarget, GraphRevisionOptions, UncommittedPolicy, execute_graph_revision,
    };
    with_workspace(|w| {
        let graph = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .unwrap();
        let root = graph.parent.unwrap();
        if std::env::var_os("SC_COMPOSE_TEST_DOLT_PORT").is_some() {
            w.command(&["dolt", "commit", "-m", "integration baseline"]);
        }
        w.set_plan(&plan("Build changed", ""));
        let request = w.repour_request(&root);
        let target = ExistingGraphTarget::PourRoot(root);
        let refuse = GraphRevisionOptions::new(target.clone(), UncommittedPolicy::Refuse);
        let mut preview = request.clone();
        preview.operation = BeadOperation::PreviewRepour;
        let clean = execute_graph_revision(&preview, &refuse);
        if let Err(error) = &clean
            && error.code() == "BEADS_GRAPH_DIRTY_UNKNOWN"
        {
            println!("\nSC_COMPOSE_MODE_UNSUPPORTED: {} {error}", error.code());
            return;
        }
        assert_eq!(
            clean.expect("server clean-state probe").outcome,
            BeadOutcome::Succeeded
        );
        let status_query = "SELECT table_name, staged, status FROM dolt_status";
        assert!(
            w.json(&["sql", "--readonly", status_query, "--json"])
                .as_array()
                .unwrap()
                .is_empty()
        );
        let build = graph.ids[&step_id("build")].to_string();
        assert_eq!(
            execute_graph_revision(&request, &refuse).unwrap().outcome,
            BeadOutcome::Succeeded
        );
        assert_eq!(
            w.json(&["show", &build, "--json"])[0]["title"],
            "Build changed"
        );
        w.set_plan(&plan("Build changed again", ""));
        w.command(&[
            "--dolt-auto-commit=off",
            "update",
            &build,
            "--notes",
            "dirty fixture",
        ]);
        assert!(
            !w.json(&["sql", "--readonly", status_query, "--json"])
                .as_array()
                .unwrap()
                .is_empty(),
            "fixture must really be dirty"
        );
        let before = w.snapshot();
        assert!(
            !w.json(&["sql", "--readonly", status_query, "--json"])
                .as_array()
                .unwrap()
                .is_empty(),
            "inspection must preserve dirty state"
        );
        let error = execute_graph_revision(&request, &refuse).unwrap_err();
        assert_eq!(error.code(), "BEADS_GRAPH_DIRTY", "{error}");
        assert_eq!(w.snapshot(), before);
        let allow = GraphRevisionOptions::new(target, UncommittedPolicy::Allow);
        assert_eq!(
            execute_graph_revision(&request, &allow).unwrap().outcome,
            BeadOutcome::Succeeded
        );
        assert_eq!(
            w.json(&["show", &build, "--json"])[0]["title"],
            "Build changed again"
        );
    });
}
