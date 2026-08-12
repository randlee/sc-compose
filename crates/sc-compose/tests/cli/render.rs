//! Capability-oriented integration tests. Shared mechanics live in `tests/support`.
#![allow(
    unused_imports,
    reason = "shared support imports are selected by platform and test configuration"
)]
use crate::support::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[test]
fn render_dry_run_does_not_create_output_file() {
    let root = temp_root("dry-run");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    let output = root.join("out.md");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&output)
        .arg("--dry-run")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(!output.exists());
}

#[test]
fn exit_code_zero_for_valid_render() {
    let root = temp_root("exit-ok");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(0));
}

#[test]
fn exit_code_two_for_validation_failure() {
    let root = temp_root("exit-validation");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let status = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));
}

#[test]
fn render_rejects_scalar_for_existing_jagged_array_fixture() {
    let root = temp_root("jagged-array-scalar");
    write_file(
        &root.join("jagged-array-values.md.j2"),
        include_str!("../../../../examples/jagged-array-values.md.j2"),
    );
    let vars_file = root.join("vars.json");
    write_file(&vars_file, r#"{"rows":"ab"}"#);

    let output = sc_compose()
        .args([
            "render",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "jagged-array-values.md.j2",
            "--var-file",
            vars_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERR_VAL_ARRAY_SHAPE_MISMATCH"), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "unexpected output: {:?}",
        output.stdout
    );
}

#[test]
fn exit_code_three_for_resolve_failure() {
    let root = temp_root("exit-resolve");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("missing.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(3));
}

#[test]
fn empty_custom_variable_delimiters_report_source_once() {
    let root = temp_root("empty-custom-delimiters");
    write_file(&root.join("template.md.j2"), "hello\n");

    let output = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "template.md.j2",
            "--variable-delimiters",
            "",
            "",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(3), "stderr: {stderr}");
    assert_eq!(stderr.matches("invalid custom delimiters").count(), 1);
    assert!(
        stderr.contains("ERR_CONFIG_PARSE: template rendering failed: invalid custom delimiters"),
        "stderr: {stderr}"
    );
}

#[test]
fn render_xml_template_escapes_interpolated_special_characters() {
    let root = temp_root("xml-autoescape");
    write_file(
        &root.join("repro.xml.j2"),
        "---\nname: repro\nversion: 1.0.0\ndescription: XML escaping regression\nformat: xml\nrequired_variables:\n  - note\n---\n<root>\n  <note>{{ note }}</note>\n</root>\n",
    );
    write_file(
        &root.join("vars.json"),
        r#"{"note":"record with <tag> & \"quotes\" & 'apostrophe' & ampersand&here"}"#,
    );

    let output = sc_compose()
        .args([
            "render",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "repro.xml.j2",
            "--var-file",
            root.join("vars.json").to_str().unwrap(),
            "--output",
            root.join("out.xml").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(
        fs::read_to_string(root.join("out.xml")).unwrap(),
        "<root>\n  <note>record with &lt;tag&gt; &amp; &quot;quotes&quot; &amp; &#x27;apostrophe&#x27; &amp; ampersand&amp;here</note>\n</root>"
    );
}

#[test]
fn render_custom_delimiter_xml_template_escapes_interpolated_special_characters() {
    let root = temp_root("xml-autoescape-custom-delimiters");
    write_file(
        &root.join("custom.xml.j2"),
        "<root>\n  <note><< note >></note>\n</root>\n",
    );

    let output = sc_compose()
        .args([
            "render",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "custom.xml.j2",
            "--variable-delimiters",
            "<<",
            ">>",
            "--var",
            "note=record with <tag> & quotes",
            "--output",
            root.join("out.xml").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(
        fs::read_to_string(root.join("out.xml")).unwrap(),
        "<root>\n  <note>record with &lt;tag&gt; &amp; quotes</note>\n</root>"
    );
}

#[test]
fn render_html_template_escapes_interpolated_special_characters() {
    let root = temp_root("html-autoescape");
    write_file(&root.join("report.html.j2"), "<p>{{ note }}</p>\n");

    let output = sc_compose()
        .args([
            "render",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "report.html.j2",
            "--var",
            "note=record with <tag> & quotes",
            "--output",
            root.join("out.html").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(
        fs::read_to_string(root.join("out.html")).unwrap(),
        "<p>record with &lt;tag&gt; &amp; quotes</p>"
    );
}

#[test]
fn render_non_markup_template_keeps_interpolated_special_characters_raw() {
    let root = temp_root("non-markup-no-autoescape");
    write_file(&root.join("notes.md.j2"), "{{ note }}\n");

    let output = sc_compose()
        .args([
            "render",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "notes.md.j2",
            "--var",
            "note=record with <tag> & quotes",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim_end(),
        "record with <tag> & quotes"
    );
}

#[test]
fn render_sprint_plan_yaml_safe_cli_regression() {
    let root = temp_root("sprint-plan-yaml-safe");
    let template = fs::read_to_string(
        repo_root().join(".claude/skills/codex-orchestration/sprint-plan.md.j2"),
    )
    .unwrap();
    write_file(&root.join("sprint-plan.md.j2"), &template);
    write_file(
        &root.join("vars.json"),
        serde_json::json!({
            "id": "FIX-276",
            "title": "Architecture: plan",
            "status": "planned",
            "branch": "fix/276-yaml-colon-space-unescaped",
            "worktree": "/tmp/sc-compose",
            "target": "develop"
        })
        .to_string()
        .as_str(),
    );

    let output_path = root.join("rendered.md");
    let output = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "sprint-plan.md.j2",
            "--var-file",
            root.join("vars.json").to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rendered = fs::read_to_string(output_path).unwrap();
    let start = rendered
        .find("---\nid:")
        .expect("rendered sprint plan must contain generated frontmatter")
        + 4;
    let body = &rendered[start..];
    let end = body
        .find("\n---\n")
        .expect("generated sprint plan frontmatter must close");
    let frontmatter: serde_yaml::Value = serde_yaml::from_str(&body[..end]).unwrap();
    assert_eq!(frontmatter["title"], "Architecture: plan");
}

#[test]
fn render_uses_json_var_file_inputs() {
    let root = temp_root("var-file-json");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.json");
    write_file(&vars_file, "{ \"name\": \"json-world\" }\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello json-world"
    );
}

#[test]
fn validate_and_render_adjacent_jinja_document_frontmatter() {
    let root = temp_root("adjacent-jinja-document-frontmatter");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - id\ndefaults:\n  worktree: \"\"\n---\n---\nid: {{ id }}\n{% if worktree %}worktree: {{ worktree }}\n{% endif %}target: x\n---\nbody\n",
    );
    let vars_set = root.join("vars-set.json");
    write_file(
        &vars_set,
        "{\"id\":\"item\",\"worktree\":\"../worktree\"}\n",
    );
    let vars_unset = root.join("vars-unset.json");
    write_file(&vars_unset, "{\"id\":\"item\"}\n");

    let validate = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_set)
        .arg("--json")
        .output()
        .unwrap();
    assert!(validate.status.success(), "{validate:?}");
    let validate_json: Value = serde_json::from_slice(&validate.stdout).unwrap();
    let validate_codes = validate_json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect::<Vec<_>>();
    assert!(!validate_codes.contains(&"ERR_CONFIG_PARSE"));

    let rendered_set = root.join("rendered-set.md");
    let render_set = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_set)
        .arg("--output")
        .arg(&rendered_set)
        .output()
        .unwrap();
    assert!(render_set.status.success(), "{render_set:?}");
    let rendered_set_text = fs::read_to_string(&rendered_set).unwrap();
    assert!(rendered_set_text.contains("worktree: ../worktree"));
    assert!(rendered_set_text.contains("id: item"));

    let rendered_unset = root.join("rendered-unset.md");
    let render_unset = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_unset)
        .arg("--output")
        .arg(&rendered_unset)
        .output()
        .unwrap();
    assert!(render_unset.status.success(), "{render_unset:?}");
    let rendered_unset_text = fs::read_to_string(&rendered_unset).unwrap();
    assert!(!rendered_unset_text.contains("worktree:"));
    assert!(rendered_unset_text.contains("id: item"));
}

#[test]
fn render_uses_yaml_var_file_inputs() {
    let root = temp_root("var-file-yaml");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(&vars_file, "name: yaml-world\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello yaml-world"
    );
}

#[test]
fn render_rejects_yaml_merge_key_var_file_with_actionable_location() {
    let root = temp_root("yaml-merge-key-var-file");
    write_file(&root.join("template.md.j2"), "{{ item }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(
        &vars_file,
        "defaults: &defaults\n  base: /tmp\n  name: base\nitem:\n  <<: *defaults\n  name: override\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERR_CONFIG_VARFILE"), "stderr: {stderr}");
    assert!(
        stderr.contains("unsupported YAML merge key `<<` at line 5, column 3"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("expand the mapping explicitly"),
        "stderr: {stderr}"
    );
}

#[test]
fn render_preserves_json_merge_shaped_keys() {
    let root = temp_root("json-merge-shaped-key");
    write_file(&root.join("template.md.j2"), "{{ config[\"<<\"] }}\n");
    let vars_file = root.join("vars.json");
    write_file(&vars_file, r#"{"config":{"<<":"literal"}}"#);

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "literal");
}

#[test]
fn verify_reports_clean_when_render_matches_deployed() {
    let root = temp_root("verify-clean");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    let deployed = root.join("deployed.md");
    write_file(&deployed, "hello world");

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("template.md.j2")
        .arg(&deployed)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("OK"));
}

#[test]
fn verify_reports_drift_with_exit_code_one() {
    let root = temp_root("verify-drift");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    let deployed = root.join("deployed.md");
    write_file(&deployed, "hello drift\n");

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("template.md.j2")
        .arg(&deployed)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DRIFT detected"), "{stderr}");
    assert!(stderr.contains("-hello drift"), "{stderr}");
    assert!(stderr.contains("+hello world"), "{stderr}");
}

#[test]
fn verify_quiet_suppresses_diff_output() {
    let root = temp_root("verify-quiet");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    let deployed = root.join("deployed.md");
    write_file(&deployed, "hello drift\n");

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("template.md.j2")
        .arg("--quiet")
        .arg(&deployed)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DRIFT detected"), "{stderr}");
    assert!(!stderr.contains("@@"), "{stderr}");
}

#[test]
fn verify_builtin_var_override_can_make_output_deterministic() {
    let root = temp_root("verify-builtin-override");
    write_file(&root.join("template.md.j2"), "{{ RENDER_DATE }}\n");
    let deployed = root.join("deployed.md");
    write_file(&deployed, "2026-01-01");

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("template.md.j2")
        .arg("--builtin-var")
        .arg("RENDER_DATE=2026-01-01")
        .arg(&deployed)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("OK"));
}

#[test]
fn verify_requires_against_in_file_mode() {
    let root = temp_root("verify-missing-against");
    let deployed = root.join("deployed.md");
    write_file(&deployed, "hello world");

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg(&deployed)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--against is required in file mode"),
        "{stderr}"
    );
}

#[test]
fn verify_reports_missing_deployed_file() {
    let root = temp_root("verify-missing-deployed");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("template.md.j2")
        .arg(root.join("missing.md"))
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("deployed file not found"), "{stderr}");
}

#[test]
fn verify_reports_missing_template_path() {
    let root = temp_root("verify-missing-template");
    let deployed = root.join("deployed.md");
    write_file(&deployed, "hello world");

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("missing.md.j2")
        .arg(&deployed)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("template path not found"), "{stderr}");
}

