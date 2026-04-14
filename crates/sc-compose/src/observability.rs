use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use sc_observability::{
    ConsoleSink, Logger, LoggerConfig, LoggingHealthReport, ServiceName, SinkRegistration,
};

use crate::CommandError;

const DEFAULT_LOG_ROOT_DIR: &str = ".sc-compose";
const LOG_SERVICE_NAME: &str = "sc-compose";

pub(crate) fn build_logger(wants_json: bool) -> Result<Logger, CommandError> {
    build_logger_for_root(default_log_root()?, wants_json)
}

pub(crate) fn build_logger_for_root(
    log_root: PathBuf,
    wants_json: bool,
) -> Result<Logger, CommandError> {
    let service_name = ServiceName::new(LOG_SERVICE_NAME).map_err(|error| {
        CommandError::usage(anyhow!("invalid observability service name: {error}"))
    })?;
    let mut config = LoggerConfig::default_for(service_name, log_root);
    config.enable_console_sink = false;
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
    println!("active_log_path: {}", health.active_log_path.display());
    println!("dropped_events_total: {}", health.dropped_events_total);
    println!("flush_errors_total: {}", health.flush_errors_total);

    match &health.query {
        Some(query) => println!("query_state: {:?}", query.state),
        None => println!("query_state: unavailable"),
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

fn default_log_root() -> Result<PathBuf, CommandError> {
    if let Ok(path) = std::env::var("SC_LOG_ROOT")
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }

    Ok(std::env::current_dir()
        .map_err(|error| {
            CommandError::usage(anyhow!(error).context("failed to determine current directory"))
        })?
        .join(DEFAULT_LOG_ROOT_DIR))
}
