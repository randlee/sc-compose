use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;

use super::display::{BoxedError, format_diagnostic_message, write_error_display};
use super::recovery::{RecoveryHint, RecoveryHintKind};
use crate::Diagnostic;
use crate::diagnostics::DiagnosticCode;
use crate::types::VariableName;

/// Canonical validation error family.
#[derive(Debug)]
pub struct ValidationError {
    code: DiagnosticCode,
    message: String,
    diagnostics: Vec<Diagnostic>,
    recovery_hints: Vec<RecoveryHint>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ValidationError {
    /// Create a new validation error.
    #[must_use]
    pub(crate) fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            diagnostics: Vec::new(),
            recovery_hints: Vec::new(),
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Create a validation error from a full diagnostics set.
    #[must_use]
    pub(crate) fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        let code = diagnostics
            .first()
            .map_or(DiagnosticCode::ErrValEmpty, |diagnostic| diagnostic.code);
        let message = diagnostics
            .iter()
            .map(format_diagnostic_message)
            .collect::<Vec<_>>()
            .join("\n");
        let mut error = Self {
            code,
            message,
            diagnostics,
            recovery_hints: Vec::new(),
            source: None,
            backtrace: Backtrace::capture(),
        };
        if let Some(path) = error
            .diagnostics
            .iter()
            .find_map(|diagnostic| diagnostic.path.clone())
        {
            error =
                error.with_recovery_hint(RecoveryHint::new(RecoveryHintKind::InspectPath { path }));
        }
        error
    }

    /// Create a duplicate-frontmatter validation error.
    #[must_use]
    pub(crate) fn duplicate_variable(variable: &VariableName) -> Self {
        Self::new(
            DiagnosticCode::ErrValDuplicate,
            format!("duplicate frontmatter variable declaration: {variable}"),
        )
        .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "required_variables".to_owned(),
        }))
    }

    /// Create a validation error for an invalid input-value shape.
    #[must_use]
    pub(crate) fn invalid_input_value(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self::new(code, message)
    }

    /// Attach a structured recovery hint.
    #[must_use]
    pub(crate) fn with_recovery_hint(mut self, recovery_hint: RecoveryHint) -> Self {
        self.recovery_hints.push(recovery_hint);
        self
    }

    /// Return the stable diagnostic code when one is available.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Return structured recovery hints.
    #[must_use]
    pub fn recovery_hints(&self) -> &[RecoveryHint] {
        &self.recovery_hints
    }

    /// Return the diagnostics preserved for this validation failure.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Return the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for ValidationError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
    }
}
