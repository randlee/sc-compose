mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use support::{
    normalize_path_str, parse_stdout, repo_root, valid_report_catalog, write_file,
    write_render_many_fixture, write_report_catalog, write_report_family_override,
    write_smoke_fixture, write_state_machine_spec,
};

fn temp_root(label: &str) -> PathBuf {
    support::temp_root(label, "sc-compose-cli")
}

fn sc_compose() -> std::process::Command {
    support::sc_compose("sc-compose-cli")
}

fn write_sql_query_spec(root: &Path, relative: &str) {
    write_file(
        &root.join(relative),
        r#"[spec]
kind = "sql_query"
id = "sql-diagrams"
title = "SQL Diagrams"
renderer_targets = ["mermaid"]

[sql_query]
purpose = "Summarize shipped orders"
tables_read = ["orders", "customers"]
tables_written = ["report_cache"]
filters = ["status = shipped"]
ordering = ["created_at DESC"]
cardinality = "many"
transactional_assumptions = ["read committed"]

[metadata]
sets = ["publish", "diagram"]
"#,
    );
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(&path, &target).unwrap();
        }
    }
}

fn stage_phase_b_reference_assets(root: &Path) {
    copy_dir_all(&repo_root().join("examples"), &root.join("examples"));
    copy_dir_all(&repo_root().join("reports"), &root.join("reports"));
}

fn render_report_summary(root: &Path, vars_path: &str, output_path: &str) {
    if let Some(parent) = root.join(output_path).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(root)
        .arg("--file")
        .arg("examples/report-evidence-summary.html.j2")
        .arg("--var-file")
        .arg(root.join(vars_path))
        .arg("--output")
        .arg(root.join(output_path))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

fn write_large_report_evidence_vars(path: &Path) {
    let items = (0..160)
        .map(|idx| {
            serde_json::json!({
                "section_key": if idx % 2 == 0 { "evidence" } else { "diagrams" },
                "label": format!("Generated item {idx}"),
                "href": format!("reports/latest/generated/{idx}.html"),
                "status": if idx % 3 == 0 { serde_json::Value::String("latest".to_owned()) } else { serde_json::Value::Null },
                "note": format!("Generated note {}", "x".repeat(24)),
            })
        })
        .collect::<Vec<_>>();
    let payload = serde_json::json!({
        "report": {
            "title": "Large Report Evidence Summary",
            "family": "Phase B proof vehicle",
            "generated_at": "2026-05-27T03:40:00Z"
        },
        "summary": {
            "status": "PASS",
            "note": "Large payload compatibility check."
        },
        "sections": [
            { "key": "evidence", "title": "Evidence" },
            { "key": "diagrams", "title": "Diagrams" }
        ],
        "items": items
    });
    write_file(path, &serde_json::to_string_pretty(&payload).unwrap());
}

fn finalize_report(root: &Path, report_id: &str, kind: &str, entrypoint: &str, artifacts: &[&str]) {
    let mut command = sc_compose();
    command
        .arg("reports")
        .arg("finalize")
        .arg("--root")
        .arg(root)
        .arg("--report-id")
        .arg(report_id)
        .arg("--kind")
        .arg(kind)
        .arg("--entrypoint")
        .arg(entrypoint)
        .arg("--archive");
    for artifact in artifacts {
        command.arg("--artifact").arg(artifact);
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{output:?}");
}

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
fn template_init_builds_multi_pass_template_from_pass_groups() {
    let root = temp_root("template-init-multi-pass");
    let file = root.join("agent.md");
    write_file(&file, "deploy test for wyvern");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pass: 2"), "{stdout}");
    assert!(stdout.contains("pass: 1"), "{stdout}");
    assert!(stdout.contains("{{{ team }}}"), "{stdout}");
    assert!(stdout.contains("{{ task }}"), "{stdout}");
}

#[test]
fn template_init_single_pass_omits_pass_one_marker() {
    let root = temp_root("template-init-single-pass");
    let file = root.join("agent.md");
    write_file(&file, "deploy test");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "---\nrequired_variables:\n  - task\ndefaults: {}\nmetadata: {}\n---\ndeploy {{ task }}\n"
    );
}

#[test]
fn template_init_round_trip_verifies_clean() {
    let root = temp_root("template-init-round-trip");
    let file = root.join("agent.md");
    let deployed = root.join("deployed.md");
    write_file(&file, "deploy test for wyvern");
    write_file(&deployed, "deploy test for wyvern");

    let init = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--force")
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
    assert!(init.status.success(), "{init:?}");

    let verify = sc_compose()
        .arg("verify")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("agent.md")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team=wyvern")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .arg(&deployed)
        .output()
        .unwrap();

    assert!(verify.status.success(), "{verify:?}");
    assert!(String::from_utf8_lossy(&verify.stdout).contains("OK"));
}

#[test]
fn template_init_reports_values_not_found() {
    let root = temp_root("template-init-value-not-found");
    let file = root.join("agent.md");
    write_file(&file, "deploy test");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("team=wyvern")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("values not found in file"), "{stderr}");
}

#[test]
fn template_init_rejects_duplicate_literal_assignments() {
    let root = temp_root("template-init-duplicate-literal");
    let file = root.join("agent.md");
    write_file(&file, "alpha alpha");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("first=alpha")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("second=alpha")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("values could not be substituted without overlap"),
        "{stderr}"
    );
}

#[test]
fn template_init_avoids_rewriting_inside_inserted_tokens() {
    let root = temp_root("template-init-substring-overlap");
    let file = root.join("agent.md");
    write_file(&file, "team me");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team_name=team")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("suffix=me")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("{{{ team_name }}} {{ suffix }}"),
        "{stdout}"
    );
    assert!(!stdout.contains("{{{ tea{{ suffix }}_name }}}"), "{stdout}");
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
    assert!(stderr.contains("invalid custom delimiters: invalid custom delimiters"));
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

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        !String::from_utf8_lossy(&output.stderr).trim().is_empty(),
        "{output:?}"
    );
}

