//! CLI regression tests promoted from the Phase T adversarial fuzz campaign.

use super::support::{parse_stdout, sc_compose, temp_root, write_file};

/// Writes a `bead render` request for `template` and returns its absolute path.
fn write_bead_render_request(root: &std::path::Path, template: &str) -> std::path::PathBuf {
    let request = root.join("request.json");
    write_file(
        &request,
        &serde_json::json!({
            "schema": "sc-compose/beads/v1",
            "operation": "render",
            "working_directory": root,
            "template": template,
            "rendered_formula": root.join("build/out.formula.toml"),
            "compose_variables": {},
            "bead_variables": {}
        })
        .to_string(),
    );
    std::fs::create_dir_all(root.join("build")).unwrap();
    request
}

// FUZZ-039: human graph refusals retain the typed diagnostic recorded in the receipt.
#[test]
fn fuzz_039_human_preview_attach_prints_parent_refusal_reason() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE") else {
        return;
    };
    let root = temp_root("fuzz-039-human-graph-refusal");
    let beads_dir = root.join(".beads");
    let initialized = std::process::Command::new(bd)
        .args([
            "init",
            "--non-interactive",
            "--quiet",
            "--skip-agents",
            "--skip-hooks",
        ])
        .current_dir(&root)
        .env("BEADS_DIR", &beads_dir)
        .output()
        .unwrap();
    assert!(initialized.status.success(), "{initialized:?}");
    write_file(
        &root.join("m.formula.toml.j2"),
        "formula = \"m\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"a\"\ntitle = \"A\"\n",
    );
    let request = root.join("request.json");
    write_file(&request, &serde_json::json!({"schema":"sc-compose/beads/v1","operation":"preview_attach","working_directory":root,"template":"m.formula.toml.j2","rendered_formula":root.join("out.formula.toml"),"compose_variables":{},"bead_variables":{},"parent":"nosuch","ref":"r"}).to_string());
    let output = sc_compose()
        .args(["bead", "preview-attach", "--request"])
        .arg(&request)
        .current_dir(&root)
        .env("BEADS_DIR", &beads_dir)
        .env("BEADS_NO_DAEMON", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(
        human.contains("parent bead `nosuch` was not found"),
        "{human}"
    );
}

// FUZZ-012: a relative template resolves against working_directory.
#[test]
fn fuzz_012_bead_request_template_is_relative_to_working_directory() {
    let root = std::fs::canonicalize(temp_root("fuzz-012-bead-relative-template")).unwrap();
    write_file(
        &root.join("wf/m.formula.toml.j2"),
        "formula = \"m\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"a\"\ntitle = \"A\"\n",
    );
    let request = write_bead_render_request(&root, "wf/m.formula.toml.j2");
    let elsewhere = root.join("other");
    std::fs::create_dir_all(&elsewhere).unwrap();

    let output = sc_compose()
        .current_dir(&elsewhere)
        .args(["bead", "render", "--json", "--request"])
        .arg(&request)
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(root.join("build/out.formula.toml").is_file());
}

// FUZZ-017: append preserves exact JSON number values.
#[test]
fn fuzz_017_render_append_keeps_exact_number_values() {
    let root = temp_root("fuzz-017-append-number-fidelity");
    write_file(
        &root.join("rec.json.j2"),
        "{\"id\": 12345678901234567890123, \"amount\": 0.10000000000000000001}",
    );
    let destination = root.join("log.jsonl");

    let output = sc_compose()
        .args(["render", "--file", "rec.json.j2", "--root"])
        .arg(&root)
        .arg("--append")
        .arg(&destination)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let line = std::fs::read_to_string(&destination).unwrap();
    assert!(
        line.contains("\"id\":12345678901234567890123")
            && line.contains("\"amount\":0.10000000000000000001"),
        "appended record changed a value: {line}"
    );
}

