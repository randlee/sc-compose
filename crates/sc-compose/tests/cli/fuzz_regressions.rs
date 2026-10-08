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

#[cfg(unix)]
fn json_contains_controls(value: &serde_json::Value, controls: &str) -> bool {
    match value {
        serde_json::Value::String(text) => controls.chars().all(|control| text.contains(control)),
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_contains_controls(value, controls)),
        serde_json::Value::Object(values) => values
            .values()
            .any(|value| json_contains_controls(value, controls)),
        _ => false,
    }
}

// FUZZ-039: human graph refusals expose canonical structured recovery fields.
#[cfg(unix)]
#[test]
fn fuzz_039_human_preview_attach_prints_parent_refusal_reason() {
    let request = human_graph_request("fuzz-039-human-graph-refusal", "[]", "", 0);
    let output = sc_compose()
        .args(["bead", "preview-attach", "--request"])
        .arg(&request)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let human = String::from_utf8(output.stdout).unwrap();
    let error = sc_composer_beads::BeadComposeError::GraphParentNotFound {
        parent: sc_composer_beads::BeadId::new("nosuch").unwrap(),
    };
    let envelope = serde_json::to_value(&error).unwrap();
    assert!(human.contains(&error.to_string()), "{human}");
    assert!(
        human.contains(&format!("details: {}", envelope["details"])),
        "{human}"
    );
    assert!(
        human.contains(&format!(
            "recovery: {}",
            envelope["recovery"].as_str().unwrap()
        )),
        "{human}"
    );
}

#[cfg(unix)]
fn human_graph_request(label: &str, stdout: &str, stderr: &str, status: i32) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_root(label);
    let bd = root.join("fake-bd");
    let cooked = r#"{"formula":"m","type":"workflow","steps":[{"id":"a","title":"A"}]}"#;
    write_file(
        &bd,
        &format!(
            "#!/bin/sh\ncase \"$1\" in\n cook) printf '%s' '{cooked}' ;;\n show) printf '%s' '{stdout}'; printf '%s' '{stderr}' >&2; exit {status} ;;\nesac\n"
        ),
    );
    std::fs::set_permissions(&bd, std::fs::Permissions::from_mode(0o755)).unwrap();
    write_file(&root.join("m.formula.toml.j2"), "formula = \"m\"\n");
    let request = root.join("request.json");
    write_file(&request, &serde_json::json!({"schema":"sc-compose/beads/v1","operation":"preview_attach","working_directory":root,"template":"m.formula.toml.j2","rendered_formula":root.join("out.formula.toml"),"compose_variables":{},"bead_variables":{},"parent":"nosuch","ref":"r","bd_executable":bd}).to_string());
    request
}

#[cfg(unix)]
#[test]
fn fuzz_039_human_receipts_preserve_conflict_id_and_read_cause() {
    let cases = [
        (
            "conflict",
            r#"[{"id":"nosuch"},{"id":"nosuch.r-a","title":"A"}]"#,
            "",
            0,
            "BEADS_GRAPH_CONFLICT",
            "\"id\":\"nosuch.r-a\"",
        ),
        (
            "read",
            "",
            r#"{"error":"permission denied"}"#,
            2,
            "BEADS_GRAPH_READ_FAILED",
            "\"cause\":\"permission denied\"",
        ),
    ];
    for (label, stdout, stderr, status, code, detail) in cases {
        let request = human_graph_request(label, stdout, stderr, status);
        let human = sc_compose()
            .args(["bead", "preview-attach", "--request"])
            .arg(&request)
            .output()
            .unwrap();
        assert_eq!(human.status.code(), Some(2), "{human:?}");
        let text = String::from_utf8(human.stdout).unwrap();
        assert!(text.contains(code), "{text}");
        assert!(text.contains("details:"), "{text}");
        assert!(text.contains(detail), "{text}");
        assert!(text.contains("recovery:"), "{text}");
        let json = sc_compose()
            .args(["bead", "preview-attach", "--request"])
            .arg(&request)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(json.status.code(), Some(2));
        let envelope: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
        assert!(envelope["payload"].get("diagnostics").is_none());
        assert!(envelope["payload"].get("error").is_none());
        assert_eq!(
            envelope["payload"]["outcome"]["refused"]["code"]
                .as_str()
                .or_else(|| envelope["payload"]["outcome"]["failed"]["code"].as_str()),
            Some(code)
        );
    }
}

