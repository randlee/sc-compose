use serde_json::{Map, Value, json};

use sc_composer::{
    CompositionObserver, IncludeOutcomeEvent, ObservationEvent, ObservationSink,
    RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
};
use sc_observability::{
    ActionName, Level, LogEvent, Logger, LoggingHealthReport, OBSERVATION_ENVELOPE_VERSION,
    OutcomeLabel, ProcessIdentity, SchemaVersion, ServiceName, TargetCategory, Timestamp,
};

const SERVICE_NAME: &str = "sc-compose";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandStartEvent {
    pub command_name: String,
    pub json_output: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEndEvent {
    pub command_name: String,
    pub exit_code: i32,
    pub elapsed_ms: u64,
    pub json_output: bool,
    pub diagnostic_code: Option<String>,
    pub diagnostic_message: Option<String>,
}

pub(crate) trait CommandLifecycleObserver {
    fn on_command_start(&mut self, event: &CommandStartEvent);
    fn on_command_end(&mut self, event: &CommandEndEvent);
}

pub struct CliObserver {
    logger: Logger,
    service: ServiceName,
}

impl CliObserver {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            service: service_name(),
        }
    }

    pub fn health(&self) -> LoggingHealthReport {
        self.logger.health()
    }

    pub fn shutdown(&self) {
        let _ignored = self.logger.shutdown();
    }

    fn emit_log(
        &self,
        level: Level,
        target: &str,
        action: &str,
        message: impl Into<String>,
        outcome: Option<&str>,
        fields: Map<String, Value>,
    ) {
        let event = LogEvent {
            version: schema_version(),
            timestamp: Timestamp::now_utc(),
            level,
            service: self.service.clone(),
            target: target_category(target),
            action: action_name(action),
            message: Some(message.into()),
            identity: ProcessIdentity::default(),
            trace: None,
            request_id: None,
            correlation_id: None,
            outcome: outcome.map(outcome_label),
            diagnostic: None,
            state_transition: None,
            fields,
        };

        let _ignored = self.logger.emit(event);
    }
}

impl CompositionObserver for CliObserver {
    fn on_resolve_attempt(&mut self, event: &ResolveAttemptEvent) {
        let mut fields = Map::new();
        fields.insert("template".to_owned(), json!(event.template));
        self.emit_log(
            Level::Info,
            "compose.resolve",
            "attempt",
            "resolve attempt",
            None,
            fields,
        );
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
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
            ),
        );
        if let Some(path) = &event.resolved_path {
            fields.insert(
                "resolved_path".to_owned(),
                json!(path.display().to_string()),
            );
        }
        if let Some(code) = event.code {
            fields.insert("diagnostic_code".to_owned(), json!(code.as_str()));
        }
        self.emit_log(
            if event.code.is_some() {
                Level::Error
            } else {
                Level::Info
            },
            "compose.resolve",
            action,
            if event.code.is_some() {
                "resolve failed"
            } else {
                "resolve completed"
            },
            Some(if event.code.is_some() {
                "failure"
            } else {
                "success"
            }),
            fields,
        );
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
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
            ),
        );
        fields.insert(
            "include_chain".to_owned(),
            json!(
                event
                    .include_chain
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
            ),
        );
        if let Some(code) = event.code {
            fields.insert("diagnostic_code".to_owned(), json!(code.as_str()));
        }
        self.emit_log(
            if event.code.is_some() {
                Level::Error
            } else {
                Level::Info
            },
            "compose.include_expand",
            action,
            if event.code.is_some() {
                "include expansion failed"
            } else {
                "include expansion completed"
            },
            Some(if event.code.is_some() {
                "failure"
            } else {
                "success"
            }),
            fields,
        );
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
        self.emit_log(
            if failed {
                Level::Error
            } else if warnings > 0 {
                Level::Warn
            } else {
                Level::Info
            },
            "compose.validate",
            if failed { "failed" } else { "completed" },
            if failed {
                "validation failed"
            } else if warnings > 0 {
                "validation completed with warnings"
            } else {
                "validation completed"
            },
            Some(if failed { "failure" } else { "success" }),
            fields,
        );
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
        self.emit_log(
            if failed { Level::Error } else { Level::Info },
            "compose.render",
            if failed { "failed" } else { "completed" },
            if failed {
                "render failed"
            } else {
                "render completed"
            },
            Some(if failed { "failure" } else { "success" }),
            fields,
        );
    }
}