#[test]
fn examples_list_uses_data_dir_override() {
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello"));
    assert!(stdout.contains("pytest-fixture"));
}

#[test]
fn examples_list_with_nonexistent_data_dir_exits_zero_and_prints_nothing() {
    let root = temp_root("examples-list-missing-data-dir");
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .env("SC_COMPOSE_DATA_DIR", root.join("missing-data-root"))
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "");
}

#[test]
fn examples_named_render_uses_data_dir_override() {
    let output = sc_compose()
        .arg("examples")
        .arg("hello")
        .arg("--var")
        .arg("name=Casey")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Hello Casey!"
    );
}

#[test]
fn examples_named_render_accepts_array_values_from_var_file() {
    let root = temp_root("examples-array");
    let vars_file = root.join("vars.json");
    write_file(
        &vars_file,
        r#"{ "module_name": "auth", "fixture_name": "fixture_state", "test_names": ["login", "logout"] }"#,
    );

    let output = sc_compose()
        .arg("examples")
        .arg("pytest-fixture")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("def test_login(fixture_state):"));
    assert!(stdout.contains("def test_logout(fixture_state):"));
}

fn sprint_report_html_sample_vars() -> PathBuf {
    repo_root()
        .join("examples")
        .join("sprint-report-html.sample-vars.json")
}

#[test]
fn examples_named_render_sprint_report_html_renders_browser_viewable_html() {
    let vars_file = sprint_report_html_sample_vars();

    let output = sc_compose()
        .arg("examples")
        .arg("sprint-report-html")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("<title>HTML Sprint Report</title>"));
    assert!(stdout.contains("https://github.com/randlee/sc-compose/pull/47"));
    assert!(stdout.contains("https://github.com/randlee/sc-compose/actions/runs/118"));
    assert!(stdout.contains("Plan Doc"));
    assert!(stdout.contains("Findings Doc"));
    assert!(stdout.contains("Structured object inputs"));
    assert!(stdout.contains("Arrays of objects"));
}

#[test]
fn examples_named_render_dry_run_derives_html_output_path_from_example_name() {
    let vars_file = sprint_report_html_sample_vars();

    let output = sc_compose()
        .arg("examples")
        .arg("sprint-report-html")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--dry-run")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("would_write: sprint-report-html.html"));
    assert!(stdout.contains("<!DOCTYPE html>"));
}

#[test]
fn examples_list_includes_report_evidence_summary() {
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("report-evidence-summary"));
}

#[test]
fn examples_named_render_report_evidence_summary_renders_browser_viewable_html() {
    let vars_file = repo_root()
        .join("examples")
        .join("report-evidence-summary.sample-vars.json");

    let output = sc_compose()
        .arg("examples")
        .arg("report-evidence-summary")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("Report Evidence Summary"));
    assert!(stdout.contains("sc-lint style evidence family"));
    assert!(stdout.contains("reports/latest/publish-manifest.json"));
}