#[test]
fn verify_treats_crlf_deployed_file_as_clean() {
    let root = temp_root("verify-crlf-clean");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\nnext line",
    );
    let deployed = root.join("deployed.md");
    fs::write(&deployed, b"hello world\r\nnext line").unwrap();

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("template.md.j2")
        .arg(&deployed)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("OK"));
}

#[test]
fn verify_treats_trailing_newline_difference_as_clean() {
    let root = temp_root("verify-trailing-newline-clean");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    let deployed = root.join("deployed.md");
    write_file(&deployed, "hello world\n");

    let output = sc_compose()
        .arg("verify")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("template.md.j2")
        .arg(&deployed)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("OK"));
}

#[test]
fn render_all_rejects_repeated_equals_syntax_prefix_in_var_argument() {
    let root = temp_root("render-all-repeated-var-prefix");
    write_file(
        &root.join("template.md.j2"),
        "---\npasses:\n  - pass: 1\nrequired_variables:\n  - foo\n---\n{{ foo }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--pass")
        .arg("1")
        .arg("--var=--var=foo=bar")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing required variable: foo"),
        "{stderr}"
    );
}

#[test]
fn render_all_renders_multi_pass_template_with_inline_pass_vars() {
    let root = temp_root("render-all-inline");
    write_file(
        &root.join("template.2.j2"),
        "---\npass: 2\n---\n---\npass: 1\n---\ndeploy {{ task }} for {{{ team }}}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.2.j2")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team=wyvern")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "deploy test for wyvern"
    );
}

