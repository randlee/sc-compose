use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use sc_composer::{
    CommandEndEvent, CommandStartEvent, CompositionObserver, IncludeOutcomeEvent, ObservationEvent,
    RenderOutcomeEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
};

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

    fn emit(&mut self, event: &ObservationEvent) {
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
    fn on_command_start(&mut self, event: &CommandStartEvent) {
        self.emit(&ObservationEvent::CommandStart(event.clone()));
    }

    fn on_command_end(&mut self, event: &CommandEndEvent) {
        self.emit(&ObservationEvent::CommandEnd(event.clone()));
    }

    fn on_resolve_outcome(&mut self, event: &ResolveOutcomeEvent) {
        self.emit(&ObservationEvent::ResolveOutcome(event.clone()));
    }

    fn on_include_outcome(&mut self, event: &IncludeOutcomeEvent) {
        self.emit(&ObservationEvent::IncludeOutcome(event.clone()));
    }

    fn on_validation_outcome(&mut self, event: &ValidationOutcomeEvent) {
        self.emit(&ObservationEvent::ValidationOutcome(event.clone()));
    }

    fn on_render_outcome(&mut self, event: &RenderOutcomeEvent) {
        self.emit(&ObservationEvent::RenderOutcome(event.clone()));
    }
}

enum SinkMode {
    Disabled,
    Stderr,
    File(PathBuf),
}
