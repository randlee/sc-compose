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