#[test]
fn render_all_renders_multi_pass_template_with_per_pass_var_files() {
    let root = temp_root("render-all-var-files");
    write_file(
        &root.join("template.2.j2"),
        "---\npass: 2\n---\n---\npass: 1\n---\ndeploy {{ task }} for {{{ team }}}\n",
    );
    write_file(&root.join("vars-pass2.json"), "{ \"team\": \"wyvern\" }\n");
    write_file(&root.join("vars-pass1.yaml"), "task: test\n");

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.2.j2")
        .arg("--pass")
        .arg("2")
        .arg("--var-file")
        .arg(root.join("vars-pass2.json"))
        .arg("--pass")
        .arg("1")
        .arg("--var-file")
        .arg(root.join("vars-pass1.yaml"))
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "deploy test for wyvern"
    );
}

#[test]
fn validate_all_accepts_multi_pass_template() {
    let root = temp_root("validate-all");
    write_file(
        &root.join("template.2.j2"),
        "---\npass: 2\n---\n---\npass: 1\n---\ndeploy {{ task }} for {{{ team }}}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.2.j2")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("undeclared referenced token"));
}

#[test]
fn validate_strict_accepts_loop_context_builtins_inside_for() {
    let root = temp_root("strict-loop-context");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  items: [one, two]\n---\n{% for item in items %}{{ loop.index }} {{ loop.index0 }} {{ loop.revindex }} {{ loop.revindex0 }} {{ loop.first }} {{ loop.last }} {{ loop.length }} {{ loop.depth }} {{ loop.depth0 }} {{ loop.cycle(\"odd\", \"even\") }}:{{ item }}{% endfor %}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--strict")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_strict_rejects_loop_context_outside_for_and_lookalikes() {
    let root = temp_root("strict-loop-context-boundaries");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  items: [one]\n---\noutside={{ loop.last }}\n{% for item in items %}inside={{ loop.anything }}{% endfor %}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--strict")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("undeclared referenced token: loop.last"));
    assert!(stdout.contains("undeclared referenced token: loop.anything"));
}

#[test]
fn render_brace_count_uses_custom_triple_brace_delimiters() {
    let root = temp_root("brace-count");
    write_file(&root.join("template.md.j2"), "hello {{{ name }}}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--brace-count")
        .arg("3")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=world")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
}

#[test]
fn render_variable_delimiters_uses_explicit_delimiter_pair() {
    let root = temp_root("variable-delimiters");
    write_file(&root.join("template.md.j2"), "hello << name >>\n");

    let output = sc_compose()
        .arg("render")
        .arg("--variable-delimiters")
        .arg("<<")
        .arg(">>")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=world")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
}

#[test]
fn render_variable_delimiters_reports_invalid_delimiters_without_panicking() {
    let root = temp_root("variable-delimiters-invalid");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--variable-delimiters")
        .arg("")
        .arg(">>")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=world")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PARSE"));
    assert_eq!(stderr.matches("invalid custom delimiters").count(), 1);
    assert!(!stderr.contains("panicked at"));
    assert!(!stderr.contains("stack backtrace"));
}

#[test]
fn render_all_warns_and_falls_back_for_single_pass_template() {
    let root = temp_root("render-all-single-pass");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("name=world")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).trim().is_empty(),
        "{output:?}"
    );
}

