use std::path::PathBuf;

use serde_json::{Map, Value, json};

use crate::observability::SERVICE_NAME;
use crate::path_utils::to_forward_slash;
use sc_composer::{
    CompositionObserver, IncludeOutcomeEvent, ObservationEvent, ObservationSink,
    RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
};
use sc_observability::{
    ActionName, Diagnostic, ErrorCode, Level, LogEvent, Logger, LoggingHealthReport,
    OBSERVATION_ENVELOPE_VERSION, OutcomeLabel, ProcessIdentity, Remediation, SchemaVersion,
    ServiceName, Stopped, TargetCategory, Timestamp,
};
use sc_observability_types::{
    DiagnosticSummary, LoggingHealthState, QueryHealthReport, QueryHealthState,
    ValueValidationError, WriterState,
};

const FALLBACK_TARGET: &str = "compose.observability";
const FALLBACK_ACTION: &str = "degraded";
const FALLBACK_OUTCOME: &str = "failure";

struct LogRecord {
    family: EventFamily,
    level: Level,
    target: &'static str,
    action: &'static str,
    message: &'static str,
    outcome: Option<&'static str>,
    diagnostic: Option<Diagnostic>,
    fields: Map<String, Value>,
}

#[derive(Clone, Copy)]
enum EventFamily {
    Pipeline,
    Command,
}

impl EventFamily {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pipeline => "pipeline",
            Self::Command => "command",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandStartEvent {
    pub command_name: String,
    pub json_output: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandEndEvent {
    pub command_name: String,
    pub exit_code: i32,
    pub success: bool,
    pub elapsed_ms: u64,
    pub json_output: bool,
    pub diagnostic_code: Option<String>,
    pub diagnostic_message: Option<String>,
}

pub(crate) trait CommandLifecycleObserver {
    fn on_command_start(&mut self, event: &CommandStartEvent);
    fn on_command_end(&mut self, event: &CommandEndEvent);
}

pub(crate) struct CliObserver {
    logger: Option<LoggerOrStopped>,
    service: ServiceName,
}

enum LoggerOrStopped {
    Running(Logger),
    Stopped(Logger<Stopped>),
}

impl CliObserver {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger: Some(LoggerOrStopped::Running(logger)),
            service: service_name(),
        }
    }

    pub fn health(&self) -> LoggingHealthReport {
        match self.logger.as_ref() {
            Some(LoggerOrStopped::Running(logger)) => logger.health(),
            Some(LoggerOrStopped::Stopped(logger)) => logger.health(),
            None => unavailable_health_report("cli observer logger state unavailable"),
        }
    }

    pub fn shutdown(&mut self) {
        let Some(state) = self.logger.take() else {
            return;
        };
        let running = match state {
            LoggerOrStopped::Running(logger) => logger,
            LoggerOrStopped::Stopped(logger) => {
                self.logger = Some(LoggerOrStopped::Stopped(logger));
                return;
            }
        };
        let logger = running.shutdown();
        self.logger = Some(LoggerOrStopped::Stopped(logger));
    }

    fn emit_record(&self, record: LogRecord) {
        let LogRecord {
            family,
            level,
            target,
            action,
            message,
            outcome,
            diagnostic,
            mut fields,
        } = record;
        fields.insert("event_family".to_owned(), json!(family.as_str()));
        let (target, action, outcome) =
            normalize_event_labels(target, action, outcome, &mut fields);
        let event = LogEvent {
            version: schema_version(),
            timestamp: Timestamp::now_utc(),
            level,
            service: self.service.clone(),
            target,
            action,
            message: Some(message.to_owned()),
            identity: ProcessIdentity::default(),
            trace: None,
            request_id: None,
            correlation_id: None,
            outcome,
            diagnostic,
            state_transition: None,
            fields,
        };

        if let Some(LoggerOrStopped::Running(logger)) = &self.logger {
            let _ignored = logger.log(event);
        }
    }

    fn emit_pipeline_record(&self, record: LogRecord) {
        self.emit_record(LogRecord {
            family: EventFamily::Pipeline,
            ..record
        });
    }

    fn emit_command_record(&self, record: LogRecord) {
        self.emit_record(LogRecord {
            family: EventFamily::Command,
            ..record
        });
    }
}

fn event_diagnostic(code: &str, message: &str, fields: &Map<String, Value>) -> Diagnostic {
    Diagnostic {
        timestamp: Timestamp::now_utc(),
        code: ErrorCode::new_owned(code),
        message: message.to_owned(),
        cause: None,
        remediation: Remediation::not_recoverable(
            "consult the command diagnostic and correct the reported input or configuration",
        ),
        docs: None,
        details: fields.clone(),
    }
}