#[test]
fn general_task_template_validate_accepts_optional_input_defaults_without_explicit_values() {
    let vars_file = temp_root("general-task-validate").join("vars.json");
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-GENERAL-TASK-REVIEW-001", "description": "review", "deliverables": "pass review", "acceptance_criteria": "passes", "references": "template + dev-template" }"#,
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn general_task_template_render_uses_optional_input_defaults_when_absent() {
    let vars_file = temp_root("general-task-defaults").join("vars.json");
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-GENERAL-TASK-REVIEW-001", "description": "review", "deliverables": "pass review", "acceptance_criteria": "passes", "references": "template + dev-template" }"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
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
    assert!(stdout.contains(r#"assignee="teammate""#));
    assert!(!stdout.contains("<worktree>"));
    assert!(!stdout.contains("<branch>"));
    assert!(!stdout.contains("<pr-target>"));
}

#[test]
fn general_task_template_render_dry_run_reports_default_usage_info() {
    let vars_file = temp_root("general-task-dry-run-defaults").join("vars.json");
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-GENERAL-TASK-REVIEW-001", "description": "review", "deliverables": "pass review", "acceptance_criteria": "passes", "references": "template + dev-template" }"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"variable assignee not provided, using default: "teammate""#));
}

#[test]
fn general_task_template_render_allows_overriding_optional_input_defaults() {
    let vars_file = temp_root("general-task-override").join("vars.json");
    let worktree_path = std::env::temp_dir().join(format!(
        "wt-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    write_file(
        &vars_file,
        &serde_json::json!({
            "task_id": "SC-GENERAL-TASK-REVIEW-001",
            "assignee": "architect",
            "description": "review",
            "worktree_path": normalize_path_str(&worktree_path),
            "branch": "feat/x",
            "pr_target": "develop",
            "deliverables": "pass review",
            "acceptance_criteria": "passes",
            "references": "template + dev-template"
        })
        .to_string(),
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
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
    assert!(stdout.contains(r#"assignee="architect""#));
    assert!(stdout.contains(&format!(
        "<worktree>{}</worktree>",
        normalize_path_str(&worktree_path)
    )));
    assert!(stdout.contains("<branch>feat/x</branch>"));
    assert!(stdout.contains("<pr-target>develop</pr-target>"));
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
fn templates_named_render_dry_run_reports_template_json_default_usage() {
    let root = temp_root("template-json-default-usage");
    let templates_root = root.join("templates");
    let pack_root = templates_root.join("report");
    write_file(
        &pack_root.join("template.json"),
        r#"{ "description": "Report template", "version": "1.0.0", "input_defaults": { "name": "world" } }"#,
    );
    write_file(&pack_root.join("report.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("templates")
        .arg("report")
        .arg("--dry-run")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"variable name not provided, using default: "world""#));
}

#[test]
fn examples_named_render_missing_pack_reports_list_recovery_hint() {
    let output = sc_compose()
        .arg("examples")
        .arg("missing-pack")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_FOUND"));
    assert!(stderr.contains("sc-compose examples list"));
}

#[test]
fn examples_named_render_reports_not_renderable_when_example_names_collide() {
    let root = temp_root("examples-collision");
    let examples_root = root.join("examples");
    write_file(&examples_root.join("hello.md.j2"), "hello");
    write_file(&examples_root.join("hello.yaml.j2"), "hello");

    let output = sc_compose()
        .arg("examples")
        .arg("hello")
        .env("SC_COMPOSE_DATA_DIR", &root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_RENDERABLE"));
    assert!(stderr.contains("ambiguous"));
}

#[test]
fn templates_named_render_uses_array_input_defaults_from_template_json() {
    let root = temp_root("templates-array-defaults");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("pytest-defaults");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Pytest defaults", "version": "1.0.0", "input_defaults": { "fixture_name": "fixture_state", "test_names": ["login", "logout"] } }"#,
    );
    write_file(
        &pack.join("pytest-tests.py.j2"),
        "{% for test_name in test_names %}def test_{{ test_name }}({{ fixture_name }}):\n    pytest.fail(\"Fail: Not implemented\")\n\n{% endfor %}",
    );

    let output = sc_compose()
        .arg("templates")
        .arg("pytest-defaults")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("def test_login(fixture_state):"));
    assert!(stdout.contains("def test_logout(fixture_state):"));
}

#[test]
fn template_json_object_input_defaults_obey_precedence() {
    let root = temp_root("templates-object-default-precedence");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("report");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Report defaults", "version": "1.0.0", "input_defaults": { "pr": { "number": 43, "url": "https://example.test/pr/43" } } }"#,
    );
    write_file(
        &pack.join("report.md.j2"),
        "---\ndefaults:\n  pr:\n    number: 7\n    url: https://frontmatter.test/pr/7\n---\nPR #{{ pr.number }} -> {{ pr.url }}\n",
    );

    let default_output = sc_compose()
        .arg("templates")
        .arg("report")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(default_output.status.success());
    assert_eq!(
        String::from_utf8(default_output.stdout).unwrap().trim(),
        "PR #43 -> https://example.test/pr/43"
    );

    let vars_file = root.join("vars.json");
    write_file(
        &vars_file,
        r#"{ "pr": { "number": 99, "url": "https://input.test/pr/99" } }"#,
    );
    let explicit_output = sc_compose()
        .arg("templates")
        .arg("report")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(explicit_output.status.success());
    assert_eq!(
        String::from_utf8(explicit_output.stdout).unwrap().trim(),
        "PR #99 -> https://input.test/pr/99"
    );
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
fn f4_cli_regression_accepts_nested_arrays_and_objects() {
    let root = temp_root("f4-cli-nested-values");
    write_file(
        &root.join("template.md.j2"),
        "{% for group in groups %}{{ group.name }}:{% for item in group.items %}{{ item.id }}={{ item.tags | join(',') }};{% endfor %}\n{% endfor %}",
    );
    let vars_file = root.join("vars.yaml");
    write_file(
        &vars_file,
        "groups:\n  - name: api\n    items:\n      - id: one\n        tags: [read, write]\n      - id: two\n        tags: [admin]\n",
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
        "api:one=read,write;two=admin;\n"
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
fn f4_cli_regression_rejects_non_string_nested_yaml_map_keys() {
    let root = temp_root("f4-cli-nested-key");
    write_file(&root.join("template.md.j2"), "{{ config }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(&vars_file, "config:\n  7: invalid\n");

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
fn templates_named_render_accepts_recursive_values_in_template_json_defaults() {
    let root = temp_root("templates-recursive-defaults");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("nested-report");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Nested report", "version": "1.0.0", "input_defaults": { "categories": [ { "name": "Added", "items": [ { "summary": "template manifest item" } ] } ] } }"#,
    );
    write_file(
        &pack.join("report.md.j2"),
        "{% for category in categories %}{% for item in category.items %}{{ category.name }}:{{ item.summary }}\n{% endfor %}{% endfor %}",
    );

    let output = sc_compose()
        .arg("templates")
        .arg("nested-report")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Added:template manifest item"));
}

#[test]
fn templates_named_render_uses_array_of_objects_input_defaults_from_template_json() {
    let root = temp_root("templates-array-object-defaults");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("sprint-summary");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Sprint summary", "version": "1.0.0", "input_defaults": { "sprints": [ { "id": "S1", "stage": "qa" }, { "id": "S2", "stage": "merged" } ] } }"#,
    );
    write_file(
        &pack.join("report.md.j2"),
        "{% for sprint in sprints %}{{ sprint.id }}:{{ sprint.stage }}\n{% endfor %}",
    );

    let output = sc_compose()
        .arg("templates")
        .arg("sprint-summary")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("S1:qa"));
    assert!(stdout.contains("S2:merged"));
}

#[test]
fn report_catalog_loads_valid_catalog_from_repo_root() {
    let root = temp_root("report-catalog-valid");
    write_report_catalog(&root, valid_report_catalog());

    let output = sc_compose()
        .arg("report-catalog")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("catalog:"));
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("sc-lint kind=lint producer=just lint required=true"));
}

#[test]
fn report_catalog_rejects_duplicate_ids_before_generation() {
    let root = temp_root("report-catalog-duplicate");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"

[[report]]
id = "sc-lint"
kind = "smoke"
producer = "just smoke"
required = false
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PARSE"));
    assert!(stderr.contains("duplicate report id 'sc-lint'"));
}

#[test]
fn report_catalog_rejects_missing_required_fields() {
    let root = temp_root("report-catalog-missing-field");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing required field 'producer'"));
}

#[test]
fn report_catalog_rejects_unknown_kind() {
    let root = temp_root("report-catalog-kind");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "mystery"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unsupported kind 'mystery'"));
}

#[test]
fn report_catalog_rejects_non_normalized_relative_paths() {
    let root = temp_root("report-catalog-path");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "../reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("field 'entrypoint' must be a normalized relative path"));
}

#[test]
fn reports_init_creates_scaffold_and_catalog_passes_validator() {
    let root = temp_root("reports-init");

    let init = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(init.status.success());
    assert!(
        root.join("reports")
            .join("catalog")
            .join("reports.toml")
            .exists()
    );
    assert!(root.join("reports").join("latest").exists());
    assert!(root.join("reports").join("archive").exists());
    assert!(root.join("reports").join("templates").exists());
    assert!(
        root.join("reports")
            .join("smoke")
            .join("reference-template.html.j2")
            .exists()
    );
    assert!(
        root.join("reports")
            .join("smoke")
            .join("sample-vars.json")
            .exists()
    );

    let validate = sc_compose()
        .arg("report-catalog")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(validate.status.success());
    let stdout = String::from_utf8(validate.stdout).unwrap();
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("smoke kind=smoke producer=just smoke required=true"));
}

#[test]
fn reports_smoke_writes_latest_smoke_artifact_set() {
    let root = temp_root("reports-smoke");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);

    let output = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let entrypoint = root
        .join("reports")
        .join("latest")
        .join("smoke")
        .join("index.html");
    let metadata = root
        .join("reports")
        .join("latest")
        .join("smoke")
        .join("report.json");
    assert!(entrypoint.exists());
    assert!(metadata.exists());
    assert!(
        fs::read_to_string(&entrypoint)
            .unwrap()
            .contains("Smoke Report")
    );
    let metadata_text = fs::read_to_string(&metadata).unwrap();
    assert!(metadata_text.contains("\"report_id\": \"smoke\""));
    assert!(metadata_text.contains("\"entrypoint\": \"reports/latest/smoke/index.html\""));
    assert!(metadata_text.contains("\"produced_at\":"));
    assert!(metadata_text.contains("\"status\": \"pass\""));
}

#[test]
fn reports_smoke_archive_writes_timestamped_archive_copy() {
    let root = temp_root("reports-smoke-archive");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);

    let output = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--archive")
        .output()
        .unwrap();

    assert!(output.status.success());
    let archive_root = root.join("reports").join("archive");
    let archive_entries = fs::read_dir(&archive_root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(archive_entries.len(), 1);
    let archived_smoke_dir = archive_entries[0].join("smoke");
    assert!(archived_smoke_dir.join("index.html").exists());
    assert!(archived_smoke_dir.join("report.json").exists());
}

#[test]
fn reports_index_summarizes_latest_report_status() {
    let root = temp_root("reports-index");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);
    write_report_catalog(
        &root,
        r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#,
    );

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .output()
        .unwrap();
    assert!(smoke.status.success());

    let index = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(index.status.success());
    let stdout = String::from_utf8(index.stdout).unwrap();
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("smoke kind=smoke required=true status=pass"));
    assert!(stdout.contains("entrypoint=reports/latest/smoke/index.html"));
}

#[test]
fn reports_verify_fails_when_required_evidence_is_missing() {
    let root = temp_root("reports-verify-missing");
    write_report_catalog(
        &root,
        r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#,
    );

    let verify = sc_compose()
        .arg("reports")
        .arg("verify")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(verify.status.code(), Some(3));
    let stderr = String::from_utf8(verify.stderr).unwrap();
    assert!(stderr.contains("missing required report evidence for smoke"));
    assert!(stderr.contains("reports/latest/smoke/index.html"));
}

#[test]
fn reports_publish_manifest_writes_machine_readable_handoff() {
    let root = temp_root("reports-publish-manifest");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);
    write_report_catalog(
        &root,
        r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#,
    );

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--archive")
        .output()
        .unwrap();
    assert!(smoke.status.success());

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(publish_manifest.status.success());
    let stdout = String::from_utf8(publish_manifest.stdout).unwrap();
    assert!(stdout.contains("manifest: reports/latest/publish-manifest.json"));
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("smoke kind=smoke entrypoint=reports/latest/smoke/index.html"));
    assert!(stdout.contains("publish_to=reports/smoke/index.html"));
    assert!(stdout.contains("archive_root: reports/archive/"));

    let manifest_path = root
        .join("reports")
        .join("latest")
        .join("publish-manifest.json");
    assert!(manifest_path.exists());
    let manifest_text = fs::read_to_string(manifest_path).unwrap();
    assert!(manifest_text.contains("\"report_id\": \"smoke\""));
    assert!(manifest_text.contains("\"publish_to\": \"reports/smoke/index.html\""));
    assert!(manifest_text.contains("\"archive_root\": \"reports/archive/"));
}

#[test]
fn reports_publish_manifest_rejects_parent_dir_artifact_from_metadata() {
    let root = temp_root("reports-publish-manifest-parent-dir");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);
    write_report_catalog(
        &root,
        r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#,
    );

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .output()
        .unwrap();
    assert!(smoke.status.success(), "{smoke:?}");

    write_file(
        &root.join("reports").join("latest").join("escaped.html"),
        "<!DOCTYPE html><html><body>escaped</body></html>\n",
    );
    write_file(
        &root
            .join("reports")
            .join("latest")
            .join("smoke")
            .join("report.json"),
        r#"{
  "report_id": "smoke",
  "kind": "smoke",
  "produced_at": "2026-05-27T03:40:00Z",
  "status": "pass",
  "entrypoint": "reports/latest/smoke/index.html",
  "artifacts": [
    "reports/latest/smoke/index.html",
    "reports/latest/smoke/report.json",
    "reports/latest/smoke/../escaped.html"
  ]
}
"#,
    );

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(publish_manifest.status.code(), Some(3));
    let stderr = String::from_utf8(publish_manifest.stderr).unwrap();
    assert!(stderr.contains("invalid publish-manifest artifact path for smoke"));
    assert!(stderr.contains("path must not contain '..' segments"));
}

