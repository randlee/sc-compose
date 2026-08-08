use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;

use super::display::{BoxedError, write_error_display};
use super::recovery::RecoveryHint;
use crate::diagnostics::DiagnosticCode;

/// Canonical configuration and parsing error family.
#[derive(Debug)]
pub struct ConfigError {
    code: DiagnosticCode,
    message: String,
    recovery_hints: Vec<RecoveryHint>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ConfigError {
    /// Create a new configuration error.
    #[must_use]
    pub(crate) fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            recovery_hints: Vec::new(),
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Attach an underlying source error.
    #[must_use]
    pub(crate) fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
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

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for ConfigError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
    }
}
