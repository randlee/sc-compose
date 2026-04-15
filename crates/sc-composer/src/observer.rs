//! Observer traits and event types for composition telemetry hooks.

use std::path::PathBuf;

use serde::Serialize;

use crate::{Diagnostic, DiagnosticCode};

/// Structured event emitted through an [`ObservationSink`].
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum ObservationEvent {
    /// Resolver attempt notification.
    ResolveAttempt(ResolveAttemptEvent),
    /// Resolver outcome notification.
    ResolveOutcome(ResolveOutcomeEvent),
    /// Include expansion outcome notification.
    IncludeExpandOutcome(IncludeOutcomeEvent),
    /// Validation outcome notification.
    ValidationOutcome(ValidationOutcomeEvent),
    /// Render outcome notification.
    RenderOutcome(RenderOutcomeEvent),
}

/// Sink trait for consumers that want a single event-stream interface.
pub trait ObservationSink {
    /// Emit one structured observation event.
    fn emit(&mut self, event: &ObservationEvent);
}

/// Open observer trait used by embedded hosts and the CLI.
pub trait CompositionObserver {
    /// Called when resolution starts.
    fn on_resolve_attempt(&mut self, _event: &ResolveAttemptEvent) {}

    /// Called when resolution completes or fails.
    fn on_resolve_outcome(&mut self, _event: &ResolveOutcomeEvent) {}

    /// Called when include expansion completes or fails.
    fn on_include_outcome(&mut self, _event: &IncludeOutcomeEvent) {}

    /// Called when validation completes.
    fn on_validation_outcome(&mut self, _event: &ValidationOutcomeEvent) {}

    /// Called when rendering completes or fails.
    fn on_render_outcome(&mut self, _event: &RenderOutcomeEvent) {}
}

/// Default no-op observer used when no host implementation is injected.
#[derive(Debug, Default)]
pub struct NoopObserver;

impl CompositionObserver for NoopObserver {}

impl ObservationSink for NoopObserver {
    fn emit(&mut self, _event: &ObservationEvent) {}
}

/// Event emitted when resolution starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolveAttemptEvent {
    /// Template or profile identifier targeted for resolution.
    pub template: String,
}

/// Event emitted after resolution succeeds or fails.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolveOutcomeEvent {
    /// Final resolved path when resolution succeeds.
    pub resolved_path: Option<PathBuf>,
    /// Attempted candidate paths.
    pub attempted_paths: Vec<PathBuf>,
    /// Stable diagnostic code when resolution fails.
    pub code: Option<DiagnosticCode>,
}

/// Event emitted after include expansion succeeds or fails.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncludeOutcomeEvent {
    /// Files resolved through include expansion when successful.
    pub resolved_files: Vec<PathBuf>,
    /// Include chain captured for failures.
    pub include_chain: Vec<PathBuf>,
    /// Stable diagnostic code when expansion fails.
    pub code: Option<DiagnosticCode>,
}

/// Event emitted after validation finishes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ValidationOutcomeEvent {
    /// Non-fatal diagnostics emitted during validation.
    pub warnings: Vec<Diagnostic>,
    /// Fatal diagnostics emitted during validation.
    pub errors: Vec<Diagnostic>,
}

/// Event emitted after rendering succeeds or fails.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RenderOutcomeEvent {
    /// Byte length of the rendered output when rendering succeeds.
    pub rendered_bytes: Option<usize>,
    /// Stable diagnostic code when rendering fails.
    pub code: Option<DiagnosticCode>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, compose};

    use super::{
        CompositionObserver, IncludeOutcomeEvent, NoopObserver, ObservationEvent, ObservationSink,
        RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
    };

    #[test]
    fn compose_without_observer_remains_fully_functional() {
        let root = temp_root("compose-noop-observer");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
        );

        let result = compose(&ComposeRequest {
            runtime: None,
            root: ConfiningRoot::new(&root).expect("root"),
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        })
        .expect("compose succeeds without an injected observer");

        assert_eq!(result.rendered_text, "hello world");
    }

    #[test]
    fn noop_observer_implements_sink_and_observer_traits() {
        fn assert_traits<T: CompositionObserver + ObservationSink>() {}

        assert_traits::<NoopObserver>();
    }

    #[test]
    fn noop_observer_accepts_callbacks_without_side_effects() {
        let mut observer = NoopObserver;
        let resolve_attempt = ResolveAttemptEvent {
            template: "template.md.j2".to_owned(),
        };
        let resolve_outcome = ResolveOutcomeEvent {
            resolved_path: Some(PathBuf::from("template.md.j2")),
            attempted_paths: vec![PathBuf::from("template.md.j2")],
            code: None,
        };
        let include_outcome = IncludeOutcomeEvent {
            resolved_files: vec![PathBuf::from("template.md.j2")],
            include_chain: Vec::new(),
            code: None,
        };
        let validation_outcome = ValidationOutcomeEvent {
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let render_outcome = RenderOutcomeEvent {
            rendered_bytes: Some(11),
            code: None,
        };

        observer.on_resolve_attempt(&resolve_attempt);
        observer.on_resolve_outcome(&resolve_outcome);
        observer.on_include_outcome(&include_outcome);
        observer.on_validation_outcome(&validation_outcome);
        observer.on_render_outcome(&render_outcome);
        observer.emit(&ObservationEvent::ResolveAttempt(resolve_attempt));
        observer.emit(&ObservationEvent::ResolveOutcome(resolve_outcome));
        observer.emit(&ObservationEvent::IncludeExpandOutcome(include_outcome));
        observer.emit(&ObservationEvent::ValidationOutcome(validation_outcome));
        observer.emit(&ObservationEvent::RenderOutcome(render_outcome));
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sc-composer-observer-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, contents).expect("write fixture");
    }
}