#[test]
fn reports_finalize_writes_shared_sidecar_for_generic_producer_outputs() {
    let root = temp_root("reports-finalize");
    write_file(
        &root
            .join("reports")
            .join("latest")
            .join("sc-lint")
            .join("index.html"),
        "<!DOCTYPE html><html><body>lint</body></html>\n",
    );
    write_file(
        &root
            .join("reports")
            .join("latest")
            .join("sc-lint")
            .join("panels")
            .join("manifest.json"),
        "{}\n",
    );

    finalize_report(
        &root,
        "sc-lint",
        "lint",
        "reports/latest/sc-lint/index.html",
        &[
            "reports/latest/sc-lint/index.html",
            "reports/latest/sc-lint/panels/manifest.json",
        ],
    );

    let metadata = root
        .join("reports")
        .join("latest")
        .join("sc-lint")
        .join("report.json");
    assert!(metadata.exists());
    let metadata_text = fs::read_to_string(&metadata).unwrap();
    assert!(metadata_text.contains("\"report_id\": \"sc-lint\""));
    assert!(metadata_text.contains("\"kind\": \"lint\""));
    assert!(metadata_text.contains("\"entrypoint\": \"reports/latest/sc-lint/index.html\""));

    let archive_root = root.join("reports").join("archive");
    assert!(archive_root.exists());
}