// FUZZ-053: successful graph stages never print their bd stderr in human output.
#[cfg(unix)]
#[test]
fn fuzz_053_human_preview_attach_suppresses_successful_stage_stderr() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_root("fuzz-053-human-success-stderr");
    let bd = root.join("fake-bd");
    let cooked = r#"{"formula":"m","type":"workflow","steps":[{"id":"a","title":"A"}]}"#;
    write_file(
        &bd,
        &format!(
            "#!/bin/sh\ncase \"$1\" in\n cook) printf '%s' '{cooked}' ;;\n show) printf '%s' '[{{\"id\":\"proj-1\"}}]'; printf '%s' 'Hint: harmless' >&2 ;;\nesac\n"
        ),
    );
    std::fs::set_permissions(&bd, std::fs::Permissions::from_mode(0o755)).unwrap();
    write_file(
        &root.join("m.formula.toml.j2"),
        "formula = \"m\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"a\"\ntitle = \"A\"\n",
    );
    let request = root.join("request.json");
    write_file(&request, &serde_json::json!({"schema":"sc-compose/beads/v1","operation":"preview_attach","working_directory":root,"template":"m.formula.toml.j2","rendered_formula":root.join("out.formula.toml"),"compose_variables":{},"bead_variables":{},"parent":"proj-1","ref":"r","bd_executable":bd}).to_string());
    let output = sc_compose()
        .args(["bead", "preview-attach", "--request"])
        .arg(&request)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(!human.contains("Hint: harmless"), "{human}");
    assert_eq!(
        human
            .lines()
            .filter(|line| line.starts_with("stage "))
            .count(),
        4,
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
                    "compose_variables":{}, "bead_variables":{}, "parent":"proj-1", "ref":reference,
                    "pour_authorization":"CreatePersistentBeads"
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
                payload["payload"]["outcome"],
                serde_json::json!({"refused":{"code":"BEADS_GRAPH_ID_INVALID"}})
            );
            let receipt: sc_composer_beads::BeadComposeReceipt =
                serde_json::from_value(payload["payload"].clone()).unwrap();
            assert!(matches!(
                receipt.outcome,
                sc_composer_beads::BeadOutcome::Refused { .. }
            ));

            assert_eq!(
                payload["payload"]["error"]["code"],
                "BEADS_GRAPH_ID_INVALID"
            );
            assert_eq!(
                payload["payload"]["error"]["details"],
                serde_json::json!({"field":"ref", "value":reference, "rule":"ref is [A-Za-z0-9_-]{1,32}"})
            );
        }
    }
}

