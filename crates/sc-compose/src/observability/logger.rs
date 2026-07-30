use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use sc_observability::{
    ConsoleSink, Logger, LoggerConfig, RetainedLogPolicy, ServiceName, SinkRegistration,
};

use crate::CommandError;
use crate::observability::SERVICE_NAME;

const DEFAULT_LOG_ROOT_DIR: &str = ".sc-compose";

pub(crate) fn build_logger(wants_json: bool) -> Result<Logger, CommandError> {
    build_logger_for_root(default_log_root()?, wants_json)
}

pub(crate) fn build_logger_for_root(
    log_root: PathBuf,
    wants_json: bool,
) -> Result<Logger, CommandError> {
    let mut builder = Logger::builder(build_logger_config(log_root)?).map_err(|error| {
        CommandError::usage(anyhow!(error).context("failed to initialize observability logger"))
    })?;
    if !wants_json {
        builder.register_sink(SinkRegistration::new(Arc::new(ConsoleSink::stderr())));
    }
    Ok(builder.build())
}

fn default_log_root() -> Result<PathBuf, CommandError> {
    default_log_root_with(std::env::current_dir)
}

fn build_service_name() -> Result<ServiceName, CommandError> {
    ServiceName::new(SERVICE_NAME).map_err(|error| {
        CommandError::usage(anyhow!("invalid observability service name: {error}"))
    })
}

pub(super) fn build_logger_config(log_root: PathBuf) -> Result<LoggerConfig, CommandError> {
    let mut config = LoggerConfig::default_for(build_service_name()?, log_root);
    config.enable_console_sink = false;
    // Keep logger-managed retained-log maintenance enabled using
    // sc-observability 1.2.0 defaults rather than adding a repo-local policy.
    config.retained_log_policy = RetainedLogPolicy::default();
    Ok(config)
}

pub(super) fn default_log_root_with(
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