#[test]
fn reports_render_spec_writes_mermaid_latest_artifact_set() {
    let root = temp_root("reports-render-spec");
    write_state_machine_spec(&root, "reports/specs/state-diagrams.toml");

    let output = sc_compose()
        .arg("reports")
        .arg("render-spec")
        .arg("--root")
        .arg(&root)
        .arg("--spec")
        .arg("reports/specs/state-diagrams.toml")
        .arg("--archive")
        .output()
        .unwrap();

    assert!(output.status.success());
    let entrypoint = root
        .join("reports")
        .join("latest")
        .join("state-diagrams")
        .join("index.mmd");
    let metadata = root
        .join("reports")
        .join("latest")
        .join("state-diagrams")
        .join("report.json");
    assert!(entrypoint.exists());
    assert!(metadata.exists());
    let rendered = fs::read_to_string(&entrypoint).unwrap();
    assert!(rendered.contains("stateDiagram-v2"));
    assert!(rendered.contains("accepted --> validated : validate_ok"));
    let metadata_text = fs::read_to_string(&metadata).unwrap();
    assert!(metadata_text.contains("\"report_id\": \"state-diagrams\""));
    assert!(metadata_text.contains("\"kind\": \"state_machine\""));
}

#[test]
fn reports_render_spec_participates_in_publish_manifest_pipeline() {
    let root = temp_root("reports-render-spec-publish-manifest");
    write_state_machine_spec(&root, "reports/specs/state-diagrams.toml");
    write_report_catalog(
        &root,
        r#"[[report]]
id = "state-diagrams"
kind = "state_machine"
producer = "just state-diagrams"
required = true
entrypoint = "reports/latest/state-diagrams/index.mmd"
metadata = "reports/latest/state-diagrams/report.json"
"#,
    );

    let render = sc_compose()
        .arg("reports")
        .arg("render-spec")
        .arg("--root")
        .arg(&root)
        .arg("--spec")
        .arg("reports/specs/state-diagrams.toml")
        .arg("--archive")
        .output()
        .unwrap();
    assert!(render.status.success());

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(publish_manifest.status.success());
    let manifest_text = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("publish-manifest.json"),
    )
    .unwrap();
    assert!(manifest_text.contains("\"report_id\": \"state-diagrams\""));
    assert!(manifest_text.contains("\"path\": \"reports/latest/state-diagrams/index.mmd\""));
    assert!(manifest_text.contains("\"publish_to\": \"reports/state-diagrams/index.mmd\""));
}

#[test]
fn report_render_many_accepts_semantic_spec_inputs_for_diagram_family() {
    let root = temp_root("report-render-many-semantic-spec");
    write_state_machine_spec(&root, "docs/specs/first.toml");
    write_sql_query_spec(&root, "docs/specs/second.toml");

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-diagrams")
        .arg("--glob")
        .arg("docs/specs/*.toml")
        .arg("--template-family")
        .arg("diagram")
        .arg("--output-dir")
        .arg("reports/latest/state-diagrams")
        .output()
        .unwrap();

    assert!(output.status.success());
    let first = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("state-diagrams")
            .join("docs")
            .join("specs")
            .join("first.html"),
    )
    .unwrap();
    assert!(first.contains("Diagram report family"));
    assert!(first.contains("stateDiagram-v2"));
    let second = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("state-diagrams")
            .join("docs")
            .join("specs")
            .join("second.html"),
    )
    .unwrap();
    assert!(second.contains("flowchart TD"));
    assert!(second.contains("read: orders"));
}

