//! Capability-oriented integration tests. Shared mechanics live in `tests/support`.
#![allow(unused_imports)]
use crate::support::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[test]
fn observability_health_json_uses_diagnostic_envelope_and_stays_stdout_clean() {
    let root = temp_root("observability-health-json");

    let output = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["logging"]["state"], "Healthy");
    assert_eq!(value["payload"]["logging"]["query"]["state"], "Healthy");
    assert_eq!(
        value["payload"]["logging"]["maintenance"]["state"],
        "Running"
    );
    assert_eq!(
        value["payload"]["logging"]["active_log_path"],
        normalize_path_str(root.join("logs").join("sc-compose.log.jsonl"))
    );
}

#[test]
fn observability_health_json_nulls_unavailable_query_state() {
    let root = temp_root("observability-health-json-null-query");

    let output = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &root)
        .env("SC_COMPOSE_TEST_FORCE_QUERY_UNAVAILABLE", "1")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(value["payload"]["logging"]["query"].is_null());
    assert_eq!(
        value["payload"]["logging"]["maintenance"]["state"],
        "Stopped"
    );
}

#[test]
fn reports_smoke_json_keeps_observability_health_green_under_logger_12() {
    let root = temp_root("reports-smoke-observability-json");
    let log_root = root.join("telemetry");
    write_smoke_fixture(&root);

    let init = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    assert!(init.status.success());

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
        .arg("--json")
        .env("SC_LOG_ROOT", &log_root)
        .output()
        .unwrap();

    assert!(smoke.status.success());
    assert!(smoke.stderr.is_empty());
    let smoke_value = parse_stdout(&smoke);
    assert_envelope(&smoke_value);
    assert_eq!(smoke_value["payload"]["report_id"], "smoke");

    let health = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &log_root)
        .output()
        .unwrap();

    assert!(health.status.success());
    assert!(health.stderr.is_empty());
    let value = parse_stdout(&health);
    assert_envelope(&value);
    assert_eq!(value["payload"]["logging"]["state"], "Healthy");
    assert_eq!(value["payload"]["logging"]["query"]["state"], "Healthy");
    assert_eq!(
        value["payload"]["logging"]["active_log_path"],
        normalize_path_str(log_root.join("logs").join("sc-compose.log.jsonl"))
    );
}
