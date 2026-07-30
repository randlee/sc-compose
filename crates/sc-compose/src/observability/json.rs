use sc_observability::LoggingHealthReport;
use serde::Serialize;
use serde_json::Value;

use crate::path_utils::to_forward_slash;

/// Convert `LoggingHealthReport` into the CLI JSON payload shape.
pub(crate) fn health_json_value(health: &LoggingHealthReport) -> Value {
    health_json_from_serialized(serialize_health_value(health), health)
}

fn serialize_health_value<T: Serialize>(value: &T) -> Result<Value, serde_json::Error> {
    serde_json::to_value(value)
}

pub(super) fn health_json_from_serialized(
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
