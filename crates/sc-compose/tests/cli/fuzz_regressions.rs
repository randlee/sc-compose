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
#[ignore = "FUZZ-017"]
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
