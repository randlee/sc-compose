use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use sc_observability::{
    ConsoleSink, Logger, LoggerConfig, LoggingHealthReport, RetainedLogPolicy, ServiceName,
    SinkRegistration,
};
use serde::Serialize;
use serde_json::Value;

use crate::CommandError;
use crate::path_utils::to_forward_slash;

const DEFAULT_LOG_ROOT_DIR: &str = ".sc-compose";
pub(crate) const SERVICE_NAME: &str = "sc-compose";

pub(crate) fn build_logger(wants_json: bool) -> Result<Logger, CommandError> {
    build_logger_for_root(default_log_root()?, wants_json)
}

pub(crate) fn build_logger_for_root(
    log_root: PathBuf,
    wants_json: bool,
) -> Result<Logger, CommandError> {
    let service_name = ServiceName::new(SERVICE_NAME).map_err(|error| {
        CommandError::usage(anyhow!("invalid observability service name: {error}"))
    })?;
    let mut config = LoggerConfig::default_for(service_name, log_root);
    config.enable_console_sink = false;
    // Keep logger-managed retained-log maintenance enabled using
    // sc-observability 1.2.0 defaults rather than adding a repo-local policy.
    config.retained_log_policy = RetainedLogPolicy::default();
    let mut builder = Logger::builder(config).map_err(|error| {
        CommandError::usage(anyhow!(error).context("failed to initialize observability logger"))
    })?;
    if !wants_json {
        builder.register_sink(SinkRegistration::new(Arc::new(ConsoleSink::stderr())));
    }
    Ok(builder.build())
}

pub(crate) fn print_observability_health(health: &LoggingHealthReport) {
    println!("state: {:?}", health.state);
    println!(
        "active_log_path: {}",
        to_forward_slash(&health.active_log_path)
    );
    println!("dropped_events_total: {}", health.dropped_events_total);
    println!("flush_errors_total: {}", health.flush_errors_total);

    match &health.query {
        Some(query) => println!("query_state: {:?}", query.state),
        None => println!("query_state: unavailable"),
    }

    match &health.maintenance {
        Some(maintenance) => println!("maintenance_state: {:?}", maintenance.state),
        None => println!("maintenance_state: unavailable"),
    }

    if health.sink_statuses.is_empty() {
        println!("sinks: none");
    } else {
        for sink in &health.sink_statuses {
            println!("sink {}: {:?}", sink.name, sink.state);
        }
    }

    if let Some(last_error) = &health.last_error {
        println!("last_error: {}", last_error.message);
    }
}

/// Convert `LoggingHealthReport` into the CLI JSON payload shape.
pub(crate) fn health_json_value(health: &LoggingHealthReport) -> Value {
    health_json_from_serialized(serialize_health_value(health), health)
}

fn default_log_root() -> Result<PathBuf, CommandError> {
    default_log_root_with(std::env::current_dir)
}

fn default_log_root_with(
    current_dir: impl FnOnce() -> std::io::Result<PathBuf>,
) -> Result<PathBuf, CommandError> {
    if let Ok(path) = std::env::var("SC_LOG_ROOT")
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }

    Ok(current_dir()
        .map_err(|error| {
            CommandError::usage(anyhow!(error).context("failed to determine current directory"))
        })?
        .join(DEFAULT_LOG_ROOT_DIR))
}

fn serialize_health_value<T: Serialize>(value: &T) -> Result<Value, serde_json::Error> {
    serde_json::to_value(value)
}

fn health_json_from_serialized(
    serialized: Result<Value, serde_json::Error>,
    health: &LoggingHealthReport,
) -> Value {
    match serialized {
        Ok(value) => normalize_health_json_value(value, health),
        Err(error) => fallback_health_json_value(health, &error),
    }
}

fn normalize_health_json_value(mut value: Value, health: &LoggingHealthReport) -> Value {
    value["active_log_path"] = Value::String(to_forward_slash(&health.active_log_path));
    if value["query"]["state"] == Value::String("Unavailable".to_owned()) {
        value["query"] = Value::Null;
    }
    value
}

fn fallback_health_json_value(health: &LoggingHealthReport, error: &serde_json::Error) -> Value {
    normalize_health_json_value(
        serde_json::json!({
            "state": health.state,
            "dropped_events_total": health.dropped_events_total,
            "flush_errors_total": health.flush_errors_total,
            "active_log_path": to_forward_slash(&health.active_log_path),
            "sink_statuses": health.sink_statuses,
            "queue_depth": health.queue_depth,
            "queue_capacity": health.queue_capacity,
            "queue_high_water_mark": health.queue_high_water_mark,
            "queue_full_drops_total": health.queue_full_drops_total,
            "writer_state": health.writer_state,
            "last_writer_error": health.last_writer_error,
            "query": health.query,
            "maintenance": health.maintenance,
            "last_error": health.last_error,
            "serialization_error": error.to_string(),
        }),
        health,
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sc_observability_types::{
        LoggingHealthReport, LoggingHealthState, QueryHealthReport, QueryHealthState, WriterState,
    };
    use serde::Serialize;

    use super::{default_log_root_with, health_json_from_serialized, health_json_value};

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
}
