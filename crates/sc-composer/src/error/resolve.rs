use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;

use super::display::{BoxedError, write_error_display};
use crate::diagnostics::DiagnosticCode;

/// Canonical resolver error family.
#[derive(Debug)]
pub struct ResolveError {
    code: DiagnosticCode,
    message: String,
    attempted_paths: Vec<PathBuf>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ResolveError {
    /// Create a new resolver error without an underlying source.
    #[must_use]
    pub(crate) fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        attempted_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            attempted_paths,
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

    /// Return the stable diagnostic code when one is available.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Return the attempted paths recorded for this failure.
    #[must_use]
    pub fn attempted_paths(&self) -> &[PathBuf] {
        &self.attempted_paths
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

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for ResolveError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
    }
}