#[test]
fn phase_b_reference_fixtures_produce_publish_manifest_for_distinct_report_families() {
    let root = temp_root("phase-b-b8-reference");
    stage_phase_b_reference_assets(&root);

    let lint = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("sc-lint")
        .arg("--glob")
        .arg("reports/inputs/lint/*.md")
        .arg("--template-family")
        .arg("lint")
        .arg("--output-dir")
        .arg("reports/latest/sc-lint/panels")
        .output()
        .unwrap();
    assert!(lint.status.success(), "{lint:?}");
    render_report_summary(
        &root,
        "reports/vars/sc-lint-summary.json",
        "reports/latest/sc-lint/index.html",
    );
    finalize_report(
        &root,
        "sc-lint",
        "lint",
        "reports/latest/sc-lint/index.html",
        &[
            "reports/latest/sc-lint/index.html",
            "reports/latest/sc-lint/panels/manifest.json",
            "reports/latest/sc-lint/panels/reports/inputs/lint/summary.html",
            "reports/latest/sc-lint/panels/reports/inputs/lint/whitespace.html",
        ],
    );

    let test = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("test-evidence")
        .arg("--glob")
        .arg("reports/inputs/test/*.md")
        .arg("--template-family")
        .arg("test")
        .arg("--output-dir")
        .arg("reports/latest/test-evidence/panels")
        .output()
        .unwrap();
    assert!(test.status.success(), "{test:?}");
    render_report_summary(
        &root,
        "reports/vars/test-evidence-summary.json",
        "reports/latest/test-evidence/index.html",
    );
    finalize_report(
        &root,
        "test-evidence",
        "test",
        "reports/latest/test-evidence/index.html",
        &[
            "reports/latest/test-evidence/index.html",
            "reports/latest/test-evidence/panels/manifest.json",
            "reports/latest/test-evidence/panels/reports/inputs/test/results.html",
            "reports/latest/test-evidence/panels/reports/inputs/test/matrix.html",
        ],
    );

    let init = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--archive")
        .output()
        .unwrap();
    assert!(smoke.status.success(), "{smoke:?}");

    let state = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-diagrams")
        .arg("--glob")
        .arg("reports/specs/state-diagrams/*.toml")
        .arg("--template-family")
        .arg("diagram")
        .arg("--output-dir")
        .arg("reports/latest/state-diagrams/panels")
        .output()
        .unwrap();
    assert!(state.status.success(), "{state:?}");
    render_report_summary(
        &root,
        "reports/vars/state-diagrams-summary.json",
        "reports/latest/state-diagrams/index.html",
    );
    finalize_report(
        &root,
        "state-diagrams",
        "state_machine",
        "reports/latest/state-diagrams/index.html",
        &[
            "reports/latest/state-diagrams/index.html",
            "reports/latest/state-diagrams/panels/manifest.json",
            "reports/latest/state-diagrams/panels/reports/specs/state-diagrams/approval-flow.html",
            "reports/latest/state-diagrams/panels/reports/specs/state-diagrams/retry-loop.html",
        ],
    );

    let sql = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("sql-diagrams")
        .arg("--glob")
        .arg("reports/specs/sql-diagrams/*.toml")
        .arg("--template-family")
        .arg("diagram")
        .arg("--output-dir")
        .arg("reports/latest/sql-diagrams/panels")
        .output()
        .unwrap();
    assert!(sql.status.success(), "{sql:?}");
    render_report_summary(
        &root,
        "reports/vars/sql-diagrams-summary.json",
        "reports/latest/sql-diagrams/index.html",
    );
    finalize_report(
        &root,
        "sql-diagrams",
        "sql_query",
        "reports/latest/sql-diagrams/index.html",
        &[
            "reports/latest/sql-diagrams/index.html",
            "reports/latest/sql-diagrams/panels/manifest.json",
            "reports/latest/sql-diagrams/panels/reports/specs/sql-diagrams/publish-manifest.html",
            "reports/latest/sql-diagrams/panels/reports/specs/sql-diagrams/release-summary.html",
        ],
    );

    render_report_summary(
        &root,
        "examples/report-evidence-summary.sample-vars.json",
        "reports/latest/report-evidence-summary/index.html",
    );
    finalize_report(
        &root,
        "report-evidence-summary",
        "custom",
        "reports/latest/report-evidence-summary/index.html",
        &["reports/latest/report-evidence-summary/index.html"],
    );

    let verify = sc_compose()
        .arg("reports")
        .arg("verify")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(verify.status.success(), "{verify:?}");

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(publish_manifest.status.success(), "{publish_manifest:?}");

    let state_panel = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("state-diagrams")
            .join("panels")
            .join("reports")
            .join("specs")
            .join("state-diagrams")
            .join("approval-flow.html"),
    )
    .unwrap();
    assert!(state_panel.contains("Diagram report family"));
    assert!(state_panel.contains("Copy json"));

    let lint_panel = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("sc-lint")
            .join("panels")
            .join("reports")
            .join("inputs")
            .join("lint")
            .join("whitespace.html"),
    )
    .unwrap();
    assert!(lint_panel.contains("Whitespace edge case"));

    let manifest_text = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("publish-manifest.json"),
    )
    .unwrap();
    assert!(manifest_text.contains("\"report_id\": \"sc-lint\""));
    assert!(manifest_text.contains("\"report_id\": \"test-evidence\""));
    assert!(manifest_text.contains("\"report_id\": \"smoke\""));
    assert!(manifest_text.contains("\"report_id\": \"state-diagrams\""));
    assert!(manifest_text.contains("\"report_id\": \"sql-diagrams\""));
    assert!(manifest_text.contains("\"report_id\": \"report-evidence-summary\""));
}

#[test]
fn report_evidence_summary_renders_large_variable_payload() {
    let root = temp_root("report-evidence-summary-large-payload");
    stage_phase_b_reference_assets(&root);
    let vars_path = root
        .join("examples")
        .join("report-evidence-summary.large-vars.json");
    let output_path = root
        .join("reports")
        .join("latest")
        .join("report-evidence-summary")
        .join("large.html");
    write_large_report_evidence_vars(&vars_path);
    fs::create_dir_all(output_path.parent().unwrap()).unwrap();

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("examples/report-evidence-summary.html.j2")
        .arg("--var-file")
        .arg(&vars_path)
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let rendered = fs::read_to_string(&output_path).unwrap();
    assert!(rendered.contains("Large Report Evidence Summary"));
    assert!(rendered.contains("Generated item 0"));
    assert!(rendered.contains("Generated item 159"));
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
fn templates_add_directory_creates_pack_and_readme_and_named_render_uses_input_defaults() {
    let root = temp_root("templates-add-dir");
    let templates_root = root.join("user-templates");
    let source_dir = root.join("report-pack");
    write_file(
        &source_dir.join("template.json"),
        r#"{ "description": "Report template", "version": "1.0.0", "input_defaults": { "name": "world" } }"#,
    );
    write_file(&source_dir.join("report.md.j2"), "Hello {{ name }}!\n");
    write_file(&source_dir.join("README.txt"), "asset");

    let add_output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source_dir)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(add_output.status.success());
    assert!(templates_root.join("README.md").exists());
    assert!(
        templates_root
            .join("report-pack")
            .join("README.txt")
            .exists()
    );

    let list_output = sc_compose()
        .arg("templates")
        .arg("list")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(list_output.status.success());
    assert!(
        String::from_utf8(list_output.stdout)
            .unwrap()
            .contains("report-pack")
    );

    let render_output = sc_compose()
        .arg("templates")
        .arg("report-pack")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(render_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&render_output.stdout).trim(),
        "Hello world!"
    );
}

