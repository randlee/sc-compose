use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use sc_composer::{
    CompositionObserver, IncludeOutcomeEvent, ObservationEvent, ObservationSink,
    RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandStartEvent {
    pub command_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandEndEvent {
    pub command_name: String,
    pub success: bool,
}

pub trait CommandLifecycleObserver {
    fn on_command_start(&mut self, event: &CommandStartEvent);
    fn on_command_end(&mut self, event: &CommandEndEvent);
}

pub struct CliObserver {
    sink: SinkMode,
}

impl CliObserver {
    pub fn from_env() -> Self {
        if let Ok(path) = std::env::var("SC_COMPOSE_OBSERVER_LOG") {
            return Self {
                sink: SinkMode::File(PathBuf::from(path)),
            };
        }
        if std::env::var("SC_COMPOSE_OBSERVER_STDERR")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        {
            return Self {
                sink: SinkMode::Stderr,
            };
        }
        Self {
            sink: SinkMode::Disabled,
        }
    }

    fn emit_record<T: Serialize>(&mut self, event: &T) {
        let Ok(line) = serde_json::to_string(event) else {
            return;
        };
        match &self.sink {
            SinkMode::Disabled => {}
            SinkMode::Stderr => {
                let _ignored = writeln!(std::io::stderr(), "{line}");
            }
            SinkMode::File(path) => {
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                    let _ignored = writeln!(file, "{line}");
                }
            }
        }
    }
}

impl CompositionObserver for CliObserver {
    fn on_resolve_attempt(&mut self, event: &ResolveAttemptEvent) {
        ObservationSink::emit(self, &ObservationEvent::ResolveAttempt(event.clone()));
    }

    fn on_resolve_outcome(&mut self, event: &ResolveOutcomeEvent) {
        ObservationSink::emit(self, &ObservationEvent::ResolveOutcome(event.clone()));
    }

    fn on_include_outcome(&mut self, event: &IncludeOutcomeEvent) {
        ObservationSink::emit(self, &ObservationEvent::IncludeExpandOutcome(event.clone()));
    }

    fn on_validation_outcome(&mut self, event: &ValidationOutcomeEvent) {
        ObservationSink::emit(self, &ObservationEvent::ValidationOutcome(event.clone()));
    }

    fn on_render_outcome(&mut self, event: &RenderOutcomeEvent) {
        ObservationSink::emit(self, &ObservationEvent::RenderOutcome(event.clone()));
    }
}

impl ObservationSink for CliObserver {
    fn emit(&mut self, event: &ObservationEvent) {
        self.emit_record(event);
    }
}

impl CommandLifecycleObserver for CliObserver {
    fn on_command_start(&mut self, event: &CommandStartEvent) {
        self.emit_record(event);
    }

    fn on_command_end(&mut self, event: &CommandEndEvent) {
        self.emit_record(event);
    }
}

enum SinkMode {
    Disabled,
    Stderr,
    File(PathBuf),
}
