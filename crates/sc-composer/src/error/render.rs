use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;

use super::display::BoxedError;
use crate::diagnostics::DiagnosticCode;

/// Canonical render error for template compilation and rendering failures.
///
/// This type is only constructed by the library; callers receive it as an
/// opaque error value by design.
#[derive(Debug)]
pub struct RenderError {
    code: Option<DiagnosticCode>,
    message: String,
    source: BoxedError,
    backtrace: Backtrace,
}

impl RenderError {
    /// Construct a canonical render error from an underlying render cause.
    ///
    /// This constructor exists so the library can erase engine-specific error
    /// types at the public API boundary.
    #[must_use]
    pub(crate) fn render(source: impl StdError + Send + Sync + 'static) -> Self {
        let message = source.to_string();
        Self {
            code: None,
            message,
            source: Box::new(source),
            backtrace: Backtrace::capture(),
        }
    }

    /// Return the captured backtrace for the render failure.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    /// Return the stable diagnostic code when one was attached by the caller.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        self.code
    }

    /// Return the render-failure message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "template rendering failed")
    }
}

impl StdError for RenderError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}
