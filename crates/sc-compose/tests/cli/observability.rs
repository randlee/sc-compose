//! Capability-oriented integration tests. Shared mechanics live in `tests/support`.
#![allow(unused_imports)]
use crate::support::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

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
