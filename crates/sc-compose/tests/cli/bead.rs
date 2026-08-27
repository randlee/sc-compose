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
    assert_eq!(request.formula_name.as_deref(), Some("toml-workflow"));
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
        .output()
        .expect("run pinned bd validation");

    assert!(command.status.success(), "{command:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&command.stdout).expect("envelope");
    assert_eq!(envelope["payload"]["outcome"], "succeeded");
}