// FUZZ-040: human refused-attach diagnostics escape terminal controls in every id field.
#[cfg(unix)]
#[test]
fn fuzz_040_human_attach_refusals_escape_identifier_controls_without_changing_json() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root("fuzz-040-human-identifier-controls");
    let bd = root.join("fake-bd");
    write_file(
        &bd,
        "#!/bin/sh\ncase \"$1\" in\n cook) printf '%s' '{\"formula\":\"m\",\"type\":\"workflow\",\"steps\":[{\"id\":\"a\",\"title\":\"A\"}]}' ;;\n show) if [ \"$4\" = proj-1 ]; then printf '%s' '[{\"id\":\"proj-1\"}]'; else printf '%s' '[]'; fi ;;\nesac\n",
    );
    std::fs::set_permissions(&bd, std::fs::Permissions::from_mode(0o755)).unwrap();
    write_file(
        &root.join("m.formula.toml.j2"),
        "formula = \"m\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"a\"\ntitle = \"A\"\n",
    );
    let controls = "\u{202e}\u{200b}\u{feff}\u{0007}";
    let cases = [
        (
            "parent",
            serde_json::json!({"parent": format!("proj-{controls}")}),
        ),
        ("ref", serde_json::json!({"ref": format!("ref-{controls}")})),
        (
            "relation",
            serde_json::json!({"relations": [{"from": format!("step:{controls}"), "to": "bead:proj-1", "type": "blocks"}]}),
        ),
    ];

    for (field, override_fields) in cases {
        let request = root.join(format!("{field}.json"));
        let mut input = serde_json::json!({
            "schema":"sc-compose/beads/v1", "operation":"attach",
            "working_directory":root, "template":"m.formula.toml.j2",
            "rendered_formula":root.join("out.formula.toml"),
            "compose_variables":{}, "bead_variables":{}, "parent":"proj-1", "ref":"valid",
            "relations":[], "pour_authorization":"CreatePersistentBeads", "bd_executable":bd
        });
        for (name, value) in override_fields.as_object().unwrap() {
            input[name] = value.clone();
        }
        write_file(&request, &input.to_string());

        let human = sc_compose()
            .args(["bead", "attach", "--request"])
            .arg(&request)
            .output()
            .unwrap();
        assert_eq!(human.status.code(), Some(2), "{human:?}");
        let human = format!(
            "{}{}",
            String::from_utf8(human.stdout).unwrap(),
            String::from_utf8(human.stderr).unwrap()
        );
        for raw in ['\u{202e}', '\u{200b}', '\u{feff}', '\u{0007}'] {
            assert!(!human.contains(raw), "{field}: {human:?}");
        }
        for escaped in ["\\u{202E}", "\\u{200B}", "\\u{FEFF}", "\\u{0007}"] {
            assert!(human.contains(escaped), "{field}: {human:?}");
        }

        let json = sc_compose()
            .args(["bead", "attach", "--json", "--request"])
            .arg(&request)
            .output()
            .unwrap();
        assert_eq!(json.status.code(), Some(2), "{json:?}");
        let envelope = parse_stdout(&json);
        assert!(envelope["payload"]["outcome"].get("refused").is_some());
        assert!(
            json_contains_controls(&envelope, controls),
            "{field}: JSON must retain the original identifier: {envelope}"
        );
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

// FUZZ-017 round 3: JSON number grammar is not limited by floating-point range.
#[test]
fn fuzz_017_append_preserves_large_exponent_lexemes() {
    let root = temp_root("fuzz-017-large-exponent");
    write_file(
        &root.join("rec.json.j2"),
        r#"{"n":1e400,"tiny":-1.2300e-4000}"#,
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
    assert_eq!(
        std::fs::read_to_string(destination).unwrap(),
        "{\"n\":1e400,\"tiny\":-1.2300e-4000}\n"
    );
}

#[test]
fn fuzz_052_request_errors_take_precedence_over_invalid_ids() {
    let root = temp_root("fuzz-052-request-precedence");
    let valid = serde_json::json!({
        "schema":"sc-compose/beads/v1", "operation":"preview_attach",
        "working_directory":root, "template":"missing.formula.toml.j2",
        "rendered_formula":root.join("out.formula.toml"), "compose_variables":{},
        "bead_variables":{}, "parent":"proj-1", "ref":"valid", "relations":[]
    });
    let mut unknown = valid.clone();
    unknown["operation"] = serde_json::json!("bogus");
    unknown["relations"] =
        serde_json::json!([{"from":"step:a", "to":"bead:bad id", "type":"blocks"}]);
    let mut unauthorized = valid.clone();
    unauthorized["operation"] = serde_json::json!("attach");
    unauthorized["ref"] = serde_json::json!("a.b");
    let mut wrong_type = valid.clone();
    wrong_type["compose_variables"] = serde_json::json!([]);
    wrong_type["parent"] = serde_json::json!("bad id");
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("template");
    missing["ref"] = serde_json::json!("a.b");
    let mut id_only = valid.clone();
    id_only["ref"] = serde_json::json!("a.b");
    for (case, code, exit) in [
        (unknown, "BEADS_REQUEST_DESERIALIZATION_FAILED", 3),
        (unauthorized, "BEADS_POUR_AUTH_REQUIRED", 3),
        (wrong_type, "BEADS_REQUEST_DESERIALIZATION_FAILED", 3),
        (missing, "BEADS_REQUEST_DESERIALIZATION_FAILED", 3),
        (id_only, "BEADS_GRAPH_ID_INVALID", 2),
    ] {
        let request = root.join("request.json");
        write_file(&request, &case.to_string());
        let command = if case["operation"] == "attach" {
            "attach"
        } else {
            "preview-attach"
        };
        let output = sc_compose()
            .args(["bead", command, "--request"])
            .arg(&request)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(exit), "{case}: {output:?}");
        let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            envelope["payload"]["error"]["code"], code,
            "{case}: {envelope}"
        );
        if exit == 2 {
            assert_eq!(
                envelope["payload"]["error"]["details"],
                serde_json::json!({"field":"ref", "value":"a.b", "rule":"ref is [A-Za-z0-9_-]{1,32}"})
            );
            assert!(
                envelope["payload"]["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("[A-Za-z0-9_-]")
            );
        }
    }
}

// The CLI subcommand, not the request file's `operation`, decides which
// operation-dependent parse rules apply (docs/manual/bead.md).
#[test]
fn pe_f6_subcommand_decides_operation_for_authorization_and_id_errors() {
    let root = temp_root("pe-f6-subcommand-operation");
    let base = serde_json::json!({
        "schema":"sc-compose/beads/v1", "operation":"preview_attach",
        "working_directory":root, "template":"missing.formula.toml.j2",
        "rendered_formula":root.join("out.formula.toml"), "compose_variables":{},
        "bead_variables":{}, "parent":"proj-1", "ref":"a.b", "relations":[]
    });
    let mut bad_parent = base.clone();
    bad_parent["operation"] = serde_json::json!("render");
    bad_parent["parent"] = serde_json::json!("bad id");
    bad_parent["ref"] = serde_json::Value::Null;
    // (request, subcommand, expected code, expected exit)
    for (case, command, code, exit) in [
        // Request file says preview_attach; `bead attach` needs authorization.
        (base.clone(), "attach", "BEADS_POUR_AUTH_REQUIRED", 3),
        // Same file as `bead preview-attach` stays an identifier refusal.
        (base.clone(), "preview-attach", "BEADS_GRAPH_ID_INVALID", 2),
        // A bad parent outside the attach family is a request error (exit
        // 3) that keeps its native typed identifier error.
        (bad_parent, "render", "BEADS_GRAPH_ID_INVALID", 3),
    ] {
        let request = root.join("request.json");
        write_file(&request, &case.to_string());
        let output = sc_compose()
            .args(["bead", command, "--request"])
            .arg(&request)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(exit), "{command}: {output:?}");
        let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let reported = envelope["payload"]["error"]["code"]
            .as_str()
            .or_else(|| envelope["payload"]["error"]["code"].as_str());
        assert_eq!(reported, Some(code), "{command}: {envelope}");
    }
}