#[test]
fn templates_list_with_nonexistent_template_dir_exits_zero_and_prints_nothing() {
    let root = temp_root("templates-list-missing-root");
    let output = sc_compose()
        .arg("templates")
        .arg("list")
        .env(
            "SC_COMPOSE_TEMPLATE_DIR",
            root.join("missing-templates-root"),
        )
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "");
}

#[test]
fn templates_add_duplicate_name_reports_template_exists() {
    let root = temp_root("templates-add-duplicate");
    let templates_root = root.join("user-templates");
    let source = root.join("hello.md.j2");
    write_file(&source, "Hello {{ name }}!\n");

    let first = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();
    assert!(first.status.success());

    let duplicate = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(duplicate.status.code(), Some(3));
    let stderr = String::from_utf8(duplicate.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_TEMPLATE_EXISTS"));
    assert!(stderr.contains("delete the existing template or use a different name"));
}

#[test]
fn templates_add_file_creates_pack_named_from_template_file() {
    let root = temp_root("templates-add-file");
    let templates_root = root.join("user-templates");
    let source = root.join("service-config.yaml.j2");
    write_file(&source, "name: {{ service_name }}\n");

    let output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(templates_root.join("service-config").is_dir());
    assert!(
        templates_root
            .join("service-config")
            .join("service-config.yaml.j2")
            .exists()
    );
}

#[test]
fn templates_named_render_reports_not_renderable_when_multiple_root_templates_exist() {
    let root = temp_root("templates-not-renderable");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("ambiguous");
    write_file(&pack.join("one.md.j2"), "one");
    write_file(&pack.join("two.md.j2"), "two");

    let output = sc_compose()
        .arg("templates")
        .arg("ambiguous")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_RENDERABLE"));
    assert!(stderr.contains("add a .j2 file to the template pack directory"));
}

#[test]
fn templates_named_render_missing_pack_reports_list_recovery_hint() {
    let root = temp_root("templates-missing-pack");
    let templates_root = root.join("user-templates");

    let output = sc_compose()
        .arg("templates")
        .arg("missing-pack")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_FOUND"));
    assert!(stderr.contains("sc-compose templates list"));
}

#[test]
fn templates_named_render_reports_parse_errors_for_invalid_template_manifest() {
    let root = temp_root("templates-invalid-manifest");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("broken");
    write_file(&pack.join("template.json"), "{ invalid json");
    write_file(&pack.join("broken.md.j2"), "hello");

    let output = sc_compose()
        .arg("templates")
        .arg("broken")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PARSE"));
    assert!(!stderr.contains("ERR_CONFIG_PACK_NOT_RENDERABLE"));
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
fn frontmatter_init_dry_run_reports_changed_and_would_change_without_writing() {
    let root = temp_root("frontmatter-dry-run-cli");
    let template = root.join("template.md.j2");
    write_file(&template, "hello {{ name }}\n");

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&template)
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let value = parse_stdout(&output);
    assert_eq!(value["payload"]["changed"], false);
    assert_eq!(value["payload"]["would_change"], true);
    assert_eq!(fs::read_to_string(&template).unwrap(), "hello {{ name }}\n");
}

#[test]
fn init_dry_run_does_not_create_workspace_and_reports_would_create_files() {
    let root = temp_root("init-dry-run-cli");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!root.join(".prompts").exists());
    let value = parse_stdout(&output);
    assert_eq!(value["payload"]["action"], "init");
    assert!(
        !value["payload"]["would_affect"]
            .as_array()
            .unwrap()
            .is_empty()
    );
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
fn init_text_reports_recommendations() {
    let root = temp_root("init-text-recommendations");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("workspace_root:"));
    assert!(stdout.contains("root template has no frontmatter"));
}

#[test]
fn init_dry_run_text_reports_recommendations() {
    let root = temp_root("init-dry-run-text-recommendations");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("would_affect:"));
    assert!(stdout.contains("root template has no frontmatter"));
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

#[test]
fn observability_health_text_reports_process_local_status() {
    let log_root = temp_root("observability-health-text");
    let health = sc_compose()
        .arg("observability-health")
        .env("SC_LOG_ROOT", &log_root)
        .output()
        .unwrap();

    assert!(health.status.success());
    let stdout = String::from_utf8_lossy(&health.stdout);
    assert!(stdout.contains("state: Healthy"));
    assert!(stdout.contains("query_state: Healthy"));
    assert!(stdout.contains("maintenance_state: Running"));
    assert!(stdout.contains("sink jsonl-file: Healthy"));
    assert!(stdout.contains(&format!(
        "active_log_path: {}",
        normalize_path_str(log_root.join("logs").join("sc-compose.log.jsonl"))
    )));
}

#[test]
fn observability_health_json_reports_process_local_status() {
    let log_root = temp_root("observability-health-json");
    let health = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &log_root)
        .output()
        .unwrap();

    assert!(health.status.success());
    let value = parse_stdout(&health);
    assert_eq!(value["payload"]["logging"]["state"], "Healthy");
    assert_eq!(value["payload"]["logging"]["query"]["state"], "Healthy");
    assert_eq!(
        value["payload"]["logging"]["maintenance"]["state"],
        "Running"
    );
    assert_eq!(
        value["payload"]["logging"]["sink_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .find(|sink| sink["name"] == "jsonl-file")
            .unwrap()["state"],
        "Healthy"
    );
    assert_eq!(
        value["payload"]["logging"]["active_log_path"],
        normalize_path_str(log_root.join("logs").join("sc-compose.log.jsonl"))
    );
}