#[test]
fn render_all_accepts_equals_syntax_for_pass_groups() {
    let root = temp_root("render-all-pass-equals");
    write_file(
        &root.join("template.2.j2"),
        "---\npass: 2\n---\n---\npass: 1\n---\ndeploy {{ task }} for {{{ team }}}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.2.j2")
        .arg("--pass=2")
        .arg("--var=team=wyvern")
        .arg("--pass=1")
        .arg("--var=task=test")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "deploy test for wyvern"
    );
}

#[test]
fn render_all_rejects_wrong_pass_order() {
    let root = temp_root("render-all-wrong-order");
    write_file(
        &root.join("template.2.j2"),
        "---\npass: 2\n---\n---\npass: 1\n---\ndeploy {{ task }} for {{{ team }}}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.2.j2")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team=wyvern")
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        !String::from_utf8_lossy(&output.stderr).trim().is_empty(),
        "{output:?}"
    );
}

#[test]
fn render_all_reports_missing_pass_variables() {
    let root = temp_root("render-all-missing-pass-vars");
    write_file(
        &root.join("template.2.j2"),
        "---\npass: 2\nrequired_variables:\n  - team\n---\n---\npass: 1\nrequired_variables:\n  - task\n---\ndeploy {{ task }} for {{{ team }}}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.2.j2")
        .arg("--pass")
        .arg("2")
        .arg("--pass")
        .arg("1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing required variable"),
        "{output:?}"
    );
}

#[test]
fn render_all_normalizes_pass_zero_to_default_pass_number() {
    let root = temp_root("render-all-pass-zero");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\ndefaults:\n  name: fallback\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--pass")
        .arg("0")
        .arg("--var")
        .arg("name=world")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
}

#[test]
fn render_brace_count_runs_validation_before_custom_rendering() {
    let root = temp_root("brace-count-validation");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{{ name }}}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--brace-count")
        .arg("3")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing required variable"),
        "{output:?}"
    );
}

#[test]
fn render_brace_count_rejects_values_below_two() {
    let root = temp_root("brace-count-invalid");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--brace-count")
        .arg("1")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(
        !String::from_utf8_lossy(&output.stderr).trim().is_empty(),
        "{output:?}"
    );
}

