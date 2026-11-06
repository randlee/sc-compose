#[path = "support/mod.rs"]
mod support;

use std::fs;

use support::{TempFixture, sc_compose, write_file};

#[test]
fn render_output_and_append_can_write_outside_root() {
    let fixture = TempFixture::new("adr0026-d5-output");
    let root = &fixture.path;
    write_file(
        &root.join("text.md.j2"),
        "outside output is caller-selected\n",
    );
    write_file(&root.join("record.json.j2"), "{\"ok\": true}");
    let directory_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .expect("temporary fixture name");
    let output = root
        .parent()
        .expect("temporary fixture parent")
        .join(format!("{directory_name}-outside.md"));
    let appended = root
        .parent()
        .expect("temporary fixture parent")
        .join(format!("{directory_name}-outside.jsonl"));
    assert!(!output.starts_with(root));
    assert!(!appended.starts_with(root));

    let rendered = sc_compose()
        .args(["render", "--mode", "file", "--file", "text.md.j2"])
        .arg("--root")
        .arg(root)
        .arg("--output")
        .arg(&output)
        .output()
        .expect("render outside root");
    assert!(
        rendered.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    assert_eq!(
        fs::read_to_string(&output).expect("rendered output"),
        "outside output is caller-selected"
    );

    let append_result = sc_compose()
        .args(["render", "--mode", "file", "--file", "record.json.j2"])
        .arg("--root")
        .arg(root)
        .arg("--append")
        .arg(&appended)
        .output()
        .expect("append outside root");
    assert!(
        append_result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&append_result.stderr)
    );
    assert_eq!(
        fs::read_to_string(&appended).expect("appended output"),
        "{\"ok\":true}\n"
    );
}

#[test]
fn yaml_12_scalar_contract_preserves_ambiguous_values_as_strings() {
    let fixture = TempFixture::new("adr0026-d11-yaml-scalars");
    let root = &fixture.path;
    write_file(
        &root.join("values.yaml"),
        "leading: 012\nunderscored: 1_000\ntruthy: yes\ndate: 2001-12-14\n",
    );
    write_file(
        &root.join("values.json.j2"),
        "{\"leading\": {{ leading | tojson }}, \"underscored\": {{ underscored | tojson }}, \"truthy\": {{ truthy | tojson }}, \"date\": {{ date | tojson }}}",
    );

    let rendered = sc_compose()
        .args(["render", "--mode", "file", "--file", "values.json.j2"])
        .arg("--root")
        .arg(root)
        .arg("--var-file")
        .arg(root.join("values.yaml"))
        .output()
        .expect("render YAML scalars");
    assert!(
        rendered.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    let output: serde_json::Value = serde_json::from_slice(&rendered.stdout).expect("JSON output");
    assert_eq!(
        output,
        serde_json::json!({
            "leading": "012",
            "underscored": "1_000",
            "truthy": "yes",
            "date": "2001-12-14"
        })
    );
}

#[test]
fn yaml_non_finite_numbers_are_rejected() {
    let fixture = TempFixture::new("adr0026-d11-yaml-non-finite");
    let root = &fixture.path;
    write_file(&root.join("value.json.j2"), "{}\n");

    for scalar in [".inf", ".nan"] {
        let vars = root.join(format!("{}.yaml", scalar.trim_start_matches('.')));
        write_file(&vars, &format!("value: {scalar}\n"));
        let rendered = sc_compose()
            .args(["render", "--mode", "file", "--file", "value.json.j2"])
            .arg("--root")
            .arg(root)
            .arg("--var-file")
            .arg(&vars)
            .output()
            .expect("reject non-finite YAML number");

        assert_eq!(rendered.status.code(), Some(3), "scalar: {scalar}");
        assert!(
            String::from_utf8_lossy(&rendered.stderr).contains("ERR_VAL_OBJECT_SHAPE"),
            "scalar: {scalar}; stderr: {}",
            String::from_utf8_lossy(&rendered.stderr)
        );
    }
}
