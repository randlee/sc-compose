use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sc_composer_beads::{BeadOperation, parse_request};
use serde_json::json;

use crate::support::{TempFixture, sc_compose, write_file};

const BEADS_SCHEMA: &str = "sc-compose/beads/v1";

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
    use std::os::unix::fs::PermissionsExt;

    let trace = root.join("bd.trace");
    let executable = root.join("fake-bd");
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$1\" >> '{}'\nif [ \"$1\" = cook ]; then exit {cook_exit}; fi\nif [ \"$1\" = where ]; then printf '%s' '{{\"path\":\"{}\"}}'; fi\nif [ \"$1\" = mol ]; then exit {pour_exit}; fi\nexit 0\n",
            trace.display(),
            root.join(".beads").display()
        ),
    )
    .expect("write fake bd");
    let mut permissions = fs::metadata(&executable)
        .expect("fake bd metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).expect("make fake bd executable");
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
fn unreadable_request_preserves_the_r1_deserialization_code() {
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
        "BEADS_REQUEST_DESERIALIZATION_FAILED"
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
                    "bd dep add {parent}.release-{from} {parent}.release-{to} --type blocks"
                )),
            "{text}"
        );
    }
    assert_eq!(workspace.snapshot(), before, "repair advice writes nothing");
}
