use crate::support::*;

fn assert_topic_contains(topic: &str, expected: &[&str]) {
    let output = sc_compose().args(["help", topic]).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in expected {
        assert!(
            stdout.contains(needle),
            "missing {needle} in {topic} help output: {stdout}"
        );
    }
}

#[test]
fn help_without_topic_prints_manual_index() {
    let output = sc_compose().arg("help").output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Available manual topics:"), "{stdout}");
    assert!(stdout.contains("exit-codes"), "{stdout}");
}

#[test]
fn help_list_is_stable_and_scriptable() {
    let output = sc_compose().args(["help", "--list"]).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "exit-codes\nfrontmatter-init\ninit\nexamples\ntemplates\nreports\n"
    );
}

#[test]
fn exit_codes_manual_documents_all_statuses() {
    let output = sc_compose().args(["help", "exit-codes"]).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for code in ["`0`", "`1`", "`2`", "`3`"] {
        assert!(stdout.contains(code), "missing {code} in {stdout}");
    }
}

#[test]
fn frontmatter_init_manual_documents_file_and_force_flags() {
    assert_topic_contains(
        "frontmatter-init",
        &["frontmatter-init", "--file", "--force"],
    );
}

#[test]
fn init_manual_documents_prompts_workspace_bootstrap() {
    assert_topic_contains("init", &[".prompts/", "--dry-run", "--root"]);
}

#[test]
fn examples_manual_documents_listing_and_var_file_rendering() {
    assert_topic_contains(
        "examples",
        &["examples list", "--var-file", "SC_COMPOSE_DATA_DIR"],
    );
}

#[test]
fn templates_manual_documents_add_and_pack_requirements() {
    assert_topic_contains(
        "templates",
        &["templates add", "template.json", "SC_COMPOSE_TEMPLATE_DIR"],
    );
}

#[test]
fn reports_manual_documents_catalog_and_publish_workflow() {
    assert_topic_contains(
        "reports",
        &[
            "reports smoke",
            "reports/catalog/reports.toml",
            "publish-manifest",
        ],
    );
}

#[test]
fn unknown_manual_topic_returns_usage_failure_and_valid_topics() {
    let output = sc_compose()
        .args(["help", "not-a-real-topic"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown manual topic"), "{stderr}");
    assert!(stderr.contains("exit-codes"), "{stderr}");
    assert!(
        stderr.contains("recovery: run `sc-compose help --list`"),
        "{stderr}"
    );
}

#[test]
fn unknown_manual_topic_json_uses_diagnostic_envelope() {
    let output = sc_compose()
        .args(["help", "not-a-real-topic", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        envelope["diagnostics"][0]["code"],
        "ERR_CONFIG_HELP_TOPIC_NOT_FOUND"
    );
    assert!(
        envelope["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("exit-codes")
    );
}

#[test]
fn help_json_returns_the_standard_envelope() {
    let output = sc_compose().args(["help", "--json"]).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(envelope["payload"]["topics"].is_array());
    assert!(envelope["diagnostics"].is_array());
}

#[test]
fn root_help_points_to_shipped_manuals() {
    let output = sc_compose().args(["--help"]).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("manuals ship with this CLI"), "{stdout}");
    assert!(stdout.contains("sc-compose help <topic>"), "{stdout}");
}