// FUZZ-018: bead render expands sc-compose @<path> includes.
#[test]
fn fuzz_018_bead_render_expands_at_path_includes() {
    let root = std::fs::canonicalize(temp_root("fuzz-018-bead-include")).unwrap();
    write_file(
        &root.join("main.formula.toml.j2"),
        "formula = \"inc\"\nversion = 1\ntype = \"workflow\"\n@<frag/common.toml.j2>\n[[steps]]\nid = \"local\"\ntitle = \"Local\"\n",
    );
    write_file(
        &root.join("frag/common.toml.j2"),
        "[[steps]]\nid = \"shared\"\ntitle = \"Shared step\"\n",
    );
    let request = write_bead_render_request(&root, "main.formula.toml.j2");

    let output = sc_compose()
        .current_dir(&root)
        .args(["bead", "render", "--json", "--request"])
        .arg(&request)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let rendered = std::fs::read_to_string(root.join("build/out.formula.toml")).unwrap();
    assert!(rendered.contains("id = \"shared\"") && !rendered.contains("@<"));
}

// FUZZ-019: failed bead renders retain template diagnostics.
#[test]
fn fuzz_019_bead_render_failure_reports_the_template_error() {
    let root = std::fs::canonicalize(temp_root("fuzz-019-bead-render-message")).unwrap();
    write_file(&root.join("bad.formula.toml.j2"), "{% if %}\n");
    let request = write_bead_render_request(&root, "bad.formula.toml.j2");

    let output = sc_compose()
        .current_dir(&root)
        .args(["bead", "render", "--json", "--request"])
        .arg(&request)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let receipt = parse_stdout(&output);
    let stage = &receipt["payload"]["stages"][0];
    assert_eq!(stage["outcome"]["failed"]["code"], "BEADS_RENDER_FAILED");
    let diagnostic = stage["stderr_excerpt"].as_str().unwrap();
    let cause = diagnostic
        .strip_prefix("formula rendering failed:")
        .expect("render diagnostic prefix");
    assert!(!cause.trim().is_empty(), "{diagnostic}");

    let output = sc_compose()
        .current_dir(&root)
        .args(["bead", "render", "--request"])
        .arg(&request)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(human.contains(&format!(
        "stage Render: failed (BEADS_RENDER_FAILED): {diagnostic}"
    )));
}

// FUZZ-021: graph-id validation failures are exit 2 with typed JSON details.
#[test]
fn fuzz_021_invalid_attach_ref_is_typed_validation_error() {
    let root = temp_root("fuzz-021-typed-ref");
    for operation in ["attach", "preview-attach"] {
        for reference in ["a.b", "", &"x".repeat(33)] {
            let request = root.join("request.json");
            write_file(
                &request,
                &serde_json::json!({
                    "schema":"sc-compose/beads/v1", "operation":operation.replace('-', "_"),
                    "working_directory":root, "template":"missing.formula.toml.j2",
                    "rendered_formula":root.join("out.formula.toml"),
                    "compose_variables":{}, "bead_variables":{}, "parent":"proj-1", "ref":reference
                })
                .to_string(),
            );
            let output = sc_compose()
                .args(["bead", operation, "--json", "--request"])
                .arg(&request)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            let payload = parse_stdout(&output);
            assert_eq!(
                payload["payload"]["error"]["code"],
                "BEADS_GRAPH_ID_INVALID"
            );
            assert_eq!(
                payload["payload"]["error"]["details"],
                serde_json::json!({"field":"ref", "value":reference})
            );
        }
    }
}

// FUZZ-038: native request paths must produce typed usage errors, never serializer panics.
#[cfg(unix)]
#[test]
fn fuzz_038_non_utf8_request_paths_are_typed_errors_in_json_and_human_modes() {
    use std::os::unix::ffi::OsStringExt;

    let root = temp_root("fuzz-038-non-utf8-request-path");
    let request = root.join(std::ffi::OsString::from_vec(b"missing-\xff.json".to_vec()));
    for operation in [
        "render",
        "validate",
        "preview-pour",
        "pour",
        "preview-attach",
        "attach",
    ] {
        for json in [false, true] {
            let mut command = sc_compose();
            command.args(["bead", operation, "--request"]).arg(&request);
            if json {
                command.arg("--json");
            }
            let output = command.output().expect("CLI");
            assert_eq!(
                output.status.code(),
                Some(3),
                "{operation}, JSON={json}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if json {
                let envelope = parse_stdout(&output);
                let error = &envelope["payload"]["error"];
                assert_eq!(error["code"], "BEADS_REQUEST_READ_FAILED");
                assert_eq!(error["details"]["path"], request.to_string_lossy().as_ref());
                assert_eq!(error["details"]["kind"], "NotFound");
                assert!(!error["recovery"].as_str().expect("recovery").is_empty());
            } else {
                assert!(output.stdout.is_empty());
                let stderr = String::from_utf8(output.stderr).expect("human diagnostic is UTF-8");
                assert!(stderr.contains("BEADS_REQUEST_READ_FAILED:"), "{stderr}");
                assert!(
                    stderr.contains(request.to_string_lossy().as_ref()),
                    "{stderr}"
                );
                assert!(stderr.contains("recovery:"), "{stderr}");
            }
        }
    }
}