#[cfg(unix)]
#[test]
fn fuzz_021_cooked_step_refusal_has_canonical_error_and_legacy_receipt() {
    for command in ["attach", "preview-attach"] {
        let request = human_graph_request("fuzz-021-cooked-step", "[]", "", 0);
        let mut input: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&request).unwrap()).unwrap();
        input["operation"] = serde_json::json!(command.replace('-', "_"));
        input["pour_authorization"] = serde_json::json!("CreatePersistentBeads");
        write_file(&request, &input.to_string());
        let bd = std::path::Path::new(input["bd_executable"].as_str().unwrap());
        let script = std::fs::read_to_string(bd)
            .unwrap()
            .replace("\"id\":\"a\"", "\"id\":\"bad-step\"");
        write_file(bd, &script);
        for json in [true, false] {
            let mut process = sc_compose();
            process.args(["bead", command, "--request"]).arg(&request);
            if json {
                process.arg("--json");
            }
            let output = process.output().unwrap();
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            if json {
                let envelope = parse_stdout(&output);
                let payload = &envelope["payload"];
                assert_eq!(
                    payload["error"]["details"],
                    serde_json::json!({"field":"step","value":"bad-step","rule":"step is [A-Za-z0-9_]{1,64}; hyphens are forbidden"})
                );
                let receipt: sc_composer_beads::BeadComposeReceipt =
                    serde_json::from_value(payload.clone()).unwrap();
                assert!(matches!(
                    receipt.outcome,
                    sc_composer_beads::BeadOutcome::Refused { .. }
                ));
            } else {
                let output = String::from_utf8(output.stdout).unwrap();
                assert!(
                    output.contains("bad-step") && output.contains("hyphens are forbidden"),
                    "{output}"
                );
            }
        }
    }
}