#[test]
fn render_treats_required_variable_as_satisfied_by_input_defaults_alias() {
    let root = temp_root("required-input-defaults");
    let vars_file = root.join("vars.json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\ninput_defaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    write_file(&vars_file, "{}");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
}

#[test]
fn validate_still_errors_for_variables_not_in_required_or_input_defaults() {
    let root = temp_root("unknown-variable-with-input-defaults");
    let vars_file = root.join("vars.json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - task_id\ninput_defaults:\n  assignee: teammate\n---\nhello {{ task_id }} {{ assignee }}\n",
    );
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-1", "unexpected": "value" }"#,
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--unknown-var-mode")
        .arg("error")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ERR_VAL_EXTRA_INPUT"));
}

#[test]
fn render_unknown_var_mode_errors_on_referenced_unbound_variable() {
    let root = temp_root("unbound-variable-policy");
    write_file(
        &root.join("template.md.j2"),
        "bound={{ bound }} missing={{ missing }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("bound=present")
        .arg("--unknown-var-mode")
        .arg("error")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("ERR_VAL_UNBOUND_VARIABLE"), "{combined}");
    assert!(combined.contains("missing"), "{combined}");
    assert!(!combined.contains("unbound variable: bound"), "{combined}");
}

#[test]
fn unknown_var_mode_help_describes_both_policy_axes() {
    let output = sc_compose().args(["render", "--help"]).output().unwrap();

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("extra caller-provided"), "{help}");
    assert!(help.contains("referenced-but-unbound"), "{help}");
}