// FUZZ-043: valid JSON beyond the supported nesting limit is not malformed.
#[test]
fn fuzz_043_json_depth_limit_is_typed_and_append_preserves_output() {
    let root = temp_root("fuzz-043-json-depth");
    let template = root.join("deep.json.j2");
    let destination = root.join("records.jsonl");
    let previous = b"{\"previous\":true}\n";
    std::fs::write(&destination, previous).unwrap();
    write_file(
        &template,
        &format!("{}1{}", "{\"a\":".repeat(128), "}".repeat(128)),
    );
    for append in [false, true] {
        let mut command = sc_compose();
        command
            .args(["render", "--json", "--file"])
            .arg(&template)
            .arg("--root")
            .arg(&root);
        if append {
            command.arg("--append").arg(&destination);
        } else {
            command.arg("--output").arg(&destination);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let envelope = parse_stdout(&output);
        let diagnostics = envelope["diagnostics"].as_array().unwrap();
        assert!(
            diagnostics
                .iter()
                .any(|d| d["code"] == "ERR_RENDER_JSON_DEPTH_LIMIT"),
            "{envelope}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d["message"].as_str().unwrap().contains("127")),
            "{envelope}"
        );
        assert_eq!(std::fs::read(&destination).unwrap(), previous);
    }
}

// FUZZ-017 round 2: nested raw values must still produce exactly one physical JSONL line.
#[test]
fn fuzz_017_nested_raw_values_append_as_one_line_without_changing_lexemes() {
    let root = temp_root("fuzz-017-nested-one-line-record");
    let template = r#"{
  "nested": {
    "array": [
      12345678901234567890123,
      { "decimal": 0.10000000000000000001, "exponent": -1.2300E+04 },
      [ 4.20e-03, "spaces stay  here", "escaped\nline\tand\rreturn", "quote: \" then \\" ]
    ],
    "escaped": "\u0061\/b",
    "after": { "value": true }
  }
}"#;
    write_file(
        &root.join("nested.json.j2"),
        &template.replace('\n', "\r\n").replace("    ", "\t"),
    );
    let destination = root.join("records.jsonl");
    let existing = "{\"existing\":true}\n";
    write_file(&destination, existing);
    let output = sc_compose()
        .args(["render", "--file", "nested.json.j2", "--root"])
        .arg(&root)
        .arg("--append")
        .arg(&destination)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let contents = std::fs::read_to_string(&destination).unwrap();
    assert!(contents.starts_with(existing), "existing records changed");
    let appended = &contents[existing.len()..];
    assert_eq!(
        appended.bytes().filter(|byte| *byte == b'\n').count(),
        1,
        "nested whitespace leaked into JSONL: {appended}"
    );
    assert!(appended.ends_with('\n'));
    let expected = concat!(
        r#"{"nested":{"array":[12345678901234567890123,{"decimal":0.10000000000000000001,"exponent":-1.2300E+04},"#,
        r#"[4.20e-03,"spaces stay  here","escaped\nline\tand\rreturn","quote: \" then \\"]],"#,
        r#""escaped":"\u0061\/b","after":{"value":true}}}"#,
        "\n"
    );
    assert_eq!(
        appended, expected,
        "numeric or escaped-string lexemes changed"
    );
    let envelope = parse_stdout(&output);
    assert_eq!(envelope["payload"]["bytes_written"], appended.len());
    assert_eq!(envelope["payload"]["appended"], true);
}
