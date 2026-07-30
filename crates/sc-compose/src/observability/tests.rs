use std::path::PathBuf;

use sc_observability_types::{
    DiagnosticSummary, LoggingHealthReport, LoggingHealthState, QueryHealthReport,
    QueryHealthState, Timestamp, WriterState,
};
use serde::Serialize;

use super::json::{health_json_from_serialized, health_json_value};
use super::logger::{build_logger_config, default_log_root_with};
use super::text::render_health_text_lines;

#[test]
fn build_logger_config_disables_console_sink_and_keeps_default_retention() {
    let config = build_logger_config(PathBuf::from("logs")).expect("logger config");

    assert!(!config.enable_console_sink);
    assert_eq!(config.log_root, PathBuf::from("logs"));
}

#[test]
fn health_json_value_nulls_unavailable_query_state() {
    let health = sample_health();
    let value = health_json_value(&health);

    assert!(value["query"].is_null());
    assert_eq!(value["active_log_path"], "logs/sc-compose.log.jsonl");
}

#[test]
fn health_json_value_falls_back_when_serialization_fails() {
    #[derive(Clone, Copy)]
    struct FailingSerialize;

    impl Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("boom"))
        }
    }

    let health = sample_health();
    let value = health_json_from_serialized(serde_json::to_value(FailingSerialize), &health);

    assert_eq!(value["state"], "Healthy");
    assert_eq!(value["active_log_path"], "logs/sc-compose.log.jsonl");
    let Some(serialization_error) = value["serialization_error"].as_str() else {
        panic!("serialization_error should be a string");
    };
    assert!(serialization_error.contains("boom"));
}

#[test]
fn default_log_root_reports_usage_error_when_current_dir_lookup_fails() {
    let result = default_log_root_with(|| Err(std::io::Error::other("cwd missing")));

    let Err(error) = result else {
        panic!("default_log_root_with should fail");
    };
    assert!(format!("{error}").contains("failed to determine current directory"));
}

#[test]
fn render_health_text_lines_formats_unavailable_sections_and_last_error() {
    let mut health = sample_health();
    health.last_error = Some(DiagnosticSummary {
        code: None,
        message: "flush failed".to_owned(),
        at: Timestamp::UNIX_EPOCH,
    });

    let lines = render_health_text_lines(&health);

    assert!(lines.iter().any(|line| line == "query_state: Unavailable"));
    assert!(
        lines
            .iter()
            .any(|line| line == "maintenance_state: unavailable")
    );
    assert!(lines.iter().any(|line| line == "sinks: none"));
    assert!(lines.iter().any(|line| line == "last_error: flush failed"));
}

fn sample_health() -> LoggingHealthReport {
    LoggingHealthReport {
        state: LoggingHealthState::Healthy,
        dropped_events_total: 0,
        flush_errors_total: 0,
        active_log_path: PathBuf::from("logs").join("sc-compose.log.jsonl"),
        sink_statuses: Vec::new(),
        queue_depth: 0,
        queue_capacity: 16,
        queue_high_water_mark: 0,
        queue_full_drops_total: 0,
        writer_state: WriterState::Running,
        last_writer_error: None,
        query: Some(QueryHealthReport {
            state: QueryHealthState::Unavailable,
            last_error: None,
        }),
        maintenance: None,
        last_error: None,
    }
}