impl ObservationSink for CliObserver {
    fn emit(&mut self, event: &ObservationEvent) {
        match event {
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
        self.emit_log(
            Level::Info,
            "compose.command",
            "started",
            "command started",
            None,
            fields,
        );
    }

    fn on_command_end(&mut self, event: &CommandEndEvent) {
        let success = event.exit_code == 0;
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
        self.emit_log(
            if success { Level::Info } else { Level::Error },
            "compose.command",
            if success { "completed" } else { "failed" },
            if success {
                "command completed"
            } else {
                "command failed"
            },
            Some(if success { "success" } else { "failure" }),
            fields,
        );
    }
}

fn schema_version() -> SchemaVersion {
    SchemaVersion::new(OBSERVATION_ENVELOPE_VERSION).expect("schema version constant is valid")
}

fn service_name() -> ServiceName {
    ServiceName::new(SERVICE_NAME).expect("service name constant is valid")
}

fn target_category(value: &str) -> TargetCategory {
    TargetCategory::new(value)
        .unwrap_or_else(|error| panic!("invalid sc-observability target {value:?}: {error}"))
}

fn action_name(value: &str) -> ActionName {
    ActionName::new(value)
        .unwrap_or_else(|error| panic!("invalid sc-observability action {value:?}: {error}"))
}

fn outcome_label(value: &str) -> OutcomeLabel {
    OutcomeLabel::new(value)
        .unwrap_or_else(|error| panic!("invalid sc-observability outcome {value:?}: {error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use sc_observability::{Logger, LoggerConfig};

    use super::{
        CliObserver, CommandEndEvent, CommandLifecycleObserver, CommandStartEvent,
        RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
        service_name,
    };
    use sc_composer::{CompositionObserver, Diagnostic, DiagnosticCode, DiagnosticSeverity};

    #[test]
    fn cli_observer_emits_command_and_pipeline_events_to_logger() {
        let root = temp_root("observer-events");
        let mut config = LoggerConfig::default_for(service_name(), root.clone());
        config.enable_console_sink = false;
        let logger = Logger::new(config).expect("logger");
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
            elapsed_ms: 12,
            json_output: false,
            diagnostic_code: None,
            diagnostic_message: None,
        });

        let lines = read_log_lines(&observer.health().active_log_path);
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0]["target"], "compose.command");
        assert_eq!(lines[0]["action"], "started");
        assert_eq!(lines[1]["target"], "compose.resolve");
        assert_eq!(lines[1]["action"], "attempt");
        assert_eq!(lines[2]["target"], "compose.resolve");
        assert_eq!(lines[2]["action"], "resolved");
        assert_eq!(lines[3]["target"], "compose.validate");
        assert_eq!(lines[3]["action"], "completed");
        assert_eq!(lines[3]["level"], "Warn");
        assert_eq!(lines[4]["target"], "compose.render");
        assert_eq!(lines[4]["action"], "completed");
        assert_eq!(lines[5]["target"], "compose.command");
        assert_eq!(lines[5]["action"], "completed");
    }

    #[test]
    fn command_end_failure_records_failure_fields() {
        let root = temp_root("observer-command-failure");
        let mut config = LoggerConfig::default_for(service_name(), root);
        config.enable_console_sink = false;
        let logger = Logger::new(config).expect("logger");
        let mut observer = CliObserver::new(logger);

        observer.on_command_end(&CommandEndEvent {
            command_name: "validate".to_owned(),
            exit_code: 2,
            elapsed_ms: 7,
            json_output: true,
            diagnostic_code: Some("ERR_VAL".to_owned()),
            diagnostic_message: Some("validation failed".to_owned()),
        });

        let lines = read_log_lines(&observer.health().active_log_path);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["action"], "failed");
        assert_eq!(lines[0]["fields"]["exit_code"], 2);
        assert_eq!(lines[0]["fields"]["json_output"], true);
        assert_eq!(lines[0]["fields"]["diagnostic_code"], "ERR_VAL");
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn read_log_lines(path: &Path) -> Vec<serde_json::Value> {
        fs::read_to_string(path)
            .expect("read log file")
            .lines()
            .map(|line| serde_json::from_str(line).expect("parse log line"))
            .collect()
    }
}
