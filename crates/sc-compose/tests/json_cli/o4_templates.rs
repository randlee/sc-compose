use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::support::*;

const TEMPLATES: &[&str] = &[
    ".claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2",
    ".claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2",
    ".claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2",
    ".claude/skills/codex-orchestration/arch-qa-assignment.json.j2",
    ".claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2",
    ".claude/skills/codex-orchestration/req-qa-assignment.json.j2",
];

fn hostile_context() -> Value {
    let hostile = "quote \" slash \\\u{000a}line ☃\u{0001}";
    json!({
        "review_mode": hostile,
        "worktree_path": "/tmp/worktree/\"hostile\"",
        "review_targets": ["src/lib.rs", "docs/☃.md"],
        "practice_mode": hostile,
        "practice_ids": ["RBP-001", "RBP-\"hostile\""],
        "round_limit": true,
        "changed_files": ["src/\"hostile\".rs", "docs/line\nend.md"],
        "carry_forward_findings_json": r#"[{"rule_id":"x\"","message":"safe"}]"#,
        "triage_records": ["record \"one\"", "record\ntwo"],
        "notes": hostile,
        "fmt": true,
        "clippy": true,
        "tests": true,
        "coverage": false,
        "baseline_ref": hostile,
        "artifact_regeneration_required": true,
        "artifact_commands": hostile,
        "topics": ["timeouts", "☃"],
        "service_indicators_extra": ["custom \"indicator\""],
        "review_type": hostile,
        "branch": "feature/hostile-branch",
        "commit": "deadbeef",
        "phase": "O",
        "sprint": "O.4",
        "sprint_doc": "docs/phase-O/sprint-o-4-template-migration.md",
        "reference_docs": ["docs/requirements.md", "docs/☃.md"],
    })
}

fn render_template(path: &str, vars: &Path) -> Value {
    let output = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            repo_root().to_str().unwrap(),
            "--file",
            path,
            "--var-file",
            vars.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{path} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{path} rendered invalid JSON: {error}; output={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn six_known_templates_render_semantically_with_hostile_values() {
    let root = temp_root("o4-six-template-hostile");
    let vars = root.join("vars.json");
    write_file(&vars, &hostile_context().to_string());

    for path in TEMPLATES {
        let rendered = render_template(path, &vars);
        assert!(rendered.is_object(), "{path} must render a JSON object");
        assert!(rendered.get("injected").is_none(), "{path} was injectable");
    }

    let best_practices = render_template(TEMPLATES[0], &vars);
    assert_eq!(
        best_practices["review_mode"],
        "quote \" slash \\\u{000a}line ☃\u{0001}"
    );
    assert_eq!(best_practices["review_targets"][1], "docs/☃.md");
    assert_eq!(
        best_practices["carry_forward_findings"][0]["rule_id"],
        "x\""
    );

    let arch = render_template(TEMPLATES[3], &vars);
    assert_eq!(arch["scope"]["sprint"], "O.4");
    assert_eq!(arch["reference_docs"][1], "docs/☃.md");

    let req = render_template(TEMPLATES[5], &vars);
    assert_eq!(
        req["authoritative_sprint_doc"],
        "docs/phase-O/sprint-o-4-template-migration.md"
    );
    assert_eq!(req["branch"], "feature/hostile-branch");
}

#[test]
fn six_known_templates_validate_with_lint_in_auto_mode() {
    let root = temp_root("o4-six-template-lint");
    let vars = root.join("vars.json");
    write_file(&vars, &hostile_context().to_string());

    for path in TEMPLATES {
        let output = sc_compose()
            .args([
                "validate",
                "--lint",
                "--mode",
                "file",
                "--root",
                repo_root().to_str().unwrap(),
                "--file",
                path,
                "--var-file",
                vars.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{path} failed validation: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let payload = parse_stdout(&output);
        assert_eq!(payload["payload"]["valid"], true, "{path}");
        assert!(
            !payload["diagnostics"]
                .to_string()
                .contains("ERR_JSON_MODE_CONTRACT")
        );
    }
}

#[test]
fn optional_and_empty_template_values_keep_json_types() {
    let root = temp_root("o4-six-template-empty-null");
    let vars = root.join("vars.json");
    let mut context = hostile_context();
    let values = context.as_object_mut().unwrap();
    for key in [
        "review_mode",
        "worktree_path",
        "practice_mode",
        "baseline_ref",
        "artifact_commands",
        "notes",
        "review_type",
        "branch",
        "commit",
        "sprint_doc",
    ] {
        values.insert(key.to_owned(), Value::String(String::new()));
    }
    for key in ["phase", "sprint"] {
        values.insert(key.to_owned(), Value::Null);
    }
    for key in [
        "review_targets",
        "practice_ids",
        "changed_files",
        "triage_records",
        "topics",
        "service_indicators_extra",
        "reference_docs",
    ] {
        values.insert(key.to_owned(), Value::Array(Vec::new()));
    }
    write_file(&vars, &context.to_string());

    for path in TEMPLATES {
        let rendered = render_template(path, &vars);
        assert!(rendered.is_object(), "{path} must render an object");
        assert!(rendered.get("injected").is_none(), "{path} was injectable");
    }
    assert_eq!(
        render_template(TEMPLATES[3], &vars)["scope"]["phase"],
        Value::Null
    );
    assert_eq!(
        render_template(TEMPLATES[4], &vars)["scope"]["sprint"],
        Value::Null
    );
    assert_eq!(
        render_template(TEMPLATES[5], &vars)["worktree_path"],
        Value::Null
    );
}

#[test]
fn template_contract_lint_passes_for_a_clean_o4_corpus() {
    let root = temp_root("o4-template-contract-lint");
    for path in TEMPLATES {
        let destination = root.join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(repo_root().join(path), destination).unwrap();
    }
    let target = root.join(".sc/sc-lint/targets/template-contracts.toml");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::copy(
        repo_root().join(".sc/sc-lint/targets/template-contracts.toml"),
        target,
    )
    .unwrap();

    let output = sc_compose()
        .args([
            "lint",
            "--target",
            "template-contracts",
            "--root",
            root.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "clean O.4 corpus failed lint: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let payload = parse_stdout(&output);
    assert_eq!(payload["payload"]["raw_payload"]["data"]["status"], "pass");
    assert_eq!(
        payload["payload"]["raw_payload"]["data"]["templates_scanned"],
        6
    );
}

#[test]
fn legacy_compatibility_fixture_is_valid_and_warns_once() {
    let root = temp_root("o4-legacy-compatibility");
    let template = root.join("legacy.json.j2");
    write_file(
        &template,
        "---\njson_escape_mode: legacy\n---\n{\"value\": \"{{ value }}\"}\n",
    );
    let vars = root.join("vars.json");
    write_file(
        &vars,
        &json!({"value": "quote \" slash \\\nline"}).to_string(),
    );

    let rendered = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "legacy.json.j2",
            "--var-file",
            vars.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(rendered.status.success(), "{rendered:?}");
    let value: Value = serde_json::from_slice(&rendered.stdout).unwrap();
    assert_eq!(value["value"], "quote \" slash \\\u{000a}line");

    let validation = sc_compose()
        .args([
            "validate",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "legacy.json.j2",
            "--var-file",
            vars.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(validation.status.success(), "{validation:?}");
    let payload = parse_stdout(&validation);
    assert_eq!(
        payload["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|diagnostic| diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE")
            .count(),
        1
    );
}
