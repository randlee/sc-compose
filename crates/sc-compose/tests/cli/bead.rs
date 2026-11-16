use crate::support::bounded_command;

trait BoundedOutput {
    fn bounded_output(&mut self) -> std::io::Result<std::process::Output>;
}
impl BoundedOutput for std::process::Command {
    fn bounded_output(&mut self) -> std::io::Result<std::process::Output> {
        Ok(bounded_command::run_command(self))
    }
}
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sc_composer_beads::{BeadOperation, parse_request};
use serde_json::json;

use crate::support::{TempFixture, sc_compose, write_file};

#[cfg(unix)]
use crate::support::fake_exec;

const BEADS_SCHEMA: &str = "sc-compose/beads/v1";

#[test]
fn blocked_graph_plan_reports_its_path_before_bd() {
    let fixture = TempFixture::new("bead-graph-plan-diagnostic");
    let root = &fixture.path;
    let template = root.join("sample.formula.toml.j2");
    let rendered = root.join("sample.formula.toml");
    let plan = root.join("sample.formula.toml.graph.json");
    write_file(&template, "formula = \"sample\"\n");
    fs::create_dir(&plan).expect("block graph plan");
    let (bd, trace) = write_fake_bd(root, 0, 0);
    let request = root.join("request.json");
    write_file(
        &request,
        &json!({
            "schema": BEADS_SCHEMA, "operation": "preview_attach",
            "working_directory": root, "template": template,
            "rendered_formula": rendered, "compose_variables": {},
            "formula_name": "sample", "bead_variables": {}, "bd_executable": bd,
            "pour_authorization": "CreatePersistentBeads", "parent": "proj-1",
            "ref": "chain", "relations": []
        })
        .to_string(),
    );
    let output = sc_compose()
        .args(["bead", "preview-attach", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run CLI");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    let error = &value["payload"]["error"];
    assert_eq!(error["code"], "BEADS_OUTPUT_PATH_INVALID");
    assert_eq!(error["details"]["field"], "graph_plan");
    assert_eq!(
        fs::canonicalize(error["details"]["value"].as_str().unwrap()).unwrap(),
        fs::canonicalize(&plan).unwrap()
    );
    assert!(plan.is_dir());
    assert!(!trace.exists(), "bd must not run");
}

fn canonical_template(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sc-composer-beads/tests/fixtures/beads")
        .join(name)
}

fn copy_canonical_template(root: &Path, name: &str) -> PathBuf {
    let destination = root.join("templates").join(name);
    fs::create_dir_all(destination.parent().expect("template parent")).expect("template dir");
    fs::copy(canonical_template(name), &destination).expect("copy canonical R.1 template");
    destination
}

fn write_request(
    root: &Path,
    template: &Path,
    rendered_formula: &Path,
    bd_executable: &Path,
    authorization: Option<&str>,
) -> PathBuf {
    let request = root.join("request.json");
    write_file(
        &request,
        &serde_json::to_string_pretty(&json!({
            "schema": BEADS_SCHEMA,
            "operation": "render",
            "working_directory": root,
            "template": template,
            "rendered_formula": rendered_formula,
            "compose_variables": {
                "project": {
                    "name": "example-project",
                    "notes": "CLI canonical fixture"
                },
                "reviewers": [{ "id": "ada", "name": "Ada" }]
            },
            "formula_name": "workflow",
            "bead_variables": { "release_name": "1.5.0" },
            "bd_executable": bd_executable,
            "pour_authorization": authorization,
        }))
        .expect("serialize request"),
    );
    request
}

fn initialize_beads_workspace(root: &Path, bd: &Path) {
    fs::create_dir_all(root.join(".home")).expect("isolated home");
    let output = Command::new(bd)
        .args([
            "init",
            "--non-interactive",
            "--quiet",
            "--skip-agents",
            "--skip-hooks",
        ])
        .env("BEADS_NO_DAEMON", "1")
        .env("HOME", root.join(".home"))
        .env("XDG_CONFIG_HOME", root.join(".home/.config"))
        .env("USERPROFILE", root.join(".home"))
        .env("BEADS_DIR", root.join(".beads"))
        .current_dir(root)
        .bounded_output()
        .expect("start pinned bd init");
    assert!(
        output.status.success(),
        "bd init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
fn write_fake_bd(root: &Path, cook_exit: i32, pour_exit: i32) -> (PathBuf, PathBuf) {
    let trace = root.join("bd.trace");
    let executable = root.join("fake-bd");
    fake_exec::install(
        &executable,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$1\" >> '{}'\nif [ \"$1\" = cook ]; then exit {cook_exit}; fi\nif [ \"$1\" = where ]; then printf '%s' '{{\"path\":\"{}\"}}'; fi\nif [ \"$1\" = mol ]; then exit {pour_exit}; fi\nexit 0\n",
            trace.display(),
            root.join(".beads").display()
        ),
    );
    (executable, trace)
}

#[cfg(windows)]
fn write_fake_bd(root: &Path, cook_exit: i32, pour_exit: i32) -> (PathBuf, PathBuf) {
    let trace = root.join("bd.trace");
    let executable = root.join("fake-bd.cmd");
    let active_registry = json_safe_path(&root.join(".beads"));
    fs::write(
        &executable,
        format!(
            "@echo off\r\nset \"stage=%~1\"\r\necho %stage%>>\"{}\"\r\nif /I \"%stage%\"==\"cook\" exit /b {cook_exit}\r\nif /I \"%stage%\"==\"where\" (\r\n  echo {{\"path\":\"{}\"}}\r\n  exit /b 0\r\n)\r\nif /I \"%stage%\"==\"mol\" exit /b {pour_exit}\r\nexit /b 0\r\n",
            trace.display(),
            active_registry,
        ),
    )
    .expect("write fake bd");
    (executable, trace)
}

fn json_safe_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn trace_stages(trace: &Path) -> Vec<String> {
    fs::read_to_string(trace)
        .expect("read bd trace")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn fake_bd_registry_path_uses_json_safe_separators() {
    assert_eq!(
        json_safe_path(Path::new(r"C:\workspace\.beads")),
        "C:/workspace/.beads"
    );
}

#[test]
fn canonical_cli_request_fixture_is_a_complete_v1_request() {
    let request = fs::read_to_string(canonical_template("request.json"))
        .expect("read canonical R.2 request fixture");
    let request = parse_request(&request).expect("parse canonical R.2 request fixture");

    assert_eq!(request.schema, BEADS_SCHEMA);
    assert_eq!(request.operation, BeadOperation::Validate);
    assert_eq!(
        request
            .formula_name
            .as_ref()
            .map(sc_composer_beads::FormulaName::as_str),
        Some("toml-workflow")
    );
    assert_eq!(
        request
            .bead_variables
            .get("release_name")
            .map(String::as_str),
        Some("1.5.0")
    );
}

#[test]
fn validate_loads_the_complete_request_and_emits_the_receipt_envelope() {
    let fixture = TempFixture::new("bead-validate");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let (fake_bd, trace) = write_fake_bd(&fixture.path, 0, 0);
    let output = fixture.path.join("out").join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(&fixture.path, &template, &output, &fake_bd, None);

    let command = sc_compose()
        .args(["bead", "validate", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run bead validate");

    assert!(command.status.success(), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(envelope["payload"]["schema"], BEADS_SCHEMA);
    assert_eq!(envelope["payload"]["operation"], "validate");
    assert_eq!(envelope["payload"]["outcome"], "succeeded");
    assert_eq!(
        envelope["payload"]["stages"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(trace_stages(&trace), ["cook"]);
}

#[test]
fn failed_validation_stops_preview_before_registry_or_pour() {
    let fixture = TempFixture::new("bead-preview-failed-cook");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let (fake_bd, trace) = write_fake_bd(&fixture.path, 7, 0);
    let output = fixture
        .path
        .join(".beads")
        .join("formulas")
        .join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(&fixture.path, &template, &output, &fake_bd, None);

    let command = sc_compose()
        .args(["bead", "preview-pour", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run bead preview");

    assert_eq!(command.status.code(), Some(2), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["outcome"]["failed"]["code"],
        "BEADS_COOK_FAILED"
    );
    assert_eq!(
        envelope["payload"]["stages"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(trace_stages(&trace), ["cook"]);
}

#[test]
fn repour_without_a_target_is_a_usage_error_that_starts_no_bd() {
    let fixture = TempFixture::new("bead-repour-usage");
    let output = sc_compose()
        .args(["bead", "repour", "plan.formula.toml.j2"])
        .current_dir(&fixture.path)
        .bounded_output()
        .expect("run bead repour");
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--root"),
        "{output:?}"
    );
}

#[test]
fn repour_with_an_invalid_root_id_returns_the_typed_id_error() {
    let fixture = TempFixture::new("bead-repour-bad-root");
    fs::write(
        fixture.path.join("plan.formula.toml.j2"),
        "formula = \"plan\"\n",
    )
    .unwrap();
    let output = sc_compose()
        .args([
            "bead",
            "repour",
            "plan.formula.toml.j2",
            "--root",
            "bad id!",
        ])
        .arg("--json")
        .current_dir(&fixture.path)
        .bounded_output()
        .expect("run bead repour");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["payload"]["error"]["code"], "BEADS_GRAPH_ID_INVALID");
}

#[test]
fn persistent_pour_refuses_before_starting_bd_without_authorization() {
    let fixture = TempFixture::new("bead-pour-refused");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let (fake_bd, trace) = write_fake_bd(&fixture.path, 0, 0);
    let output = fixture.path.join("out").join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(&fixture.path, &template, &output, &fake_bd, None);

    let command = sc_compose()
        .args(["bead", "pour", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run bead pour");

    assert_eq!(command.status.code(), Some(3), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["error"]["code"],
        "BEADS_POUR_AUTH_REQUIRED"
    );
    assert!(
        !trace.exists(),
        "bd started despite refused persistent pour"
    );
}

#[test]
fn malformed_request_preserves_the_r1_deserialization_code() {
    let fixture = TempFixture::new("bead-invalid-request");
    let request = fixture.path.join("request.json");
    write_file(&request, "{ not valid JSON");

    let command = sc_compose()
        .args(["bead", "validate", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run malformed request");

    assert_eq!(command.status.code(), Some(3), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["error"]["code"],
        "BEADS_REQUEST_DESERIALIZATION_FAILED"
    );
}

#[test]
fn unreadable_request_preserves_path_and_io_kind() {
    let fixture = TempFixture::new("bead-unreadable-request");
    let request = fixture.path.join("missing-request.json");

    let command = sc_compose()
        .args(["bead", "validate", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run unreadable request");

    assert_eq!(command.status.code(), Some(3), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["error"]["code"],
        "BEADS_REQUEST_READ_FAILED"
    );
    assert_eq!(
        envelope["payload"]["error"]["details"]["path"],
        request.to_str().unwrap()
    );
    assert_eq!(envelope["payload"]["error"]["details"]["kind"], "NotFound");
    assert!(
        envelope["payload"]["error"]["recovery"]
            .as_str()
            .unwrap()
            .contains("permission")
    );
}

#[test]
fn invalid_utf8_request_is_a_read_failure() {
    let fixture = TempFixture::new("bead-invalid-utf8-request");
    let request = fixture.path.join("request.json");
    fs::write(&request, [0xff]).unwrap();
    let command = sc_compose()
        .args(["bead", "validate", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .unwrap();
    assert_eq!(command.status.code(), Some(3));
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).unwrap();
    assert_eq!(
        envelope["payload"]["error"]["code"],
        "BEADS_REQUEST_READ_FAILED"
    );
    assert_eq!(
        envelope["payload"]["error"]["details"]["kind"],
        "InvalidData"
    );
}

#[test]
fn render_failure_uses_the_r1_receipt_code_and_nonzero_exit() {
    let fixture = TempFixture::new("bead-render-failed");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let (fake_bd, _) = write_fake_bd(&fixture.path, 0, 0);
    let output = fixture.path.join("out").join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(&fixture.path, &template, &output, &fake_bd, None);
    let mut document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&request).expect("request")).expect("JSON");
    document["compose_variables"] = json!({});
    write_file(
        &request,
        &serde_json::to_string(&document).expect("serialize malformed composition request"),
    );

    let command = sc_compose()
        .args(["bead", "render", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run bead render");

    assert_eq!(command.status.code(), Some(2), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["outcome"]["failed"]["code"],
        "BEADS_RENDER_FAILED"
    );
}

#[test]
fn unavailable_bd_preserves_the_r1_error_code_and_nonzero_exit() {
    let fixture = TempFixture::new("bead-bd-unavailable");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let output = fixture.path.join("out").join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(
        &fixture.path,
        &template,
        &output,
        &fixture.path.join("missing-bd"),
        None,
    );

    let command = sc_compose()
        .args(["bead", "validate", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run unavailable bd request");

    assert_eq!(command.status.code(), Some(2), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(envelope["payload"]["error"]["code"], "BEADS_BD_UNAVAILABLE");
}

#[test]
fn failed_preview_reports_its_exact_r1_stage_code() {
    let fixture = TempFixture::new("bead-preview-failed");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let (fake_bd, trace) = write_fake_bd(&fixture.path, 0, 9);
    let output = fixture
        .path
        .join(".beads")
        .join("formulas")
        .join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(&fixture.path, &template, &output, &fake_bd, None);

    let command = sc_compose()
        .args(["bead", "preview-pour", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run failed bead preview");

    assert_eq!(command.status.code(), Some(2), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["outcome"]["failed"]["code"],
        "BEADS_PREVIEW_POUR_FAILED"
    );
    assert_eq!(trace_stages(&trace), ["cook", "where", "mol"]);
}

#[test]
fn authorized_failed_pour_reports_its_exact_r1_stage_code() {
    let fixture = TempFixture::new("bead-pour-failed");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let (fake_bd, trace) = write_fake_bd(&fixture.path, 0, 9);
    let output = fixture
        .path
        .join(".beads")
        .join("formulas")
        .join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(
        &fixture.path,
        &template,
        &output,
        &fake_bd,
        Some("CreatePersistentBeads"),
    );

    let command = sc_compose()
        .args(["bead", "pour", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run failed authorized pour");

    assert_eq!(command.status.code(), Some(2), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["outcome"]["failed"]["code"],
        "BEADS_POUR_FAILED"
    );
    assert_eq!(trace_stages(&trace), ["cook", "where", "mol"]);
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn pinned_bd_validates_the_canonical_cli_fixture_when_configured() {
    let Some(pinned_bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let fixture = TempFixture::new("bead-pinned-bd");
    initialize_beads_workspace(&fixture.path, &pinned_bd);
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let output = fixture.path.join("out").join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("output parent")).expect("output directory");
    let request = write_request(&fixture.path, &template, &output, &pinned_bd, None);

    let command = sc_compose()
        .args(["bead", "validate", "--request"])
        .arg(&request)
        .arg("--json")
        .env("BEADS_NO_DAEMON", "1")
        .env("HOME", fixture.path.join(".home"))
        .env("XDG_CONFIG_HOME", fixture.path.join(".home/.config"))
        .env("USERPROFILE", fixture.path.join(".home"))
        .env("BEADS_DIR", fixture.path.join(".beads"))
        .bounded_output()
        .expect("run pinned bd validation");

    assert!(command.status.success(), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(envelope["payload"]["outcome"], "succeeded");
}

#[test]
fn invalid_pour_authorization_returns_its_code_before_render_or_bd() {
    let fixture = TempFixture::new("bead-pour-invalid-auth");
    let template = copy_canonical_template(&fixture.path, "toml-workflow.formula.toml.j2");
    let (fake_bd, trace) = write_fake_bd(&fixture.path, 0, 0);
    let output = fixture.path.join("out").join("workflow.formula.toml");
    fs::create_dir_all(output.parent().expect("parent")).expect("output directory");
    let request = write_request(&fixture.path, &template, &output, &fake_bd, Some("invalid"));
    let command = sc_compose()
        .args(["bead", "pour", "--request"])
        .arg(request)
        .arg("--json")
        .bounded_output()
        .expect("bead pour");
    assert_eq!(command.status.code(), Some(3), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(
        envelope["payload"]["error"]["code"],
        "BEADS_POUR_AUTH_INVALID"
    );
    assert!(!trace.exists(), "authorization error starts no bd process");
    assert!(!output.exists(), "authorization error writes no formula");
}

#[test]
#[cfg(unix)]
fn malformed_relation_endpoint_has_stable_request_code_before_side_effects() {
    let fixture = TempFixture::new("bead-invalid-endpoint");
    let root = &fixture.path;
    let template = copy_canonical_template(root, "toml-workflow.formula.toml.j2");
    let output = root.join("output.formula.toml");
    let (bd, trace) = write_fake_bd(root, 0, 0);
    let request = write_request(root, &template, &output, &bd, None);
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&request).unwrap()).unwrap();
    value["relations"] = json!([{"from":"build","to":"bead:parent","type":"blocks"}]);
    write_file(&request, &value.to_string());
    let result = sc_compose()
        .args(["bead", "render", "--request"])
        .arg(request)
        .arg("--json")
        .bounded_output()
        .unwrap();
    assert_eq!(result.status.code(), Some(3));
    let envelope: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        envelope["payload"]["error"]["code"],
        "BEADS_RELATION_ENDPOINT_INVALID"
    );
    assert!(!output.exists());
    assert!(!trace.exists());
}

struct GraphCliWorkspace {
    fixture: TempFixture,
    bd: PathBuf,
    beads_dir: PathBuf,
}

impl GraphCliWorkspace {
    fn new(bd: PathBuf) -> Self {
        let fixture = TempFixture::new("bead-graph-e2e");
        let beads_dir = fixture.path.join(".beads");
        fs::create_dir_all(fixture.path.join(".home")).expect("isolated home");
        let workspace = Self {
            fixture,
            bd,
            beads_dir,
        };
        let mut init = vec![
            "init".to_owned(),
            "--non-interactive".into(),
            "--quiet".into(),
            "--skip-agents".into(),
            "--skip-hooks".into(),
            "--prefix".into(),
            "cli".into(),
        ];
        // A bd built without embedded Dolt needs a dedicated throwaway server.
        if let Some(port) = std::env::var_os("SC_COMPOSE_TEST_DOLT_PORT") {
            fs::create_dir_all(&workspace.beads_dir).expect("beads directory");
            let sequence = workspace
                .fixture
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.rsplit('-').next())
                .unwrap_or("0")
                .to_owned();
            init.extend([
                "--server".into(),
                "--server-port".into(),
                port.to_string_lossy().into_owned(),
                "--database".into(),
                format!("scc_{}_{sequence}", std::process::id()),
            ]);
        }
        let init: Vec<&str> = init.iter().map(String::as_str).collect();
        workspace.bd_output(&init);
        fs::create_dir_all(workspace.fixture.path.join("build")).expect("build directory");
        workspace
    }

    fn bd_output(&self, args: &[&str]) -> String {
        let mut command = Command::new(&self.bd);
        command
            .args(args)
            .current_dir(&self.fixture.path)
            .env("HOME", self.fixture.path.join(".home"))
            .env("XDG_CONFIG_HOME", self.fixture.path.join(".home/.config"))
            .env("USERPROFILE", self.fixture.path.join(".home"))
            .env("BEADS_DIR", &self.beads_dir)
            .env("BEADS_NO_DAEMON", "1");
        let output = bounded_command::run_command(&mut command);
        assert!(output.status.success(), "{args:?}: {output:?}");
        String::from_utf8(output.stdout).expect("bd UTF-8")
    }

    fn bd_json(&self, args: &[&str]) -> serde_json::Value {
        serde_json::from_str(&self.bd_output(args)).expect("fixture bd JSON")
    }

    fn snapshot(&self) -> serde_json::Value {
        let beads = self.bd_json(&["list", "--all", "-n", "0", "--json"]);
        let mut edges = std::collections::BTreeMap::new();
        for bead in beads.as_array().expect("beads") {
            let id = bead["id"].as_str().expect("id");
            edges.insert(id, self.bd_json(&["dep", "list", id, "--json"]));
        }
        json!({"beads": beads, "edges": edges})
    }

    fn cli(
        &self,
        operation: &str,
        request: &Path,
        json_output: bool,
        code: i32,
    ) -> std::process::Output {
        let mut command = sc_compose();
        command
            .args(["bead", operation, "--request"])
            .arg(request)
            .args(["--render-by", "bd"])
            .current_dir(&self.fixture.path)
            .env("HOME", self.fixture.path.join(".home"))
            .env("XDG_CONFIG_HOME", self.fixture.path.join(".home/.config"))
            .env("USERPROFILE", self.fixture.path.join(".home"))
            .env("BEADS_DIR", &self.beads_dir)
            .env("BEADS_NO_DAEMON", "1");
        if json_output {
            command.arg("--json");
        }
        let output = command.bounded_output().expect("bead CLI");
        assert_eq!(output.status.code(), Some(code), "{operation}: {output:?}");
        output
    }

    fn write_request(&self, value: &serde_json::Value) -> PathBuf {
        let path = self.fixture.path.join("request.json");
        write_file(
            &path,
            &serde_json::to_string_pretty(value).expect("request JSON"),
        );
        path
    }
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn direct_plan_fix_pour_persists_values_and_declared_dependency() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let workspace = GraphCliWorkspace::new(bd);
    let root = &workspace.fixture.path;
    let registry = root.join(".beads/formulas");
    fs::create_dir_all(&registry).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/beads/plan-fix.formula.toml.j2"),
        registry.join("plan-fix.formula.toml.j2"),
    )
    .unwrap();
    let bin = root.join("fake-bin");
    fs::create_dir(&bin).unwrap();
    fs::copy(
        &workspace.bd,
        bin.join(if cfg!(windows) { "bd.exe" } else { "bd" }),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "bead",
            "pour",
            "plan-fix",
            "--var",
            "plan=phase-v",
            "--var",
            "finding=EDGE-1",
            "--var",
            "literal={{ evidence }}",
            "--json",
        ])
        .current_dir(root)
        .env("PATH", workspace.fixture.path_with_fake_tools())
        .env("HOME", workspace.fixture.path.join(".home"))
        .env(
            "XDG_CONFIG_HOME",
            workspace.fixture.path.join(".home/.config"),
        )
        .env("USERPROFILE", workspace.fixture.path.join(".home"))
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // This first pour creates beads, so its staged graph plan is reported.
    assert_eq!(
        assert_receipt_paths_exist_or_are_ephemeral(&envelope["payload"]),
        ["rendered_formula", "graph.plan_path"]
    );
    let graph = &envelope["payload"]["graph"];
    let review = graph["ids"]["review"].as_str().expect("review id");
    let fix = graph["ids"]["fix"].as_str().expect("fix id");
    let review_row = workspace.bd_json(&["show", review, "--json"]);
    let fix_row = workspace.bd_json(&["show", fix, "--json"]);
    assert_eq!(review_row[0]["title"], "Review EDGE-1");
    assert!(
        review_row[0]["description"]
            .as_str()
            .unwrap()
            .contains("{{ evidence }}")
    );
    assert_eq!(fix_row[0]["title"], "Fix EDGE-1");
    assert!(
        review_row[0]["labels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|label| label == "plan-phase-v")
    );
    let edges = workspace.bd_json(&["dep", "list", fix, "--json"]);
    assert!(
        edges
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["id"] == review && edge["dependency_type"] == "blocks")
    );

    assert_plan_fix_json_matches(&workspace, &registry, &review_row[0], &fix_row[0]);
    assert_plan_fix_path_matches(&workspace, &review_row[0], &fix_row[0]);
    assert_direct_pour_refusals_preserve_state(&workspace);
}

fn assert_plan_fix_path_matches(
    workspace: &GraphCliWorkspace,
    review: &serde_json::Value,
    fix: &serde_json::Value,
) {
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "bead",
            "pour",
            ".beads/formulas/plan-fix.formula.toml.j2",
            "--var",
            "plan=phase-v",
            "--var",
            "finding=EDGE-1",
            "--var",
            "literal={{ evidence }}",
            "--json",
        ])
        .current_dir(&workspace.fixture.path)
        .env("PATH", workspace.fixture.path_with_fake_tools())
        .env("HOME", workspace.fixture.path.join(".home"))
        .env(
            "XDG_CONFIG_HOME",
            workspace.fixture.path.join(".home/.config"),
        )
        .env("USERPROFILE", workspace.fixture.path.join(".home"))
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_receipt_paths_exist_or_are_ephemeral(&receipt["payload"]);
    for (step, expected) in [("review", review), ("fix", fix)] {
        let id = receipt["payload"]["graph"]["ids"][step].as_str().unwrap();
        let row = workspace.bd_json(&["show", id, "--json"]);
        for field in ["title", "description", "labels"] {
            assert_eq!(row[0][field], expected[field]);
        }
    }
}

fn assert_plan_fix_json_matches(
    workspace: &GraphCliWorkspace,
    registry: &Path,
    review_row: &serde_json::Value,
    fix_row: &serde_json::Value,
) {
    let root = &workspace.fixture.path;
    let request = workspace.write_request(&json!({
        "schema": BEADS_SCHEMA,
        "operation": "pour",
        "working_directory": fs::canonicalize(root).unwrap(),
        "template": registry.join("plan-fix.formula.toml.j2"),
        "rendered_formula": root.join("build/plan-fix.formula.toml"),
        "compose_variables": {},
        "formula_name": "plan-fix",
        "bead_variables": {"plan":"phase-v", "finding":"EDGE-1", "literal":"{{ evidence }}"},
        "bd_executable": workspace.bd,
        "pour_authorization": "CreatePersistentBeads"
    }));
    let json_output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args(["bead", "pour", "--request"])
        .arg(&request)
        .arg("--json")
        .current_dir(root)
        .env("HOME", workspace.fixture.path.join(".home"))
        .env(
            "XDG_CONFIG_HOME",
            workspace.fixture.path.join(".home/.config"),
        )
        .env("USERPROFILE", workspace.fixture.path.join(".home"))
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap();
    assert!(json_output.status.success(), "{json_output:?}");
    let json_receipt: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    for (step, direct) in [("review", review_row), ("fix", fix_row)] {
        let id = json_receipt["payload"]["graph"]["ids"][step]
            .as_str()
            .unwrap();
        let row = workspace.bd_json(&["show", id, "--json"]);
        for field in ["title", "description", "labels"] {
            assert_eq!(row[0][field], direct[field], "{step} {field}");
        }
    }
    let json_review = json_receipt["payload"]["graph"]["ids"]["review"]
        .as_str()
        .unwrap();
    let json_fix = json_receipt["payload"]["graph"]["ids"]["fix"]
        .as_str()
        .unwrap();
    let json_edges = workspace.bd_json(&["dep", "list", json_fix, "--json"]);
    assert!(
        json_edges
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["id"] == json_review && edge["dependency_type"] == "blocks")
    );
}

fn assert_direct_pour_refusals_preserve_state(workspace: &GraphCliWorkspace) {
    let root = &workspace.fixture.path;
    let before = workspace.snapshot();
    let refusal_cases: &[&[&str]] = &[
        &[
            "bead", "pour", "plan-fix", "--var", "plan=one", "--var", "plan=two",
        ],
        &["bead", "pour", "plan-fix", "--request", "request.json"],
        &["bead", "pour", "not-a-formula", "--var", "plan=phase-v"],
        &[
            "bead",
            "pour",
            "plan-fix",
            "--var",
            "plan=phase-v",
            "--var",
            "literal=value",
        ],
    ];
    for args in refusal_cases {
        let result = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
            .args(*args)
            .current_dir(root)
            .env("PATH", workspace.fixture.path_with_fake_tools())
            .env("HOME", workspace.fixture.path.join(".home"))
            .env(
                "XDG_CONFIG_HOME",
                workspace.fixture.path.join(".home/.config"),
            )
            .env("USERPROFILE", workspace.fixture.path.join(".home"))
            .env("BEADS_DIR", &workspace.beads_dir)
            .env("BEADS_NO_DAEMON", "1")
            .bounded_output()
            .unwrap();
        assert!(!result.status.success(), "{args:?}: {result:?}");
        assert_eq!(
            workspace.snapshot(),
            before,
            "{args:?} must not change beads or edges"
        );
    }
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn pinned_bd_graph_pour_and_attach_run_the_release_example_with_text_and_exit_contracts() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let workspace = GraphCliWorkspace::new(bd);
    let root = &workspace.fixture.path;
    let example =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/beads/release-under-epic");
    fs::copy(
        example.join("release.formula.toml.j2"),
        root.join("release.formula.toml.j2"),
    )
    .expect("example template");
    let mut request: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(example.join("request.json")).expect("example request"),
    )
    .expect("example JSON");
    request["working_directory"] = json!(fs::canonicalize(root).expect("canonical workspace"));
    request["rendered_formula"] = json!(root.join("build/release.formula.toml"));
    request["bd_executable"] = json!(workspace.bd);
    let parent = workspace.bd_json(&["create", "Release 1.6.1", "--type", "epic", "--json"])["id"]
        .as_str()
        .expect("epic id")
        .to_owned();
    request["parent"] = json!(parent);

    assert_by_path_pour(&workspace, &request);
    assert_attach_exit_codes(&workspace, &request);
    assert_attach_and_repeat(&workspace, &request, &parent);
    assert_missing_edge_recovery(&workspace, &request, &parent);
    let help = sc_compose()
        .args(["help", "bead"])
        .bounded_output()
        .expect("help bead");
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Release under an epic"));
}

fn assert_by_path_pour(workspace: &GraphCliWorkspace, request: &serde_json::Value) {
    let mut pour = request.clone();
    pour["operation"] = json!("pour");
    pour.as_object_mut().expect("request").remove("parent");
    pour.as_object_mut().expect("request").remove("ref");
    let path = workspace.write_request(&pour);
    assert!(!workspace.beads_dir.join("formulas").exists());
    let before = workspace.snapshot();
    let preview = workspace.cli("preview-pour", &path, false, 0);
    let text = String::from_utf8(preview.stdout).expect("preview text");
    assert!(text.contains("pour_mode: graph"));
    assert!(text.contains("create: _root -> pending"));
    assert!(text.contains("create: build -> pending"));
    assert!(text.contains("edges: 5"));
    assert!(text.contains("plan_path: "));
    assert_eq!(
        workspace.snapshot(),
        before,
        "pour preview writes no beads or edges"
    );
    let poured = workspace.cli("pour", &path, true, 0);
    let envelope: serde_json::Value =
        serde_json::from_slice(&poured.stdout).expect("pour envelope");
    assert_eq!(envelope["payload"]["pour_mode"], "graph");
    assert_eq!(
        envelope["payload"]["graph"]["ids"]
            .as_object()
            .expect("ids")
            .len(),
        3
    );
    assert!(
        !workspace.beads_dir.join("formulas").exists(),
        "by-path pour never creates a registry"
    );
}

fn assert_attach_exit_codes(workspace: &GraphCliWorkspace, request: &serde_json::Value) {
    for operation in ["preview-attach", "attach"] {
        let mut invalid = request.clone();
        invalid.as_object_mut().expect("request").remove("parent");
        let path = workspace.write_request(&invalid);
        let output = workspace.cli(operation, &path, true, 3);
        let envelope: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("request error envelope");
        assert_eq!(
            envelope["payload"]["error"]["code"],
            "BEADS_REQUEST_DESERIALIZATION_FAILED"
        );
        let mut missing = request.clone();
        missing["parent"] = json!("cli-missing");
        let path = workspace.write_request(&missing);
        let before = workspace.snapshot();
        let output = workspace.cli(operation, &path, true, 2);
        let envelope: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("refusal envelope");
        assert_eq!(
            envelope["payload"]["outcome"]["refused"]["code"],
            "BEADS_GRAPH_PARENT_NOT_FOUND"
        );
        assert_eq!(workspace.snapshot(), before);
    }
}

fn assert_attach_and_repeat(
    workspace: &GraphCliWorkspace,
    request: &serde_json::Value,
    parent: &str,
) {
    let path = workspace.write_request(request);
    let before = workspace.snapshot();
    let preview = workspace.cli("preview-attach", &path, false, 0);
    let text = String::from_utf8(preview.stdout).expect("attach preview text");
    for step in ["build", "verify", "publish"] {
        assert!(
            text.contains(&format!("create: {step} -> {parent}.release-{step}")),
            "{text}"
        );
    }
    assert!(text.contains("edges: 5"));
    assert!(text.contains("plan_path: "));
    assert_eq!(workspace.snapshot(), before);
    let attached = workspace.cli("attach", &path, false, 0);
    let text = String::from_utf8(attached.stdout).expect("attach text");
    for step in ["build", "verify", "publish"] {
        assert!(
            text.contains(&format!("created: {step} -> {parent}.release-{step}")),
            "{text}"
        );
    }
    let before = workspace.snapshot();
    let repeated = workspace.cli("attach", &path, true, 0);
    let envelope: serde_json::Value =
        serde_json::from_slice(&repeated.stdout).expect("repeat envelope");
    let graph = &envelope["payload"]["graph"];
    assert!(graph.get("plan_path").is_none());
    assert_eq!(graph["parent"], parent);
    for step in ["build", "verify", "publish"] {
        assert_eq!(graph["ids"][step], format!("{parent}.release-{step}"));
    }
    assert!(
        graph["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .all(|node| node["action"] == "existing")
    );
    assert!(
        graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .all(|edge| edge["action"] == "existing")
    );
    assert_eq!(
        workspace.snapshot(),
        before,
        "repeat writes no beads or edges"
    );
    let repeated = workspace.cli("attach", &path, false, 0);
    let text = String::from_utf8(repeated.stdout).expect("repeat text");
    assert!(text.contains(&format!("existing: build -> {parent}.release-build")));
    assert!(!text.contains("plan_path:"));
}

fn assert_missing_edge_recovery(
    workspace: &GraphCliWorkspace,
    request: &serde_json::Value,
    parent: &str,
) {
    let path = workspace.write_request(request);
    for (from, to) in [("verify", "build"), ("publish", "verify")] {
        workspace.bd_output(&[
            "dep",
            "remove",
            &format!("{parent}.release-{from}"),
            &format!("{parent}.release-{to}"),
        ]);
    }
    let before = workspace.snapshot();
    let refusal = workspace.cli("attach", &path, false, 2);
    let text = String::from_utf8(refusal.stdout).expect("repair text");
    assert!(text.contains("BEADS_GRAPH_EDGE_MISSING"), "{text}");
    for (from, to) in [("verify", "build"), ("publish", "verify")] {
        assert!(
            text.lines().any(|line| line
                == format!(
                    "bd dep add '{parent}.release-{from}' '{parent}.release-{to}' --type 'blocks'"
                )),
            "{text}"
        );
    }
    assert_eq!(workspace.snapshot(), before, "repair advice writes nothing");
}

#[test]
fn direct_pour_rejects_duplicate_mixed_and_missing_sources_before_staging() {
    let root = TempFixture::new("direct-pour-refusals");
    let cases: &[&[&str]] = &[
        &[
            "bead", "pour", "plan-fix", "--var", "key=one", "--var", "key=two",
        ],
        &["bead", "pour", "plan-fix", "--request", "request.json"],
        &["bead", "pour", "missing-formula", "--var", "key=value"],
    ];
    for args in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
            .args(*args)
            .current_dir(&root.path)
            .bounded_output()
            .expect("run direct pour refusal");
        assert!(!output.status.success(), "{args:?}: {output:?}");
        assert_eq!(fs::read_dir(&root.path).unwrap().count(), 0, "{args:?}");
    }
}

#[test]
fn missing_direct_formula_reports_typed_source_failure() {
    let root = TempFixture::new("direct-pour-missing-source");
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args(["bead", "pour", "missing.formula.toml", "--json"])
        .current_dir(&root.path)
        .bounded_output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("BEADS_DIRECT_SOURCE_FAILED"), "{text}");
    assert!(text.contains("missing.formula.toml"), "{text}");
    assert!(text.contains("recovery"), "{text}");
    assert_eq!(fs::read_dir(&root.path).unwrap().count(), 0);
}

/// Every path a receipt prints must exist after the command returns, or be
/// listed in `ephemeral_paths` because the command removed it.
fn assert_receipt_paths_exist_or_are_ephemeral(payload: &serde_json::Value) -> Vec<String> {
    let ephemeral: Vec<String> = payload
        .get("ephemeral_paths")
        .map(|fields| serde_json::from_value(fields.clone()).expect("ephemeral_paths"))
        .unwrap_or_default();
    for (field, path) in [
        ("rendered_formula", &payload["rendered_formula"]),
        ("graph.plan_path", &payload["graph"]["plan_path"]),
    ] {
        if let Some(path) = path.as_str() {
            assert!(
                Path::new(path).exists() || ephemeral.iter().any(|name| name == field),
                "{field} names a missing file without flagging it: {payload}"
            );
        }
    }
    ephemeral
}

// Direct pour resolves `bd` from PATH, and only Unix finds a script there.
#[cfg(unix)]
#[test]
fn direct_pour_receipt_flags_staged_paths_removed_on_exit() {
    let fixture = TempFixture::new("direct-pour-receipt-paths");
    let root = &fixture.path;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/beads/plan-fix.formula.toml.j2"),
        root.join("plan-fix.formula.toml.j2"),
    )
    .expect("copy plan-fix example");
    // A failing cook stops the pour after the formula is rendered and staged.
    let (fake_bd, trace) = write_fake_bd(root, 1, 0);
    fs::create_dir(root.join("fake-bin")).expect("fake-bin");
    let body = fs::read_to_string(fake_exec::script_path(&fake_bd)).expect("fake bd script");
    fake_exec::install(&root.join("fake-bin/bd"), &body);
    let pour = |json: bool| {
        let mut command = sc_compose();
        command
            .args(["bead", "pour", "plan-fix.formula.toml.j2"])
            .args(["--var", "plan=p", "--var", "finding=F", "--var"])
            .arg("literal={{ x }}")
            .current_dir(root)
            .env("PATH", fixture.path_with_fake_tools());
        if json {
            command.arg("--json");
        }
        let output = command.bounded_output().expect("run direct pour");
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        output
    };

    let envelope: serde_json::Value =
        serde_json::from_slice(&pour(true).stdout).expect("receipt envelope");
    let payload = &envelope["payload"];
    assert_eq!(payload["outcome"]["failed"]["code"], "BEADS_COOK_FAILED");
    let staged = payload["rendered_formula"]
        .as_str()
        .expect("rendered_formula");
    assert!(staged.contains(".sc-compose-pour-"), "{payload}");
    assert!(!Path::new(staged).exists(), "staging is removed on exit");
    assert_eq!(
        assert_receipt_paths_exist_or_are_ephemeral(payload),
        ["rendered_formula"]
    );

    let human = String::from_utf8(pour(false).stdout).expect("human receipt");
    assert_eq!(
        trace_stages(&trace),
        ["cook", "cook"],
        "configured fake handled both calls"
    );
    let line = human
        .lines()
        .find(|line| line.starts_with("rendered_formula: "))
        .expect("rendered_formula line");
    assert!(
        line.ends_with(" (ephemeral: removed when this command exited)"),
        "{line}"
    );
    assert!(
        fs::read_dir(root).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".sc-compose-pour-")),
        "no staging directory is left behind"
    );

    // A request names its own output, which is kept and never flagged.
    let template = copy_canonical_template(root, "toml-workflow.formula.toml.j2");
    let output = root.join("build/workflow.formula.toml");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let request = write_request(root, &template, &output, &fake_bd, None);
    let rendered = sc_compose()
        .args(["bead", "render", "--request"])
        .arg(&request)
        .arg("--json")
        .bounded_output()
        .expect("run request render");
    assert!(rendered.status.success(), "{rendered:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&rendered.stdout).expect("envelope");
    assert!(envelope["payload"].get("ephemeral_paths").is_none());
    assert!(assert_receipt_paths_exist_or_are_ephemeral(&envelope["payload"]).is_empty());
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn direct_bd_owned_registry_pour_passes_variables_to_bd() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let workspace = GraphCliWorkspace::new(bd);
    let root = &workspace.fixture.path;
    let registry = root.join(".beads/formulas");
    fs::create_dir_all(&registry).unwrap();
    fs::write(registry.join("legacy.formula.toml.j2"), "formula = \"legacy\"\nversion = 1\ntype = \"workflow\"\n[vars.release]\nrequired = true\n[[steps]]\nid = \"build\"\ntitle = \"Release {{ release }}\"\n").unwrap();
    fs::create_dir(root.join("fake-bin")).unwrap();
    fs::copy(
        &workspace.bd,
        root.join("fake-bin")
            .join(if cfg!(windows) { "bd.exe" } else { "bd" }),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "bead",
            "pour",
            "legacy",
            "--render-by",
            "bd",
            "--var",
            "release=2.0",
            "--json",
        ])
        .current_dir(root)
        .env("PATH", workspace.fixture.path_with_fake_tools())
        .env("HOME", workspace.fixture.path.join(".home"))
        .env(
            "XDG_CONFIG_HOME",
            workspace.fixture.path.join(".home/.config"),
        )
        .env("USERPROFILE", workspace.fixture.path.join(".home"))
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let stages = receipt["payload"]["stages"].as_array().unwrap();
    let pour = stages
        .iter()
        .find(|stage| stage["stage"] == "pour")
        .unwrap();
    let argv = pour["argv"].as_array().unwrap();
    assert!(
        argv.windows(2)
            .any(|pair| pair[0] == "--var" && pair[1] == "release=2.0")
    );
    let listed = workspace.bd_json(&["list", "--all", "-n", "0", "--json"]);
    assert!(
        listed
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["title"] == "Release {{ release }}"),
        "{listed}"
    );
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn direct_named_user_registry_snapshots_confined_includes() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let workspace = GraphCliWorkspace::new(bd);
    let home = TempFixture::new("bead-registry-home");
    let registry = home.path.join(".beads/formulas");
    fs::create_dir_all(&registry).unwrap();
    fs::write(
        registry.join("external.formula.toml.j2"),
        "formula = \"external\"\nversion = 1\ntype = \"workflow\"\n@<part.j2>\n",
    )
    .unwrap();
    fs::write(registry.join("part.j2"), "---\ndefaults:\n  fallback: '{{ evidence }}'\n---\n[[steps]]\nid = \"build\"\ntitle = \"Build {{ value }}\"\ndescription = \"{{ fallback }}\"\n").unwrap();
    let bin = workspace.fixture.path.join("fake-bin");
    fs::create_dir(&bin).unwrap();
    fs::copy(
        &workspace.bd,
        bin.join(if cfg!(windows) { "bd.exe" } else { "bd" }),
    )
    .unwrap();
    let receipt = run_external_registry_pour(&workspace, &home.path);
    let id = receipt["payload"]["graph"]["ids"]["build"]
        .as_str()
        .unwrap();
    let row = workspace.bd_json(&["show", id, "--json"]);
    assert_eq!(row[0]["title"], "Build outside");
    assert_eq!(row[0]["description"], "{{ evidence }}");
    workspace.bd_output(&["update", id, "--notes", "keep", "--assignee", "reviewer"]);
    let second = run_external_registry_pour(&workspace, &home.path);
    let second_id = second["payload"]["graph"]["ids"]["build"].as_str().unwrap();
    let second_row = workspace.bd_json(&["show", second_id, "--json"]);
    assert_eq!(row[0]["metadata"]["sc_compose_graph"]["v"], 2);
    assert!(
        row[0]["metadata"]["sc_compose_graph"]["node_definition"]
            .as_str()
            .is_some()
    );
    assert_eq!(
        row[0]["metadata"]["sc_compose_graph"],
        second_row[0]["metadata"]["sc_compose_graph"]
    );
    let preserved = workspace.bd_json(&["show", id, "--json"]);
    assert_eq!(preserved[0]["notes"], "keep");
    assert_eq!(preserved[0]["assignee"], "reviewer");
    assert!(fs::read_dir(&workspace.fixture.path).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".sc-compose-pour-")
    }));

    let active = workspace.beads_dir.join("formulas");
    fs::create_dir_all(&active).unwrap();
    fs::write(active.join("priority.formula.toml"), "formula = \"priority\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"Active source\"\n").unwrap();
    fs::write(registry.join("priority.formula.toml.j2"), "formula = \"priority\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"User source\"\n").unwrap();
    let priority = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args(["bead", "pour", "priority", "--json"])
        .current_dir(&workspace.fixture.path)
        .env("PATH", workspace.fixture.path_with_fake_tools())
        .env("HOME", &home.path)
        .env("USERPROFILE", &home.path)
        .env("GT_ROOT", &home.path)
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap();
    assert!(priority.status.success(), "{priority:?}");
    let priority_receipt: serde_json::Value = serde_json::from_slice(&priority.stdout).unwrap();
    let priority_id = priority_receipt["payload"]["graph"]["ids"]["build"]
        .as_str()
        .unwrap();
    assert_eq!(
        workspace.bd_json(&["show", priority_id, "--json"])[0]["title"],
        "Active source"
    );

    fs::write(registry.join("escape.formula.toml.j2"), "@<../secret.j2>\n").unwrap();
    fs::write(home.path.join(".beads/secret.j2"), "private").unwrap();
    let before = workspace.snapshot();
    let refused = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args(["bead", "pour", "escape", "--json"])
        .current_dir(&workspace.fixture.path)
        .env("PATH", workspace.fixture.path_with_fake_tools())
        .env("HOME", &home.path)
        .env("USERPROFILE", &home.path)
        .env("GT_ROOT", &home.path)
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(workspace.snapshot(), before);
}

fn run_external_registry_pour(workspace: &GraphCliWorkspace, home: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "bead",
            "pour",
            "external",
            "--var",
            "value=outside",
            "--json",
        ])
        .current_dir(&workspace.fixture.path)
        .env("PATH", workspace.fixture.path_with_fake_tools())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("GT_ROOT", home)
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn advanced_attach_missing_edge_previews_then_applies_the_repair() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let workspace = GraphCliWorkspace::new(bd);
    let root = &workspace.fixture.path;
    let example =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/beads/release-under-epic");
    fs::copy(
        example.join("release.formula.toml.j2"),
        root.join("release.formula.toml.j2"),
    )
    .unwrap();
    let mut request: serde_json::Value =
        serde_json::from_slice(&fs::read(example.join("request.json")).unwrap()).unwrap();
    let parent = workspace.bd_json(&["create", "Parent", "--type", "epic", "--json"])["id"]
        .as_str()
        .unwrap()
        .to_owned();
    request["working_directory"] = json!(fs::canonicalize(root).unwrap());
    request["rendered_formula"] = json!(root.join("build/release.formula.toml"));
    request["bd_executable"] = json!(workspace.bd);
    request["parent"] = json!(parent);
    let request = workspace.write_request(&request);
    let applied = advanced_graph_cli(&workspace, "attach", &request);
    assert!(applied.status.success(), "{applied:?}");
    let created: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    let graph = &created["payload"]["graph"]["ids"];
    let build = graph["build"].as_str().unwrap();
    let verify = graph["verify"].as_str().unwrap();
    workspace.bd_output(&["dep", "remove", verify, build]);
    let before = workspace.snapshot();
    let preview = advanced_graph_cli(&workspace, "preview-attach", &request);
    assert!(preview.status.success(), "{preview:?}");
    let planned: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert!(
        planned["payload"]["graph"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["from"] == verify && edge["to"] == build && edge["action"] == "add")
    );
    assert_eq!(workspace.snapshot(), before);
    let repaired = advanced_graph_cli(&workspace, "attach", &request);
    assert!(repaired.status.success(), "{repaired:?}");
    let value: serde_json::Value = serde_json::from_slice(&repaired.stdout).unwrap();
    assert!(
        value["payload"]["graph"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["from"] == verify && edge["to"] == build && edge["action"] == "added"),
        "{value:#}"
    );
    assert_ne!(workspace.snapshot(), before, "the missing edge is restored");
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn repour_by_request_updates_an_attached_plan_and_keeps_runtime_state() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let workspace = GraphCliWorkspace::new(bd);
    let root = &workspace.fixture.path;
    let example =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/beads/release-under-epic");
    fs::copy(
        example.join("release.formula.toml.j2"),
        root.join("release.formula.toml.j2"),
    )
    .unwrap();
    let mut request: serde_json::Value =
        serde_json::from_slice(&fs::read(example.join("request.json")).unwrap()).unwrap();
    let parent = workspace.bd_json(&["create", "Parent", "--type", "epic", "--json"])["id"]
        .as_str()
        .unwrap()
        .to_owned();
    request["working_directory"] = json!(fs::canonicalize(root).unwrap());
    request["rendered_formula"] = json!(root.join("build/release.formula.toml"));
    request["bd_executable"] = json!(workspace.bd);
    request["parent"] = json!(parent);
    let first = workspace.write_request(&request);
    let applied = advanced_graph_cli(&workspace, "attach", &first);
    assert!(applied.status.success(), "{applied:?}");
    let created: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    let build = created["payload"]["graph"]["ids"]["build"]
        .as_str()
        .unwrap()
        .to_owned();
    workspace.bd_output(&[
        "update",
        &build,
        "--status",
        "in_progress",
        "--notes",
        "keep",
    ]);
    let beads = workspace
        .bd_json(&["list", "--all", "-n", "0", "--json"])
        .as_array()
        .unwrap()
        .len();

    request["compose_variables"] = json!({"version": "1.6.2"});
    let edited = workspace.write_request(&request);
    // Pinned bd 1.3.1 embedded mode cannot probe Dolt state; this case checks
    // applied runtime preservation. Refusal remains covered by revision_cli tests.
    let repoured =
        advanced_graph_cli_with_args(&workspace, "repour", &edited, &["--allow-uncommitted"]);
    assert!(repoured.status.success(), "{repoured:?}");
    let value: serde_json::Value = serde_json::from_slice(&repoured.stdout).unwrap();
    assert_eq!(value["payload"]["outcome"], "succeeded", "{value:#}");
    assert_eq!(value["payload"]["graph"]["ids"]["build"], build.as_str());
    let shown = workspace.bd_json(&["show", &build, "--json"]);
    assert_eq!(shown[0]["title"], "Build 1.6.2");
    assert_eq!(shown[0]["status"], "in_progress");
    assert_eq!(shown[0]["notes"], "keep");
    let after = workspace.bd_json(&["list", "--all", "-n", "0", "--json"]);
    assert_eq!(
        after.as_array().unwrap().len(),
        beads,
        "nothing poured fresh"
    );

    let replay =
        advanced_graph_cli_with_args(&workspace, "repour", &edited, &["--allow-uncommitted"]);
    assert!(replay.status.success(), "{replay:?}");

    // A target that does not exist is refused (exit 2) before any write.
    let before = workspace.snapshot();
    request["parent"] = json!("cli-nope");
    let missing = advanced_graph_cli_with_args(
        &workspace,
        "repour",
        &workspace.write_request(&request),
        &["--allow-uncommitted"],
    );
    assert_eq!(missing.status.code(), Some(2), "{missing:?}");
    let value: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(
        value["payload"]["outcome"]["refused"]["code"],
        "BEADS_GRAPH_PARENT_NOT_FOUND"
    );
    assert_eq!(workspace.snapshot(), before);
}

fn advanced_graph_cli(
    workspace: &GraphCliWorkspace,
    operation: &str,
    request: &Path,
) -> std::process::Output {
    advanced_graph_cli_with_args(workspace, operation, request, &[])
}

fn advanced_graph_cli_with_args(
    workspace: &GraphCliWorkspace,
    operation: &str,
    request: &Path,
    extra_args: &[&str],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args(["bead", operation, "--request"])
        .arg(request)
        .arg("--json")
        .args(extra_args)
        .current_dir(&workspace.fixture.path)
        .env("HOME", workspace.fixture.path.join(".home"))
        .env(
            "XDG_CONFIG_HOME",
            workspace.fixture.path.join(".home/.config"),
        )
        .env("USERPROFILE", workspace.fixture.path.join(".home"))
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .bounded_output()
        .unwrap()
}

#[test]
fn cli_strict_is_explicit_and_schema_scoped() {
    let fixture = TempFixture::new("bead-strict-schema");
    let template = fixture.path.join("strict.formula.toml.j2");
    let output = fixture.path.join("strict.formula.toml");
    fs::write(
        &template,
        "---\nrequired_variables: []\n---\ntitle = \"{{ value }}\"\n",
    )
    .unwrap();
    let request = fixture.path.join("request.json");
    write_file(&request, &json!({
        "schema": BEADS_SCHEMA, "operation": "render", "working_directory": fixture.path,
        "template": template, "rendered_formula": output, "compose_variables": {"value": "allowed"},
        "formula_name": null, "bead_variables": {}, "bd_executable": null, "pour_authorization": null
    }).to_string());
    let run = |strict: bool| {
        let mut command = sc_compose();
        command.args(["bead", "render", "--request"]).arg(&request);
        if strict {
            command.arg("--strict");
        }
        command.bounded_output().unwrap()
    };
    assert!(run(false).status.success());
    assert_eq!(run(true).status.code(), Some(2));
    fs::write(&template, "title = \"{{ value }}\"\n").unwrap();
    assert!(run(true).status.success());
}

fn revision_config(fixture: &TempFixture, scenario: &str, sql: &str, invalid: bool) {
    write_file(&fixture.path.join("revision-bd.json"), &json!({
        "trace": fixture.path.join("trace"), "scenario": scenario, "sql": sql, "invalid": invalid,
    }).to_string());
}

fn revision_cli_fixture(fixture: &TempFixture) -> (PathBuf, PathBuf) {
    let root = &fixture.path;
    let template = root.join("sample.formula.toml.j2");
    let rendered = root.join("sample.formula.toml");
    let trace = root.join("trace");
    let runner = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-support/fake_revision_bd.py");
    let bd = root.join(if cfg!(windows) {
        "fake-revision-bd.cmd"
    } else {
        "fake-revision-bd"
    });
    write_file(&template, "formula = \"sample\"\n");
    revision_config(fixture, "policy", "unknown", false);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&runner, &bd).expect("link committed Python fixture");
    #[cfg(windows)]
    write_file(
        &bd,
        &format!(
            "@echo off\r\npython \"{}\" --config \"{}\" %*\r\nexit /b %errorlevel%\r\n",
            runner.display(),
            root.join("revision-bd.json").display()
        ),
    );
    let request = root.join("request.json");
    write_file(
        &request,
        &json!({
            "schema": BEADS_SCHEMA, "operation":"repour", "working_directory":root,
            "template":template, "rendered_formula":rendered, "compose_variables":{},
            "formula_name":"sample", "bead_variables":{}, "bd_executable":bd,
            "pour_authorization":"CreatePersistentBeads", "parent":"proj-1", "relations":[]
        })
        .to_string(),
    );
    (request, trace)
}

#[test]
fn revision_cli_refuses_unknown_database_and_previews_with_override() {
    let fixture = TempFixture::new("bead-revision-policy");
    let (request, trace) = revision_cli_fixture(&fixture);
    let conflict = sc_compose()
        .args(["bead", "pour", "--request"])
        .arg(&request)
        .args(["--dry-run", "--json"])
        .bounded_output()
        .unwrap();
    assert_eq!(conflict.status.code(), Some(2), "{conflict:?}");
    let body: serde_json::Value = serde_json::from_slice(&conflict.stdout).unwrap();
    assert_eq!(body["payload"]["error"]["code"], "BEADS_GRAPH_CONFLICT");
    assert_eq!(
        body["payload"]["error"]["details"]["reason"],
        "plan_changed"
    );
    assert!(
        body["payload"]["error"]["recovery"]
            .as_str()
            .unwrap()
            .contains("repour")
    );
    let human = sc_compose()
        .args(["bead", "pour", "--request"])
        .arg(&request)
        .arg("--dry-run")
        .bounded_output()
        .unwrap();
    assert_eq!(human.status.code(), Some(2));
    let text = String::from_utf8(human.stderr).unwrap();
    assert!(
        text.contains("plan_changed") && text.contains("repour"),
        "{text}"
    );
    let refused = sc_compose()
        .args(["bead", "repour", "--request"])
        .arg(&request)
        .args(["--dry-run", "--json"])
        .bounded_output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2), "{refused:?}");
    let body: serde_json::Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(
        body["payload"]["error"]["code"],
        "BEADS_GRAPH_DIRTY_UNKNOWN"
    );
    assert!(
        body["payload"]["error"]["recovery"]
            .as_str()
            .unwrap()
            .contains("--allow-uncommitted")
    );
    let preview = sc_compose()
        .args(["bead", "repour", "--request"])
        .arg(&request)
        .args(["--dry-run", "--allow-uncommitted", "--json"])
        .bounded_output()
        .unwrap();
    assert!(preview.status.success(), "{preview:?}");
    let body: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(body["payload"]["operation"], "preview_repour");
    let changes = body["payload"]["graph"]["changes"].as_array().unwrap();
    assert!(changes.contains(&json!({"change":"add_node", "step":"build"})));
    assert!(changes.contains(&json!({"change":"field_change", "bead":"proj-1", "status":"open", "field":"title", "before":"old", "after":"sample"})));
    let commands = fs::read_to_string(&trace).unwrap();
    assert!(
        !commands
            .lines()
            .any(|line| line.starts_with("update ") || line.starts_with("dep add "))
    );
    let commands = fs::read_to_string(trace).unwrap();
    assert!(
        commands
            .lines()
            .any(|line| line.starts_with("create ") && line.contains("--dry-run"))
    );
}

#[test]
fn revision_cli_reports_partial_revision() {
    let fixture = TempFixture::new("bead-revision-partial");
    let (request, _) = revision_cli_fixture(&fixture);
    let partial = sc_compose()
        .args(["bead", "repour", "--request"])
        .arg(&request)
        .args(["--allow-uncommitted", "--json"])
        .bounded_output()
        .unwrap();
    assert_eq!(partial.status.code(), Some(2), "{partial:?}");
    let body: serde_json::Value = serde_json::from_slice(&partial.stdout).unwrap();
    assert_eq!(
        body["payload"]["error"]["code"],
        "BEADS_GRAPH_APPLY_PARTIAL"
    );
    let details = &body["payload"]["error"]["details"];
    assert_eq!(
        details["applied"],
        json!([{"change":"add_node", "step":"build"}])
    );
    assert_eq!(
        details["pending"],
        json!([{"change":"field_change", "bead":"proj-1", "status":"open", "field":"title", "before":"old", "after":"sample"}])
    );
}

#[test]
fn revision_cli_blocks_invalid_graph_path_before_bd() {
    let fixture = TempFixture::new("bead-revision-blocked-path");
    let (request, trace) = revision_cli_fixture(&fixture);
    let blocked = fixture.path.join("sample.formula.toml.graph.json");
    fs::create_dir(&blocked).unwrap();
    let output = sc_compose()
        .args(["bead", "repour", "--request"])
        .arg(&request)
        .args(["--dry-run", "--json"])
        .bounded_output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        body["payload"]["error"]["code"],
        "BEADS_OUTPUT_PATH_INVALID"
    );
    assert_eq!(body["payload"]["error"]["details"]["field"], "graph_plan");
    assert!(!trace.exists(), "a blocked graph path must not invoke bd");
}

#[test]
fn revision_cli_dirty_and_invalid_plans_never_write_without_override() {
    let fixture = TempFixture::new("revision-dirty-validation");
    let (request, trace) = revision_cli_fixture(&fixture);
    revision_config(&fixture, "policy", "dirty", false);
    let run = |args: &[&str]| {
        sc_compose()
            .args(["bead", "repour", "--request"])
            .arg(&request)
            .args(args)
            .arg("--json")
            .bounded_output()
            .unwrap()
    };
    let refused = run(&[]);
    let value: serde_json::Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(value["payload"]["error"]["code"], "BEADS_GRAPH_DIRTY");
    let recovery = value["payload"]["error"]["recovery"].as_str().unwrap();
    assert!(recovery.contains("bd dolt commit") && recovery.contains("--allow-uncommitted"));
    let calls = fs::read_to_string(&trace).unwrap();
    assert!(!calls.lines().any(|line| line.starts_with("create ")
        || line.starts_with("update ")
        || line.starts_with("dep add ")
        || line.starts_with("dep remove ")));
    let overridden = run(&["--allow-uncommitted"]);
    let value: serde_json::Value = serde_json::from_slice(&overridden.stdout).unwrap();
    assert_eq!(
        value["payload"]["error"]["code"],
        "BEADS_GRAPH_APPLY_PARTIAL"
    );
    assert!(
        fs::read_to_string(&trace)
            .unwrap()
            .lines()
            .any(|line| line.starts_with("create "))
    );
    fs::remove_file(&trace).unwrap();
    revision_config(&fixture, "policy", "dirty", true);
    let invalid = run(&["--allow-uncommitted"]);
    let value: serde_json::Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert_eq!(
        value["payload"]["outcome"]["failed"]["code"],
        "BEADS_COOK_FAILED"
    );
    let calls = fs::read_to_string(trace).unwrap();
    assert!(!calls.lines().any(|line| line.starts_with("create ")
        || line.starts_with("update ")
        || line.starts_with("dep add ")
        || line.starts_with("dep remove ")));
}

#[test]
fn revision_help_points_to_both_manual_topics() {
    let output = sc_compose()
        .args(["bead", "repour", "--help"])
        .bounded_output()
        .unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("sc-compose help repour"));
    assert!(help.contains("sc-compose help repour-strategy"));
}

fn configure_revision_graph(fixture: &TempFixture) {
    revision_config(fixture, "graph", "unknown", false);
}

#[test]
fn revision_cli_reports_exact_graph_change_variants() {
    let fixture = TempFixture::new("revision-delta");
    let (request, _) = revision_cli_fixture(&fixture);
    configure_revision_graph(&fixture);
    let output = sc_compose()
        .args(["bead", "repour", "--request"])
        .arg(&request)
        .args(["--dry-run", "--allow-uncommitted", "--json"])
        .bounded_output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let changes = value["payload"]["graph"]["changes"].as_array().unwrap();
    assert!(changes.contains(&json!({"change":"add_node", "step":"build"})));
    assert!(changes.contains(&json!({"change":"add_node", "step":"verify"})));
    assert!(changes.contains(
        &json!({"change":"add_edge", "from":"step:verify", "to":"step:build", "kind":"blocks"})
    ));
    assert!(changes.contains(
        &json!({"change":"remove_internal_edge", "from":"proj-3", "to":"proj-1", "kind":"blocks"})
    ));
    assert!(changes.contains(&json!({"change":"retain_external_edge", "from":"outside-1", "to":"proj-3", "kind":"blocks"})));
    assert!(changes.contains(&json!({"change":"retain_external_edge", "from":"proj-3", "to":"outside-2", "kind":"blocks"})));
    assert!(changes.contains(&json!({"change":"mark_removed_node", "bead":"proj-3", "status":"closed", "remove_command":"bd delete -- 'proj-3'"})));
}