fn composer_diagnostic(diagnostic: &sc_composer::Diagnostic) -> Diagnostic {
    let mut details = Map::new();
    if let Some(path) = &diagnostic.path {
        details.insert("path".to_owned(), json!(to_forward_slash(path)));
    }
    if let Some(line) = diagnostic.line {
        details.insert("line".to_owned(), json!(line));
    }
    if let Some(column) = diagnostic.column {
        details.insert("column".to_owned(), json!(column));
    }
    if !diagnostic.include_chain.is_empty() {
        details.insert(
            "include_chain".to_owned(),
            json!(
                diagnostic
                    .include_chain
                    .iter()
                    .map(|path| to_forward_slash(path))
                    .collect::<Vec<_>>()
            ),
        );
    }
    event_diagnostic(diagnostic.code.as_str(), &diagnostic.message, &details)
}

impl Drop for CliObserver {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl CompositionObserver for CliObserver {
    fn on_resolve_attempt(&mut self, event: &ResolveAttemptEvent) {
        let mut fields = Map::new();
        fields.insert("template".to_owned(), json!(event.template));
        self.emit_pipeline_record(LogRecord {
            family: EventFamily::Pipeline,
            level: Level::Info,
            target: "compose.resolve",
            action: "attempt",
            message: "resolve attempt",
            outcome: None,
            diagnostic: None,
            fields,
        });
    }

    fn on_resolve_outcome(&mut self, event: &ResolveOutcomeEvent) {
        let action = if event.code.is_some() {
            "failed"
        } else {
            "resolved"
        };
        let mut fields = Map::new();
        fields.insert(
            "attempted_paths".to_owned(),
            json!(
                event
                    .attempted_paths
                    .iter()
                    .map(|path| to_forward_slash(path))
                    .collect::<Vec<_>>()
            ),
        );
        if let Some(path) = &event.resolved_path {
            fields.insert("resolved_path".to_owned(), json!(to_forward_slash(path)));
        }
        if let Some(code) = event.code {
            fields.insert("diagnostic_code".to_owned(), json!(code.as_str()));
        }
        self.emit_pipeline_record(LogRecord {
            family: EventFamily::Pipeline,
            level: if event.code.is_some() {
                Level::Error
            } else {
                Level::Info
            },
            target: "compose.resolve",
            action,
            message: if event.code.is_some() {
                "resolve failed"
            } else {
                "resolve completed"
            },
            outcome: Some(if event.code.is_some() {
                "failure"
            } else {
                "success"
            }),
            diagnostic: event
                .code
                .map(|code| event_diagnostic(code.as_str(), "resolve failed", &fields)),
            fields,
        });
    }

    fn on_include_outcome(&mut self, event: &IncludeOutcomeEvent) {
        let action = if event.code.is_some() {
            "failed"
        } else {
            "expanded"
        };
        let mut fields = Map::new();
        fields.insert(
            "resolved_files".to_owned(),
            json!(
                event
                    .resolved_files
                    .iter()
                    .map(|path| to_forward_slash(path))
                    .collect::<Vec<_>>()
            ),
        );
        fields.insert(
            "include_chain".to_owned(),
            json!(
                event
                    .include_chain
                    .iter()
                    .map(|path| to_forward_slash(path))
                    .collect::<Vec<_>>()
            ),
        );
        if let Some(code) = event.code {
            fields.insert("diagnostic_code".to_owned(), json!(code.as_str()));
        }
        self.emit_pipeline_record(LogRecord {
            family: EventFamily::Pipeline,
            level: if event.code.is_some() {
                Level::Error
            } else {
                Level::Info
            },
            target: "compose.include_expand",
            action,
            message: if event.code.is_some() {
                "include expansion failed"
            } else {
                "include expansion completed"
            },
            outcome: Some(if event.code.is_some() {
                "failure"
            } else {
                "success"
            }),
            diagnostic: event
                .code
                .map(|code| event_diagnostic(code.as_str(), "include expansion failed", &fields)),
            fields,
        });
    }

