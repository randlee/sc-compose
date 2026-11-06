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
        .output()
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
                    "name": "sc-compose",
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
    let output = Command::new(bd)
        .args([
            "init",
            "--non-interactive",
            "--quiet",
            "--skip-agents",
            "--skip-hooks",
        ])
        .env("BEADS_NO_DAEMON", "1")
        .env("BEADS_DIR", root.join(".beads"))
        .current_dir(root)
        .output()
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
        .output()
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
        .output()
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
        .output()
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
        .output()
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
        .output()
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
        .output()
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
        .output()
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
        .output()
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
        .output()
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
        .output()
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
fn pinned_bd_validates_the_canonical_cli_fixture_when_configured() {
    let Some(pinned_bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping pinned bd CLI integration: BD_EXECUTABLE is not configured");
        return;
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
        .env("BEADS_DIR", fixture.path.join(".beads"))
        .output()
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
        .output()
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
        .output()
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
        let workspace = Self {
            fixture,
            bd,
            beads_dir,
        };
        workspace.bd_output(&[
            "init",
            "--non-interactive",
            "--quiet",
            "--skip-agents",
            "--skip-hooks",
            "--prefix",
            "cli",
        ]);
        fs::create_dir_all(workspace.fixture.path.join("build")).expect("build directory");
        workspace
    }

    fn bd_output(&self, args: &[&str]) -> String {
        let output = Command::new(&self.bd)
            .args(args)
            .current_dir(&self.fixture.path)
            .env("BEADS_DIR", &self.beads_dir)
            .env("BEADS_NO_DAEMON", "1")
            .output()
            .expect("fixture bd");
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
            .env("BEADS_DIR", &self.beads_dir)
            .env("BEADS_NO_DAEMON", "1");
        if json_output {
            command.arg("--json");
        }
        let output = command.output().expect("bead CLI");
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
fn direct_plan_fix_pour_persists_values_and_declared_dependency() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping direct pour integration: BD_EXECUTABLE is not configured");
        return;
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
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
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
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
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
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
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
            .env("BEADS_DIR", &workspace.beads_dir)
            .env("BEADS_NO_DAEMON", "1")
            .output()
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
fn pinned_bd_graph_pour_and_attach_run_the_release_example_with_text_and_exit_contracts() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping graph CLI integration: BD_EXECUTABLE is not configured");
        return;
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
        .output()
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
            .output()
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
        .output()
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
    let (fake_bd, _trace) = write_fake_bd(root, 1, 0);
    fs::create_dir(root.join("fake-bin")).expect("fake-bin");
    fs::copy(&fake_bd, root.join("fake-bin/bd")).expect("install fake bd on PATH");
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
        let output = command.output().expect("run direct pour");
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
        .output()
        .expect("run request render");
    assert!(rendered.status.success(), "{rendered:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&rendered.stdout).expect("envelope");
    assert!(envelope["payload"].get("ephemeral_paths").is_none());
    assert!(assert_receipt_paths_exist_or_are_ephemeral(&envelope["payload"]).is_empty());
}

#[test]
fn direct_bd_owned_registry_pour_passes_variables_to_bd() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping direct bd-owned integration: BD_EXECUTABLE is not configured");
        return;
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
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
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
fn direct_named_user_registry_snapshots_confined_includes() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping external registry integration: BD_EXECUTABLE is not configured");
        return;
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
        .env("GT_ROOT", &home.path)
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
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
        .env("GT_ROOT", &home.path)
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
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
        .env("GT_ROOT", home)
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn advanced_attach_missing_edge_previews_and_refuses_unqualified_apply_without_mutation() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping atomic refusal integration: BD_EXECUTABLE is not configured");
        return;
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
    let refused = advanced_graph_cli(&workspace, "attach", &request);
    assert_eq!(refused.status.code(), Some(2), "{refused:?}");
    let value: serde_json::Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(
        value["payload"]["outcome"]["refused"]["code"],
        "BEADS_GRAPH_ATOMIC_UPDATE_UNAVAILABLE"
    );
    assert_eq!(workspace.snapshot(), before);
}

fn advanced_graph_cli(
    workspace: &GraphCliWorkspace,
    operation: &str,
    request: &Path,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args(["bead", operation, "--request"])
        .arg(request)
        .arg("--json")
        .current_dir(&workspace.fixture.path)
        .env("BEADS_DIR", &workspace.beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
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
        command.output().unwrap()
    };
    assert!(run(false).status.success());
    assert_eq!(run(true).status.code(), Some(2));
    fs::write(&template, "title = \"{{ value }}\"\n").unwrap();
    assert!(run(true).status.success());
}
