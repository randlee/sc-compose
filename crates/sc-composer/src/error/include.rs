use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;

use super::display::{BoxedError, write_error_display};
use crate::diagnostics::DiagnosticCode;

/// Canonical include-processing error family.
#[derive(Debug)]
pub struct IncludeError {
    code: DiagnosticCode,
    message: String,
    include_chain: Vec<PathBuf>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl IncludeError {
    /// Create a new include error.
    #[must_use]
    pub(crate) fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        include_chain: Vec<PathBuf>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            include_chain,
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

    /// Return the include chain captured for the failure.
    #[must_use]
    pub fn include_chain(&self) -> &[PathBuf] {
        &self.include_chain
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

impl fmt::Display for IncludeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for IncludeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
    }
}