    fn on_validation_outcome(&mut self, event: &ValidationOutcomeEvent) {
        let failed = !event.errors.is_empty();
        let warnings = event.warnings.len();
        let errors = event.errors.len();
        let mut fields = Map::new();
        fields.insert("warning_count".to_owned(), json!(warnings));
        fields.insert("error_count".to_owned(), json!(errors));
        if let Some(diagnostic) = event.errors.first().or_else(|| event.warnings.first()) {
            fields.insert(
                "diagnostic_code".to_owned(),
                json!(diagnostic.code.as_str()),
            );
            fields.insert(
                "diagnostic_message".to_owned(),
                json!(diagnostic.message.clone()),
            );
        }
        self.emit_pipeline_record(LogRecord {
            family: EventFamily::Pipeline,
            level: if failed {
                Level::Error
            } else if warnings > 0 {
                Level::Warn
            } else {
                Level::Info
            },
            target: "compose.validate",
            action: if failed { "failed" } else { "completed" },
            message: if failed {
                "validation failed"
            } else if warnings > 0 {
                "validation completed with warnings"
            } else {
                "validation completed"
            },
            outcome: Some(if failed { "failure" } else { "success" }),
            diagnostic: event
                .errors
                .first()
                .or_else(|| event.warnings.first())
                .map(composer_diagnostic),
            fields,
        });
    }

    fn on_render_outcome(&mut self, event: &RenderOutcomeEvent) {
        let failed = event.code.is_some();
        let mut fields = Map::new();
        if let Some(rendered_bytes) = event.rendered_bytes {
            fields.insert("rendered_bytes".to_owned(), json!(rendered_bytes));
        }
        if let Some(code) = event.code {
            fields.insert("diagnostic_code".to_owned(), json!(code.as_str()));
        }
        self.emit_pipeline_record(LogRecord {
            family: EventFamily::Pipeline,
            level: if failed { Level::Error } else { Level::Info },
            target: "compose.render",
            action: if failed { "failed" } else { "completed" },
            message: if failed {
                "render failed"
            } else {
                "render completed"
            },
            outcome: Some(if failed { "failure" } else { "success" }),
            diagnostic: event
                .code
                .map(|code| event_diagnostic(code.as_str(), "render failed", &fields)),
            fields,
        });
    }
}

impl ObservationSink for CliObserver {
    /// Dispatch `ObservationEvent` values from `sc-composer` observer hooks.
    ///
    /// This method is unrelated to the deprecated `sc-observability::Logger::emit()`
    /// compatibility path; CLI logging writes happen through `Logger::log()`.
    fn emit(&mut self, event: &ObservationEvent) {
        match event {
            ObservationEvent::PassStart(event) => self.on_pass_start(event),
            ObservationEvent::PassEnd(event) => self.on_pass_end(event),
            ObservationEvent::VerifyStart(event) => self.on_verify_start(event),
            ObservationEvent::VerifyEnd(event) => self.on_verify_end(event),
            ObservationEvent::ResolveAttempt(event) => self.on_resolve_attempt(event),
            ObservationEvent::ResolveOutcome(event) => self.on_resolve_outcome(event),
            ObservationEvent::IncludeExpandOutcome(event) => self.on_include_outcome(event),
            ObservationEvent::ValidationOutcome(event) => self.on_validation_outcome(event),
            ObservationEvent::RenderOutcome(event) => self.on_render_outcome(event),
        }
    }
}

impl CommandLifecycleObserver for CliObserver {
    fn on_command_start(&mut self, event: &CommandStartEvent) {
        let mut fields = Map::new();
        fields.insert("command".to_owned(), json!(event.command_name));
        fields.insert("json_output".to_owned(), json!(event.json_output));
        self.emit_command_record(LogRecord {
            family: EventFamily::Command,
            level: Level::Info,
            target: "compose.command",
            action: "started",
            message: "command started",
            outcome: None,
            diagnostic: None,
            fields,
        });
    }

