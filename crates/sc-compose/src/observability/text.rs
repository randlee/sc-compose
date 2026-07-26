use sc_observability::LoggingHealthReport;

use crate::path_utils::to_forward_slash;

pub(crate) fn print_observability_health(health: &LoggingHealthReport) {
    for line in render_health_text_lines(health) {
        println!("{line}");
    }
}

pub(super) fn render_health_text_lines(health: &LoggingHealthReport) -> Vec<String> {
    let mut lines = vec![
        format!("state: {:?}", health.state),
        format!(
            "active_log_path: {}",
            to_forward_slash(&health.active_log_path)
        ),
        format!("dropped_events_total: {}", health.dropped_events_total),
        format!("flush_errors_total: {}", health.flush_errors_total),
        format!("query_state: {}", query_state_text(health)),
        format!("maintenance_state: {}", maintenance_state_text(health)),
    ];

    if health.sink_statuses.is_empty() {
        lines.push("sinks: none".to_owned());
    } else {
        lines.extend(
            health
                .sink_statuses
                .iter()
                .map(|sink| format!("sink {}: {:?}", sink.name, sink.state)),
        );
    }

    if let Some(last_error) = &health.last_error {
        lines.push(format!("last_error: {}", last_error.message));
    }

    lines
}

fn query_state_text(health: &LoggingHealthReport) -> String {
    match &health.query {
        Some(query) => format!("{:?}", query.state),
        None => "unavailable".to_owned(),
    }
}

fn maintenance_state_text(health: &LoggingHealthReport) -> String {
    match &health.maintenance {
        Some(maintenance) => format!("{:?}", maintenance.state),
        None => "unavailable".to_owned(),
    }
}
