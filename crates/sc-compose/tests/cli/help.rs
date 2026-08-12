use crate::support::*;

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
    assert_eq!(String::from_utf8_lossy(&output.stdout), "exit-codes\n");
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
fn unknown_manual_topic_returns_usage_failure_and_valid_topics() {
    let output = sc_compose()
        .args(["help", "not-a-real-topic"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown manual topic"), "{stderr}");
    assert!(stderr.contains("exit-codes"), "{stderr}");
}

#[test]
fn root_help_points_to_shipped_manuals() {
    let output = sc_compose().args(["--help"]).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("manuals ship with this CLI"), "{stdout}");
    assert!(stdout.contains("sc-compose help <topic>"), "{stdout}");
}