    fn on_command_end(&mut self, event: &CommandEndEvent) {
        let success = event.success;
        let mut fields = Map::new();
        fields.insert("command".to_owned(), json!(event.command_name));
        fields.insert("exit_code".to_owned(), json!(event.exit_code));
        fields.insert("elapsed_ms".to_owned(), json!(event.elapsed_ms));
        fields.insert("json_output".to_owned(), json!(event.json_output));
        if let Some(code) = &event.diagnostic_code {
            fields.insert("diagnostic_code".to_owned(), json!(code));
        }
        if let Some(message) = &event.diagnostic_message {
            fields.insert("diagnostic_message".to_owned(), json!(message));
        }
        self.emit_command_record(LogRecord {
            family: EventFamily::Command,
            level: if success { Level::Info } else { Level::Error },
            target: "compose.command",
            action: if success { "completed" } else { "failed" },
            message: if success {
                "command completed"
            } else {
                "command failed"
            },
            outcome: Some(if success { "success" } else { "failure" }),
            diagnostic: match (&event.diagnostic_code, &event.diagnostic_message) {
                (Some(code), Some(message)) => Some(event_diagnostic(code, message, &fields)),
                (Some(code), None) => Some(event_diagnostic(code, "command failed", &fields)),
                _ => None,
            },
            fields,
        });
    }
}

/// Return the static observation envelope schema version.
///
/// # Panics
///
/// Panics only if the crate-owned `OBSERVATION_ENVELOPE_VERSION` constant stops
/// satisfying `sc-observability` schema-version validation.
fn schema_version() -> SchemaVersion {
    match SchemaVersion::new(OBSERVATION_ENVELOPE_VERSION) {
        Ok(value) => value,
        Err(error) => {
            panic!(
                "invalid observation envelope schema version {OBSERVATION_ENVELOPE_VERSION:?}: {error}"
            )
        }
    }
}

/// Return the static service name used for CLI observations.
///
/// # Panics
///
/// Panics only if the crate-owned `SERVICE_NAME` constant stops satisfying
/// `sc-observability` service-name validation.
fn service_name() -> ServiceName {
    match ServiceName::new(SERVICE_NAME) {
        Ok(value) => value,
        Err(error) => panic!("invalid observability service name {SERVICE_NAME:?}: {error}"),
    }
}

/// Normalize a static event target into the validated observability newtype.
fn target_category(value: &str) -> Result<TargetCategory, ValueValidationError> {
    TargetCategory::new(value)
}

/// Normalize a static event action into the validated observability newtype.
fn action_name(value: &str) -> Result<ActionName, ValueValidationError> {
    ActionName::new(value)
}

/// Normalize a static event outcome into the validated observability newtype.
fn outcome_label(value: &str) -> Result<OutcomeLabel, ValueValidationError> {
    OutcomeLabel::new(value)
}

fn fallback_target_category() -> TargetCategory {
    match TargetCategory::new(FALLBACK_TARGET) {
        Ok(value) => value,
        Err(error) => panic!("invalid fallback target {FALLBACK_TARGET:?}: {error}"),
    }
}

fn fallback_action_name() -> ActionName {
    match ActionName::new(FALLBACK_ACTION) {
        Ok(value) => value,
        Err(error) => panic!("invalid fallback action {FALLBACK_ACTION:?}: {error}"),
    }
}

fn fallback_outcome_label() -> OutcomeLabel {
    match OutcomeLabel::new(FALLBACK_OUTCOME) {
        Ok(value) => value,
        Err(error) => panic!("invalid fallback outcome {FALLBACK_OUTCOME:?}: {error}"),
    }
}

fn normalize_event_labels(
    target: &str,
    action: &str,
    outcome: Option<&str>,
    fields: &mut Map<String, Value>,
) -> (TargetCategory, ActionName, Option<OutcomeLabel>) {
    let mut errors = Vec::new();

    let normalized_target = match target_category(target) {
        Ok(value) => value,
        Err(error) => {
            fields.insert("requested_target".to_owned(), json!(target));
            errors.push(format!("target {target:?}: {error}"));
            fallback_target_category()
        }
    };

    let normalized_action = match action_name(action) {
        Ok(value) => value,
        Err(error) => {
            fields.insert("requested_action".to_owned(), json!(action));
            errors.push(format!("action {action:?}: {error}"));
            fallback_action_name()
        }
    };

    let normalized_outcome = outcome.map(|value| match outcome_label(value) {
        Ok(label) => label,
        Err(error) => {
            fields.insert("requested_outcome".to_owned(), json!(value));
            errors.push(format!("outcome {value:?}: {error}"));
            fallback_outcome_label()
        }
    });

    if !errors.is_empty() {
        fields.insert("label_validation_errors".to_owned(), json!(errors));
    }

    (normalized_target, normalized_action, normalized_outcome)
}

fn unavailable_health_report(message: &str) -> LoggingHealthReport {
    let summary = DiagnosticSummary {
        code: None,
        message: message.to_owned(),
        at: Timestamp::now_utc(),
    };
    LoggingHealthReport {
        state: LoggingHealthState::Unavailable,
        dropped_events_total: 0,
        flush_errors_total: 0,
        active_log_path: PathBuf::from(".sc-compose")
            .join("logs")
            .join("sc-compose.log.jsonl"),
        sink_statuses: Vec::new(),
        queue_depth: 0,
        queue_capacity: 0,
        queue_high_water_mark: 0,
        queue_full_drops_total: 0,
        writer_state: WriterState::Stopped,
        last_writer_error: Some(summary.clone()),
        query: Some(QueryHealthReport {
            state: QueryHealthState::Unavailable,
            last_error: Some(summary.clone()),
        }),
        maintenance: None,
        last_error: Some(summary),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use sc_observability::{
        Level, LogEvent, LogSink, Logger, LoggerConfig, ProcessIdentity, SinkHealth,
        SinkHealthState, SinkRegistration, Timestamp, error_codes,
    };
    use sc_observability_types::{
        ErrorContext, LogSinkError, LoggingHealthState, QueryHealthState, Remediation, SinkName,
        WriterState,
    };
    use serde_json::Map;

    use super::{
        CliObserver, CommandEndEvent, CommandLifecycleObserver, CommandStartEvent, EventFamily,
        LogRecord, RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent,
        ValidationOutcomeEvent, action_name, normalize_event_labels, outcome_label, schema_version,
        service_name, target_category,
    };
    use sc_composer::{
        CompositionObserver, Diagnostic, DiagnosticCode, DiagnosticSeverity, IncludeOutcomeEvent,
    };

    #[test]
    fn cli_observer_emits_command_and_pipeline_events_to_logger() {
        let root = temp_root("observer-events");
        let mut config = LoggerConfig::default_for(service_name(), root.clone());
        config.enable_console_sink = false;
        let logger = match Logger::builder(config) {
            Ok(builder) => builder.build(),
            Err(error) => panic!("logger builder: {error}"),
        };
        if let Err(error) = logger.log(sample_log_event("preflight log")) {
            panic!("preflight log: {error}");
        }
        let mut observer = CliObserver::new(logger);

        observer.on_command_start(&CommandStartEvent {
            command_name: "render".to_owned(),
            json_output: false,
        });
        observer.on_resolve_attempt(&ResolveAttemptEvent {
            template: "agent:writer".to_owned(),
        });
        observer.on_resolve_outcome(&ResolveOutcomeEvent {
            resolved_path: Some(PathBuf::from("fixtures/template.md.j2")),
            attempted_paths: vec![PathBuf::from("fixtures/template.md.j2")],
            code: None,
        });
        observer.on_validation_outcome(&ValidationOutcomeEvent {
            warnings: vec![Diagnostic::new(
                DiagnosticSeverity::Warning,
                DiagnosticCode::ErrValExtraInput,
                "unused variable",
            )],
            errors: Vec::new(),
        });
        observer.on_render_outcome(&RenderOutcomeEvent {
            rendered_bytes: Some(42),
            code: None,
        });
        observer.on_command_end(&CommandEndEvent {
            command_name: "render".to_owned(),
            exit_code: 0,
            success: true,
            elapsed_ms: 12,
            json_output: false,
            diagnostic_code: None,
            diagnostic_message: None,
        });

        observer.shutdown();
        let lines = read_log_lines(&observer.health().active_log_path);
        assert_eq!(lines.len(), 7);
        assert_eq!(lines[0]["action"], "preflight");
        assert_eq!(lines[1]["target"], "compose.command");
        assert_eq!(lines[1]["action"], "started");
        assert_eq!(lines[1]["message"], "command started");
        assert_eq!(lines[2]["target"], "compose.resolve");
        assert_eq!(lines[2]["action"], "attempt");
        assert_eq!(lines[2]["message"], "resolve attempt");
        assert_eq!(lines[3]["target"], "compose.resolve");
        assert_eq!(lines[3]["action"], "resolved");
        assert_eq!(lines[4]["target"], "compose.validate");
        assert_eq!(lines[4]["action"], "completed");
        assert_eq!(lines[4]["level"], "Warn");
        assert_eq!(lines[5]["target"], "compose.render");
        assert_eq!(lines[5]["action"], "completed");
        assert_eq!(lines[6]["target"], "compose.command");
        assert_eq!(lines[6]["action"], "completed");
    }

    #[test]
    fn cli_observer_health_degrades_when_logger_state_is_missing() {
        let root = temp_root("observer-health-missing-state");
        let mut config = LoggerConfig::default_for(service_name(), root);
        config.enable_console_sink = false;
        let logger = match Logger::builder(config) {
            Ok(builder) => builder.build(),
            Err(error) => panic!("logger builder: {error}"),
        };
        let mut observer = CliObserver::new(logger);
        observer.logger = None;

        let health = observer.health();

        assert_eq!(health.state, LoggingHealthState::Unavailable);
        assert_eq!(health.writer_state, WriterState::Stopped);
        assert_eq!(
            match health.query {
                Some(query) => query.state,
                None => panic!("query health present"),
            },
            QueryHealthState::Unavailable
        );
        assert!(health.last_error.is_some());
    }

    #[test]
    fn cli_observer_shutdown_preserves_stopped_health_and_flush_durability() {
        let root = temp_root("observer-shutdown-durability");
        let mut config = LoggerConfig::default_for(service_name(), root.clone());
        config.enable_console_sink = false;
        let logger = match Logger::builder(config) {
            Ok(builder) => builder.build(),
            Err(error) => panic!("logger builder: {error}"),
        };
        let mut observer = CliObserver::new(logger);

        observer.on_command_start(&CommandStartEvent {
            command_name: "reports-smoke".to_owned(),
            json_output: false,
        });
        observer.shutdown();

        let health = observer.health();
        assert!(health.active_log_path.exists());
        assert!(health.last_writer_error.is_none());
        let lines = read_log_lines(&health.active_log_path);
        assert!(!lines.is_empty());
        assert_eq!(lines[0]["action"], "started");
    }

    #[test]
    fn command_end_failure_records_failure_fields() {
        let root = temp_root("observer-command-failure");
        let mut config = LoggerConfig::default_for(service_name(), root);
        config.enable_console_sink = false;
        let logger = match Logger::builder(config) {
            Ok(builder) => builder.build(),
            Err(error) => panic!("logger builder: {error}"),
        };
        if let Err(error) = logger.try_log(sample_log_event("preflight try-log")) {
            panic!("preflight try-log: {error}");
        }
        let mut observer = CliObserver::new(logger);

        observer.on_command_end(&CommandEndEvent {
            command_name: "validate".to_owned(),
            exit_code: 2,
            success: false,
            elapsed_ms: 7,
            json_output: true,
            diagnostic_code: Some("ERR_VAL".to_owned()),
            diagnostic_message: Some("validation failed".to_owned()),
        });

        observer.shutdown();
        let lines = read_log_lines(&observer.health().active_log_path);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["action"], "preflight");
        assert_eq!(lines[1]["action"], "failed");
        assert_eq!(lines[1]["message"], "command failed");
        assert_eq!(lines[1]["fields"]["exit_code"], 2);
        assert_eq!(lines[1]["fields"]["json_output"], true);
        assert_eq!(lines[1]["fields"]["diagnostic_code"], "ERR_VAL");
    }

    #[test]
    fn include_outcome_failure_records_failure_fields() {
        let root = temp_root("observer-include-failure");
        let mut config = LoggerConfig::default_for(service_name(), root);
        config.enable_console_sink = false;
        let logger = match Logger::builder(config) {
            Ok(builder) => builder.build(),
            Err(error) => panic!("logger builder: {error}"),
        };
        let mut observer = CliObserver::new(logger);

        observer.on_include_outcome(&IncludeOutcomeEvent {
            resolved_files: vec![PathBuf::from("partials/header.md.j2")],
            include_chain: vec![
                PathBuf::from("template.md.j2"),
                PathBuf::from("partials/header.md.j2"),
            ],
            code: Some(DiagnosticCode::ErrIncludeNotFound),
        });

        observer.shutdown();
        let lines = read_log_lines(&observer.health().active_log_path);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["target"], "compose.include_expand");
        assert_eq!(lines[0]["action"], "failed");
        assert_eq!(lines[0]["outcome"], "failure");
        assert_eq!(
            lines[0]["fields"]["diagnostic_code"],
            "ERR_INCLUDE_NOT_FOUND"
        );
        assert_eq!(
            lines[0]["fields"]["resolved_files"][0],
            "partials/header.md.j2"
        );
        assert_eq!(lines[0]["fields"]["include_chain"][0], "template.md.j2");
    }