#[test]
fn validate_warns_when_defaults_and_input_defaults_both_exist() {
    let root = temp_root("input-defaults-alias-warning");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: old\ninput_defaults:\n  name: new\nrequired_variables:\n  - task_id\n---\nhello {{ task_id }} {{ name }}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("task_id=SC-1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("WARN_VAL_CONFLICTING_DEFAULT_SECTIONS"));
    assert!(stdout.contains("input_defaults"));
    assert!(stdout.contains("defaults"));
}

#[test]
fn render_accepts_array_of_objects_in_json_var_file() {
    let root = temp_root("array-objects-json");
    write_file(
        &root.join("template.md.j2"),
        "{% for sprint in sprints %}{{ sprint.id }}:{{ sprint.stage }}\n{% endfor %}",
    );
    let vars_file = root.join("vars.json");
    write_file(
        &vars_file,
        r#"{ "sprints": [ { "id": "S1", "stage": "qa" }, { "id": "S2", "stage": "merged" } ] }"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("S1:qa"));
    assert!(stdout.contains("S2:merged"));
}

#[test]
fn render_accepts_array_of_objects_in_yaml_var_file() {
    let root = temp_root("array-objects-yaml");
    write_file(
        &root.join("template.md.j2"),
        "{% for sprint in sprints %}{{ sprint.id }}:{{ sprint.stage }}\n{% endfor %}",
    );
    let vars_file = root.join("vars.yaml");
    write_file(
        &vars_file,
        "sprints:\n  - id: S1\n    stage: qa\n  - id: S2\n    stage: merged\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("S1:qa"));
    assert!(stdout.contains("S2:merged"));
}

#[test]
fn render_accepts_issue_157_nested_categories_fixture() {
    let vars_file = repo_root()
        .join("examples")
        .join("changelog-categories.sample-vars.json");
    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg("examples/changelog-categories.md.j2")
        .arg("--var-file")
        .arg(vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("### Added"));
    assert!(
        stdout.contains("Deconstruct methods and matching constructors on SerialNumber. (#588)")
    );
    assert!(stdout.contains("### Changed"));
    assert!(stdout.contains("Bumped from 0.54.0 to 0.55.0 (MINOR)."));
}

#[test]
fn render_accepts_issue_157_jagged_array_fixture() {
    let vars_file = repo_root()
        .join("examples")
        .join("jagged-array-values.sample-vars.json");
    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg("examples/jagged-array-values.md.j2")
        .arg("--var-file")
        .arg(vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let normalized_stdout = stdout.replace("\r\n", "\n");
    assert_eq!(normalized_stdout.trim(), "1, 2, 3\n4, 5");
}

#[test]
fn render_accepts_recursive_values_in_yaml_var_file() {
    let root = temp_root("recursive-yaml-var-file");
    write_file(
        &root.join("template.md.j2"),
        "{% for category in categories %}{% for item in category.items %}{{ category.name }}:{{ item.summary }}\n{% endfor %}{% endfor %}{% for row in rows %}{{ row | join(',') }}\n{% endfor %}",
    );
    let vars_file = root.join("vars.yaml");
    write_file(
        &vars_file,
        "categories:\n  - name: Added\n    items:\n      - summary: nested YAML item\nrows:\n  - [1, 2, 3]\n  - [4, 5]\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Added:nested YAML item"));
    assert!(stdout.contains("1,2,3"));
    assert!(stdout.contains("4,5"));
}

#[test]
fn f4_cli_regression_accepts_deeply_nested_json_values() {
    let root = temp_root("f4-cli-deep-json-values");
    write_file(
        &root.join("template.md.j2"),
        "{% for group in groups %}{{ group.name }}:{% for item in group.items %}{{ item.id }}={{ item.tags | join(',') }}:{% for value in item.matrix %}{{ value }}{% endfor %};{% endfor %}\n{% endfor %}",
    );
    let vars_file = root.join("vars.json");
    write_file(
        &vars_file,
        r#"{"groups":[{"name":"api","items":[{"id":"one","tags":["read","write"],"matrix":[1,2]},{"id":"two","tags":["admin"],"matrix":[3,4]}]}]}"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "api:one=read,write:12;two=admin:34;\n"
    );
}

#[test]
fn f4_cli_regression_rejects_nested_duplicate_json_and_yaml_keys() {
    let root = temp_root("f4-cli-nested-duplicates");
    write_file(&root.join("template.md.j2"), "{{ config }}\n");
    for (filename, contents) in [
        (
            "nested.json",
            r#"{"config":{"name":"first","name":"second"}}"#,
        ),
        ("nested.yaml", "config:\n  name: first\n  name: second\n"),
    ] {
        let vars_file = root.join(filename);
        write_file(&vars_file, contents);
        let output = sc_compose()
            .arg("render")
            .arg("--mode")
            .arg("file")
            .arg("--root")
            .arg(&root)
            .arg("--file")
            .arg("template.md.j2")
            .arg("--var-file")
            .arg(&vars_file)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(3), "{filename}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("ERR_CONFIG_PARSE"),
            "{filename}: {:?}",
            output.stderr
        );
    }
}

#[test]
fn f4_cli_regression_rejects_non_string_yaml_key_inside_array_object() {
    let root = temp_root("f4-cli-array-nested-key");
    write_file(&root.join("template.md.j2"), "{{ items }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(&vars_file, "items:\n  - metadata:\n      7: invalid\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_VAL_OBJECT_SHAPE"));
}

#[test]
fn f4_cli_regression_rejects_top_level_non_object_var_file() {
    let root = temp_root("f4-cli-top-level-sequence");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.json");
    write_file(&vars_file, "[\"not\", \"an object\"]\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ERR_CONFIG_VARFILE"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn f4_cli_regression_rejects_malformed_var_file() {
    let root = temp_root("f4-cli-malformed-var-file");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.json");
    write_file(&vars_file, "{ \"name\": \"unterminated\"\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ERR_CONFIG_PARSE"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn render_strips_utf8_bom_before_frontmatter() {
    let root = temp_root("frontmatter-utf8-bom");
    fs::write(
        root.join("template.md.j2"),
        b"\xef\xbb\xbf---\nrequired_variables:\n  - name\n---\nHello {{name}}\n",
    )
    .unwrap();

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=x")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello x\n");
}

#[test]
fn render_reports_invalid_template_utf8_as_config_read() {
    let root = temp_root("template-invalid-utf8");
    fs::write(
        root.join("template.md.j2"),
        b"---\n---\nbad \xff\xfe bytes\n",
    )
    .unwrap();

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("ERR_CONFIG_READ"), "stderr: {stderr}");
    assert!(
        !stderr.contains("ERR_INCLUDE_NOT_FOUND"),
        "stderr: {stderr}"
    );
}

#[test]
fn render_reports_declared_required_variable_as_missing() {
    let root = temp_root("declared-required-variable");
    write_file(
        &root.join("template.md.j2"),
        "---\nvariables:\n  needed:\n    required: true\n---\nX {{needed}}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr.contains("ERR_VAL_MISSING_REQUIRED"),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("ERR_VAL_UNDECLARED_TOKEN"),
        "stderr: {stderr}"
    );
}

#[test]
fn render_rejects_duplicate_json_and_yaml_var_file_keys() {
    let root = temp_root("duplicate-var-file-keys");
    write_file(&root.join("template.md.j2"), "{{ a }}\n");
    let inputs = [
        ("vars.json", r#"{"a": 1, "a": 2}"#),
        ("vars.yaml", "a: 1\na: 2\n"),
    ];

    for (filename, contents) in inputs {
        let vars_file = root.join(filename);
        write_file(&vars_file, contents);
        let output = sc_compose()
            .arg("render")
            .arg("--mode")
            .arg("file")
            .arg("--root")
            .arg(&root)
            .arg("--file")
            .arg("template.md.j2")
            .arg("--var-file")
            .arg(&vars_file)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert_eq!(output.status.code(), Some(3), "{filename}: {stderr}");
        assert!(stderr.contains("ERR_CONFIG_PARSE"), "{filename}: {stderr}");
    }
}

#[test]
fn render_rejects_non_string_nested_yaml_map_keys() {
    let root = temp_root("recursive-yaml-invalid-key");
    write_file(&root.join("template.md.j2"), "{{ value }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(&vars_file, "value:\n  1: invalid-key\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_VAL_OBJECT_SHAPE"));
}

#[test]
fn render_rejects_top_level_yaml_sequence_var_file() {
    let root = temp_root("recursive-yaml-top-level-sequence");
    write_file(&root.join("template.md.j2"), "{{ value }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(&vars_file, "- one\n- two\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_CONFIG_VARFILE"));
}

#[test]
fn render_accepts_recursive_values_in_frontmatter_defaults() {
    let root = temp_root("recursive-frontmatter-defaults");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - categories\ninput_defaults:\n  categories:\n    - name: Added\n      items:\n        - summary: frontmatter item\n---\n{% for category in categories %}{% for item in category.items %}{{ category.name }}:{{ item.summary }}\n{% endfor %}{% endfor %}",
    );
    write_file(&root.join("vars.json"), "{}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(root.join("vars.json"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Added:frontmatter item"));
}

#[test]
fn render_xml_control_byte_output_is_well_formed() {
    let root = temp_root("xml-control-byte");
    write_file(
        &root.join("report.xml.j2"),
        "<root><title>{{ value }}</title></root>\n",
    );
    write_file(&root.join("vars.json"), "{\"value\": \"\\u0000\"}\n");
    let output_path = root.join("rendered.xml");
    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("report.xml.j2")
        .arg("--var-file")
        .arg(root.join("vars.json"))
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rendered = fs::read_to_string(&output_path).unwrap();
    assert!(rendered.contains("&#xfffd;"), "rendered XML: {rendered}");

    let python_code = "import sys, xml.etree.ElementTree as ET; ET.fromstring(sys.argv[1])";
    let python = if Command::new("python3")
        .arg("--version")
        .output()
        .map(|probe| probe.status.success())
        .unwrap_or(false)
    {
        "python3"
    } else {
        "python"
    };
    let parsed = Command::new(python)
        .arg("-c")
        .arg(python_code)
        .arg(&rendered)
        .output()
        .unwrap();
    assert!(
        parsed.status.success(),
        "Python XML parse failed: {}\nXML: {rendered}",
        String::from_utf8_lossy(&parsed.stderr)
    );
}

#[test]
fn render_markdown_table_safe_cli_regression() {
    let root = temp_root("markdown-table-safe");
    write_file(
        &root.join("table.md.j2"),
        "| Value |\n| --- |\n| {{ value | md_table_safe }} |\n",
    );
    write_file(
        &root.join("vars.json"),
        r#"{"value":"cache|hit\nnext"}
"#,
    );
    let output_path = root.join("rendered.md");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("table.md.j2")
        .arg("--var-file")
        .arg(root.join("vars.json"))
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rendered = fs::read_to_string(&output_path).unwrap();
    assert!(
        rendered.contains(r"cache\|hit next"),
        "rendered: {rendered}"
    );
}

#[test]
fn render_sprint_plan_cli_neutralizes_injected_frontmatter_delimiters() {
    let root = temp_root("sprint-plan-frontmatter-injection");
    let template_path = root
        .join(".claude")
        .join("skills")
        .join("codex-orchestration")
        .join("sprint-plan.md.j2");
    write_file(
        &template_path,
        include_str!("../../../../.claude/skills/codex-orchestration/sprint-plan.md.j2"),
    );
    let vars_path = root.join("repro-vars.json");
    write_file(
        &vars_path,
        r#"{
  "id": "1.2",
  "title": "Injected frontmatter break\n---\nmalicious: true\n---",
  "branch": "main",
  "target": "develop"
}
"#,
    );
    let output_path = root.join("rendered.md");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg(".claude/skills/codex-orchestration/sprint-plan.md.j2")
        .arg("--var-file")
        .arg(&vars_path)
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rendered = fs::read_to_string(&output_path).unwrap();
    assert_eq!(rendered.lines().filter(|line| *line == "---").count(), 2);
    assert!(rendered.contains("malicious: true"));
    assert!(rendered.contains(r"\-\-\-"));
}

#[test]
fn render_supports_multi_level_nested_includes() {
    let root = temp_root("nested-include-proof");
    write_file(
        &root.join("partials").join("footer.md.j2"),
        "Footer for {{ owner }}\n",
    );
    write_file(
        &root.join("partials").join("middle.md.j2"),
        "Middle start\n@<footer.md.j2>\nMiddle end\n",
    );
    write_file(
        &root.join("nested-root.md.j2"),
        "---\nrequired_variables:\n  - owner\n---\nTop start\n@<partials/middle.md.j2>\nTop end\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("nested-root.md.j2")
        .arg("--var")
        .arg("owner=Phase B")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let rendered = String::from_utf8(output.stdout).unwrap();
    assert!(rendered.contains("Top start"));
    assert!(rendered.contains("Middle start"));
    assert!(rendered.contains("Footer for Phase B"));
    assert!(rendered.contains("Middle end"));
    assert!(rendered.contains("Top end"));
}

#[test]
fn render_accepts_object_values_in_json_var_file() {
    let root = temp_root("object-json-var-file");
    let vars_file = root.join("vars.json");
    write_file(
        &root.join("template.md.j2"),
        "PR #{{ pr.number }} -> {{ pr.url }}\n",
    );
    write_file(
        &vars_file,
        r#"{ "pr": { "number": 43, "url": "https://example.test/pr/43" } }"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "PR #43 -> https://example.test/pr/43"
    );
}

#[test]
fn render_accepts_object_values_in_yaml_var_file() {
    let root = temp_root("object-yaml-var-file");
    let vars_file = root.join("vars.yaml");
    write_file(
        &root.join("template.md.j2"),
        "PR #{{ pr.number }} -> {{ pr.url }}\n",
    );
    write_file(
        &vars_file,
        "pr:\n  number: 43\n  url: https://example.test/pr/43\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "PR #43 -> https://example.test/pr/43"
    );
}

#[test]
fn render_uses_env_prefix_inputs() {
    let root = temp_root("env-prefix");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--env-prefix")
        .arg("SC_")
        .env("SC_NAME", "env-world")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello env-world"
    );
}

#[test]
fn render_injects_builtin_variables_and_respects_override_precedence() {
    let root = temp_root("render-builtins");
    write_file(
        &root.join("report.md.j2"),
        "---\ndefaults:\n  HOSTNAME: default-host\n---\n{{ TEMPLATE_NAME }}|{{ HOSTNAME }}|{{ USERNAME }}|{{ RENDER_DATE }}|{{ RENDER_TIMESTAMP }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("report.md.j2")
        .arg("--env-prefix")
        .arg("SC_")
        .arg("--var")
        .arg("USERNAME=cli-user")
        .env("SC_HOSTNAME", "env-host")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parts = stdout.trim().split('|').collect::<Vec<_>>();
    assert_eq!(parts[0], "report.md.j2");
    assert_eq!(parts[1], "env-host");
    assert_eq!(parts[2], "cli-user");
    assert_eq!(parts[3].len(), 10);
    assert!(parts[4].contains('T'));
}

#[test]
fn render_dry_run_text_reports_would_change() {
    let root = temp_root("render-dry-run-text");
    let output_path = root.join("out.md");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    write_file(&output_path, "hello world");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&output_path)
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("would_change: false"));
}

#[test]
fn render_reports_include_escape_for_path_confinement_violations() {
    let namespace = temp_root("render-include-escape-cli");
    let root = namespace.join("repo");
    let outside = namespace.join("outside-include.md");
    write_file(&outside, "outside\n");
    write_file(&root.join("template.md.j2"), "@<../outside-include.md>\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_INCLUDE_ESCAPE"));
}

#[test]
fn render_reports_include_escape_for_symlink_escape_at_cli_layer() {
    let namespace = temp_root("render-symlink-escape-cli");
    let root = namespace.join("repo");
    let outside = namespace.join("outside-symlink-include.md");
    write_file(&outside, "outside\n");
    let symlink_path = root.join("linked-outside.md");
    if !create_symlink_if_supported(&outside, &symlink_path) {
        return;
    }
    write_file(&root.join("template.md.j2"), "@<linked-outside.md>\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_INCLUDE_ESCAPE"));
}

#[cfg(windows)]
#[test]
fn windows_backslash_escape_requires_cli_confinement_coverage() {
    let namespace = temp_root("render-backslash-escape-cli");
    let root = namespace.join("repo");
    let outside = namespace.join("outside-backslash-include.md");
    write_file(&outside, "outside\n");
    write_file(
        &root.join("template.md.j2"),
        "@<..\\outside-backslash-include.md>\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_INCLUDE_ESCAPE"));
}

#[test]
fn render_smoke_pipeline_handles_includes_vars_var_file_env_and_output() {
    let root = temp_root("render-smoke");
    let output = root.join("out.md");
    let vars_file = root.join("vars.yaml");
    write_file(
        &root.join("template.md.j2"),
        concat!(
            "---\nrequired_variables:\n  - name\n  - title\n  - mood\n---\n",
            "@<partials/body.md>\n"
        ),
    );
    write_file(
        &root.join("partials").join("body.md"),
        "Name: {{ name }}\nTitle: {{ title }}\nMood: {{ mood }}\n",
    );
    write_file(&vars_file, "title: Engineer\n");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=Casey")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--env-prefix")
        .arg("SC_")
        .arg("--output")
        .arg(&output)
        .env("SC_MOOD", "focused")
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "Name: Casey\nTitle: Engineer\nMood: focused"
    );
}