#[test]
fn release_smoke_covers_render_pipeline_and_observability_health() {
    let root = temp_root("release-smoke-observability");
    let logs_root = root.join("telemetry");
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

    let render = sc_compose()
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
        .env("SC_LOG_ROOT", &logs_root)
        .output()
        .unwrap();

    assert!(render.status.success());
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "Name: Casey\nTitle: Engineer\nMood: focused"
    );
    assert!(logs_root.join("logs").join("sc-compose.log.jsonl").exists());

    let health = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &logs_root)
        .output()
        .unwrap();

    assert!(health.status.success());
    let value = parse_stdout(&health);
    assert_eq!(value["payload"]["logging"]["state"], "Healthy");
    assert_eq!(
        value["payload"]["logging"]["active_log_path"],
        normalize_path_str(logs_root.join("logs").join("sc-compose.log.jsonl"))
    );
    assert_eq!(
        value["payload"]["logging"]["maintenance"]["state"],
        "Running"
    );
}

#[test]
fn reports_smoke_keeps_observability_health_green_under_logger_12() {
    let root = temp_root("reports-smoke-observability");
    let logs_root = root.join("telemetry");
    write_smoke_fixture(&root);

    let init = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--archive")
        .env("SC_LOG_ROOT", &logs_root)
        .output()
        .unwrap();

    assert!(smoke.status.success(), "{smoke:?}");

    let health = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &logs_root)
        .output()
        .unwrap();

    assert!(health.status.success());
    let value = parse_stdout(&health);
    assert_eq!(value["payload"]["logging"]["state"], "Healthy");
    assert_eq!(value["payload"]["logging"]["query"]["state"], "Healthy");
    assert_eq!(
        value["payload"]["logging"]["active_log_path"],
        normalize_path_str(logs_root.join("logs").join("sc-compose.log.jsonl"))
    );
}

#[test]
fn report_render_many_renders_one_output_per_source_in_sorted_order() {
    let root = temp_root("report-render-many");
    write_render_many_fixture(&root);
    write_file(
        &root.join("docs").join("diagrams").join("b.txt"),
        "# title: Bravo\n# sets:\n#   - publish\nbravo body\n",
    );
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "# title: Alpha\nalpha body\n",
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("reports/templates/panel.html.j2")
        .arg("--output-dir")
        .arg("reports/latest/panels")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(
            root.join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("diagrams")
                .join("a.html")
        )
        .unwrap(),
        "<article>Alpha|alpha body|reports/latest/panels/docs/diagrams/a.html</article>"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("diagrams")
                .join("b.html")
        )
        .unwrap(),
        "<article>Bravo|bravo body|reports/latest/panels/docs/diagrams/b.html|publish</article>"
    );

    let manifest = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("panels")
            .join("manifest.json"),
    )
    .unwrap();
    let alpha_index = manifest
        .find("\"source_path\": \"docs/diagrams/a.txt\"")
        .unwrap();
    let bravo_index = manifest
        .find("\"source_path\": \"docs/diagrams/b.txt\"")
        .unwrap();
    assert!(alpha_index < bravo_index);
}

#[test]
fn report_render_many_accepts_repo_relative_root_argument() {
    let root = temp_root("report-render-many-relative-root");
    write_render_many_fixture(&root);
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "# title: Alpha\nalpha body\n",
    );

    let output = sc_compose()
        .current_dir(&root)
        .arg("report-render-many")
        .arg("--root")
        .arg(".")
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("reports/templates/panel.html.j2")
        .arg("--output-dir")
        .arg("reports/latest/panels")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_to_string(
            root.join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("diagrams")
                .join("a.html")
        )
        .unwrap(),
        "<article>Alpha|alpha body|reports/latest/panels/docs/diagrams/a.html</article>"
    );
}

#[test]
fn report_render_many_supports_shared_diagram_template_selector() {
    let root = temp_root("report-render-many-shared");
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "# title: Alpha\n# fragment_href: /reports/alpha\nalpha body\n",
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("shared:diagram")
        .arg("--output-dir")
        .arg("reports/latest/diagrams")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let rendered = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("diagrams")
            .join("docs")
            .join("diagrams")
            .join("a.html"),
    )
    .unwrap();
    assert!(rendered.contains("Diagram report family"));
    assert!(rendered.contains("<pre>alpha body</pre>"));
    assert!(rendered.contains("Copy text"));
    assert!(rendered.contains("Open fragment"));
    assert!(!rendered.contains("Copy json"));
}

#[test]
fn report_render_many_uses_repo_local_template_family_override() {
    let root = temp_root("report-render-many-family-override");
    write_file(
        &root.join("docs").join("lint").join("a.txt"),
        "# title: Lint Alpha\n# copy_json:\n#   status: pass\nalpha body\n",
    );
    write_report_family_override(&root);
    write_report_catalog(
        &root,
        r#"[reporting.templates.lint]
path = "reports/templates/lint/report.html.j2"
"#,
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("lint")
        .arg("--glob")
        .arg("docs/lint/*.txt")
        .arg("--template-family")
        .arg("lint")
        .arg("--output-dir")
        .arg("reports/latest/lint")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let rendered = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("lint")
            .join("docs")
            .join("lint")
            .join("a.html"),
    )
    .unwrap();
    assert!(rendered.contains("Lint override"));
    assert!(rendered.contains("Override body marker"));
    assert!(rendered.contains("Copy text"));
    assert!(rendered.contains("Copy json"));
}

#[cfg(unix)]
fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    use std::os::windows::fs::symlink_file;

    match symlink_file(target, link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(_) => false,
    }
}