    #[test]
    fn invalid_event_labels_degrade_to_fallback_log_values() {
        let root = temp_root("observer-invalid-labels");
        let mut config = LoggerConfig::default_for(service_name(), root.clone());
        config.enable_console_sink = false;
        let logger = match Logger::builder(config) {
            Ok(builder) => builder.build(),
            Err(error) => panic!("logger builder: {error}"),
        };
        let mut observer = CliObserver::new(logger);

        observer.emit_record(LogRecord {
            family: EventFamily::Pipeline,
            level: Level::Info,
            target: "compose/invalid",
            action: "bad action",
            message: "invalid labels",
            outcome: Some("bad outcome"),
            diagnostic: None,
            fields: Map::new(),
        });

        observer.shutdown();
        let lines = read_log_lines(&observer.health().active_log_path);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["target"], "compose.observability");
        assert_eq!(lines[0]["action"], "degraded");
        assert_eq!(lines[0]["outcome"], "failure");
        assert_eq!(lines[0]["fields"]["requested_target"], "compose/invalid");
        assert_eq!(lines[0]["fields"]["requested_action"], "bad action");
        assert_eq!(lines[0]["fields"]["requested_outcome"], "bad outcome");
        assert_eq!(
            lines[0]["fields"]["label_validation_errors"]
                .as_array()
                .map(Vec::len),
            Some(3)
        );
    }

    #[test]
    fn normalized_event_label_matrix_preserves_pipeline_and_command_contracts() {
        let cases = [
            ("compose.resolve", "attempt", None),
            ("compose.resolve", "resolved", Some("success")),
            ("compose.resolve", "failed", Some("failure")),
            ("compose.include_expand", "expanded", Some("success")),
            ("compose.include_expand", "failed", Some("failure")),
            ("compose.validate", "completed", Some("success")),
            ("compose.validate", "failed", Some("failure")),
            ("compose.render", "completed", Some("success")),
            ("compose.render", "failed", Some("failure")),
            ("compose.command", "started", None),
            ("compose.command", "completed", Some("success")),
            ("compose.command", "failed", Some("failure")),
        ];

        for (target_text, action_text, outcome_text) in cases {
            let mut fields = Map::new();
            let (target, action, outcome) =
                normalize_event_labels(target_text, action_text, outcome_text, &mut fields);
            let normalized_outcome = outcome.map(|value| value.to_string());

            assert_eq!(target.to_string(), target_text);
            assert_eq!(action.to_string(), action_text);
            assert_eq!(normalized_outcome.as_deref(), outcome_text);
            assert!(
                fields.is_empty(),
                "valid labels should not add fallback fields"
            );
        }
    }

    #[test]
    fn observer_callbacks_emit_failure_diagnostics_and_event_families() {
        let root = temp_root("observer-callback-diagnostics");
        let mut config = LoggerConfig::default_for(service_name(), root);
        config.enable_console_sink = false;
        let logger = match Logger::builder(config) {
            Ok(builder) => builder.build(),
            Err(error) => panic!("logger builder: {error}"),
        };
        let mut observer = CliObserver::new(logger);

        observer.on_resolve_outcome(&ResolveOutcomeEvent {
            resolved_path: None,
            attempted_paths: vec![PathBuf::from("missing.md.j2")],
            code: Some(DiagnosticCode::ErrResolveNotFound),
        });
        observer.on_include_outcome(&IncludeOutcomeEvent {
            resolved_files: vec![PathBuf::from("partials/header.md.j2")],
            include_chain: vec![PathBuf::from("template.md.j2")],
            code: None,
        });
        observer.on_validation_outcome(&ValidationOutcomeEvent {
            warnings: Vec::new(),
            errors: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrValMissingRequired,
                "name is required",
            )],
        });
        observer.on_render_outcome(&RenderOutcomeEvent {
            rendered_bytes: None,
            code: Some(DiagnosticCode::ErrRenderWrite),
        });

        observer.shutdown();
        let lines = read_log_lines(&observer.health().active_log_path);
        assert_eq!(lines.len(), 4);

        assert_eq!(lines[0]["fields"]["event_family"], "pipeline");
        assert_eq!(lines[0]["action"], "failed");
        assert_eq!(lines[0]["diagnostic"]["code"], "ERR_RESOLVE_NOT_FOUND");
        assert_eq!(lines[0]["diagnostic"]["message"], "resolve failed");

        assert_eq!(lines[1]["fields"]["event_family"], "pipeline");
        assert_eq!(lines[1]["action"], "expanded");
        assert!(lines[1]["diagnostic"].is_null());

        assert_eq!(lines[2]["fields"]["event_family"], "pipeline");
        assert_eq!(lines[2]["action"], "failed");
        assert_eq!(lines[2]["diagnostic"]["code"], "ERR_VAL_MISSING_REQUIRED");
        assert_eq!(lines[2]["diagnostic"]["message"], "name is required");

        assert_eq!(lines[3]["fields"]["event_family"], "pipeline");
        assert_eq!(lines[3]["action"], "failed");
        assert_eq!(lines[3]["diagnostic"]["code"], "ERR_RENDER_WRITE");
    }

    #[test]
    fn write_failures_degrade_health_without_breaking_observer_calls() {
        struct WriteFailSink;

        impl LogSink for WriteFailSink {
            fn write(&self, _event: &sc_observability::LogEvent) -> Result<(), LogSinkError> {
                Err(LogSinkError(Box::new(ErrorContext::new(
                    error_codes::LOGGER_SINK_WRITE_FAILED,
                    "test sink write failed",
                    Remediation::not_recoverable("test sink intentionally fails writes"),
                ))))
            }

            fn health(&self) -> SinkHealth {
                SinkHealth {
                    name: match SinkName::new("write-fail") {
                        Ok(name) => name,
                        Err(error) => panic!("valid sink name: {error}"),
                    },
                    state: SinkHealthState::DegradedDropping,
                    last_error: None,
                }
            }
        }

        let root = temp_root("observer-write-failure");
        let mut config = LoggerConfig::default_for(service_name(), root);
        config.enable_file_sink = false;
        let mut builder = match Logger::builder(config) {
            Ok(builder) => builder,
            Err(error) => panic!("logger builder: {error}"),
        };
        builder.register_sink(SinkRegistration::new(Arc::new(WriteFailSink)));
        let logger = builder.build();
        let mut observer = CliObserver::new(logger);

        observer.on_command_start(&CommandStartEvent {
            command_name: "render".to_owned(),
            json_output: true,
        });

        observer.shutdown();
        let health = observer.health();
        assert_eq!(health.dropped_events_total, 1);
        assert!(health.last_error.is_some());
    }

    #[test]
    fn shutdown_flush_failures_are_counted_and_mark_query_unavailable() {
        struct FlushFailSink;

        impl LogSink for FlushFailSink {
            fn write(&self, _event: &sc_observability::LogEvent) -> Result<(), LogSinkError> {
                Ok(())
            }

            fn flush(&self) -> Result<(), LogSinkError> {
                Err(LogSinkError(Box::new(ErrorContext::new(
                    error_codes::LOGGER_FLUSH_FAILED,
                    "test sink flush failed",
                    Remediation::not_recoverable("test sink intentionally fails flush"),
                ))))
            }

            fn health(&self) -> SinkHealth {
                SinkHealth {
                    name: match SinkName::new("flush-fail") {
                        Ok(name) => name,
                        Err(error) => panic!("valid sink name: {error}"),
                    },
                    state: SinkHealthState::DegradedDropping,
                    last_error: None,
                }
            }
        }

        let root = temp_root("observer-shutdown-failure");
        let mut config = LoggerConfig::default_for(service_name(), root);
        config.enable_file_sink = false;
        let mut builder = match Logger::builder(config) {
            Ok(builder) => builder,
            Err(error) => panic!("logger builder: {error}"),
        };
        builder.register_sink(SinkRegistration::new(Arc::new(FlushFailSink)));
        let mut observer = CliObserver::new(builder.build());

        observer.shutdown();

        let health = observer.health();
        assert!(health.last_error.is_some());
        assert!(health.last_writer_error.is_some());
        assert_eq!(
            match health.query {
                Some(query) => query.state,
                None => panic!("query health present"),
            },
            QueryHealthState::Unavailable
        );
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_nanos(),
            Err(error) => panic!("time: {error}"),
        };
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        if let Err(error) = fs::create_dir_all(&root) {
            panic!("create temp root: {error}");
        }
        root
    }

    fn read_log_lines(path: &Path) -> Vec<serde_json::Value> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => panic!("read log file: {error}"),
        };
        contents
            .lines()
            .map(|line| match serde_json::from_str(line) {
                Ok(value) => value,
                Err(error) => panic!("parse log line: {error}"),
            })
            .collect()
    }

    fn sample_log_event(message: &str) -> LogEvent {
        LogEvent {
            version: schema_version(),
            timestamp: Timestamp::now_utc(),
            level: Level::Info,
            service: service_name(),
            target: match target_category("compose.command") {
                Ok(value) => value,
                Err(error) => panic!("valid target: {error}"),
            },
            action: match action_name("preflight") {
                Ok(value) => value,
                Err(error) => panic!("valid action: {error}"),
            },
            message: Some(message.to_owned()),
            identity: ProcessIdentity::default(),
            trace: None,
            request_id: None,
            correlation_id: None,
            outcome: Some(match outcome_label("success") {
                Ok(value) => value,
                Err(error) => panic!("valid outcome: {error}"),
            }),
            diagnostic: None,
            state_transition: None,
            fields: Map::new(),
        }
    }
}
