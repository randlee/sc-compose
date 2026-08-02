//! Error types for the known-template extraction contract.

use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt;

use crate::diagnostics::DiagnosticCode;
use crate::error::{RecoveryHint, RecoveryHintKind};

use super::{ExtractionDiagnostic, ExtractionDiagnosticKind};

type BoxedError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Debug)]
pub struct ErrorDetails {
    recovery_hints: Vec<RecoveryHint>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ErrorDetails {
    fn new(recovery_hints: Vec<RecoveryHint>, source: Option<BoxedError>) -> Self {
        Self {
            recovery_hints,
            source,
            backtrace: Backtrace::capture(),
        }
    }
}

/// Contract-level failures returned by the extraction API.
#[derive(Debug)]
pub enum ExtractError {
    /// The request violates an input or report-construction invariant.
    InvalidRequest {
        /// Stable diagnostic code for the invalid request.
        code: DiagnosticCode,
        /// Explanation of the invalid request.
        message: String,
        /// Recovery metadata, source chain, and construction backtrace.
        details: Box<ErrorDetails>,
    },
    /// The rendered input is malformed XML.
    MalformedXml {
        /// Structured evidence for the malformed input.
        diagnostic: ExtractionDiagnostic,
        /// Recovery metadata, source chain, and construction backtrace.
        details: Box<ErrorDetails>,
    },
    /// The template uses syntax outside the reversible contract.
    UnsupportedSyntax {
        /// Structured evidence for the unsupported syntax.
        diagnostic: ExtractionDiagnostic,
        /// Recovery metadata, source chain, and construction backtrace.
        details: Box<ErrorDetails>,
    },
    /// The input cannot be mapped to one unambiguous extraction result.
    AmbiguousStructure {
        /// Structured evidence for the ambiguity.
        diagnostic: ExtractionDiagnostic,
        /// Recovery metadata, source chain, and construction backtrace.
        details: Box<ErrorDetails>,
    },
}

impl ExtractError {
    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            code: DiagnosticCode::ErrExtractInvalidRequest,
            message: message.into(),
            details: Box::new(ErrorDetails::new(
                vec![review_hint("extraction request sources and filters")],
                None,
            )),
        }
    }

    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::unsupported_with_boxed(message.into(), None)
    }

    pub(crate) fn unsupported_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::unsupported_with_boxed(message.into(), Some(Box::new(source)))
    }

    fn unsupported_with_boxed(message: String, source: Option<BoxedError>) -> Self {
        Self::UnsupportedSyntax {
            diagnostic: ExtractionDiagnostic::new(
                DiagnosticCode::ErrExtractUnsupported,
                ExtractionDiagnosticKind::Unsupported,
                message,
                None,
            ),
            details: Box::new(ErrorDetails::new(
                vec![review_hint("supported XML scalar syntax and filters")],
                source,
            )),
        }
    }

    pub(crate) fn malformed(message: impl Into<String>) -> Self {
        Self::malformed_with_boxed(message.into(), None)
    }

    pub(crate) fn malformed_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::malformed_with_boxed(message.into(), Some(Box::new(source)))
    }

    fn malformed_with_boxed(message: String, source: Option<BoxedError>) -> Self {
        Self::MalformedXml {
            diagnostic: ExtractionDiagnostic::new(
                DiagnosticCode::ErrExtractMalformed,
                ExtractionDiagnosticKind::Malformed,
                message,
                None,
            ),
            details: Box::new(ErrorDetails::new(
                vec![review_hint(
                    "well-formed rendered XML and entity declarations",
                )],
                source,
            )),
        }
    }

    pub(crate) fn ambiguous(
        message: impl Into<String>,
        occurrence: Option<super::OccurrenceIndex>,
    ) -> Self {
        Self::AmbiguousStructure {
            diagnostic: ExtractionDiagnostic::new(
                DiagnosticCode::ErrExtractAmbiguous,
                ExtractionDiagnosticKind::Ambiguous,
                message,
                occurrence,
            ),
            details: Box::new(ErrorDetails::new(
                vec![review_hint(
                    "XML occurrence paths and include/exclude selection",
                )],
                None,
            )),
        }
    }

    fn recovery_hints_for(&self) -> &[RecoveryHint] {
        match self {
            Self::InvalidRequest { details, .. }
            | Self::MalformedXml { details, .. }
            | Self::UnsupportedSyntax { details, .. }
            | Self::AmbiguousStructure { details, .. } => &details.recovery_hints,
        }
    }

    /// Return the stable diagnostic code associated with this error.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        match self {
            Self::InvalidRequest { code, .. } => *code,
            Self::MalformedXml { diagnostic, .. }
            | Self::UnsupportedSyntax { diagnostic, .. }
            | Self::AmbiguousStructure { diagnostic, .. } => diagnostic.code,
        }
    }

    /// Return the structured diagnostic carried by this error, when present.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&ExtractionDiagnostic> {
        match self {
            Self::InvalidRequest { .. } => None,
            Self::MalformedXml { diagnostic, .. }
            | Self::UnsupportedSyntax { diagnostic, .. }
            | Self::AmbiguousStructure { diagnostic, .. } => Some(diagnostic),
        }
    }

    /// Return structured recovery hints for this error.
    #[must_use]
    pub fn recovery_hints(&self) -> &[RecoveryHint] {
        self.recovery_hints_for()
    }

    /// Return the captured construction backtrace.
    pub fn backtrace(&self) -> &Backtrace {
        match self {
            Self::InvalidRequest { details, .. }
            | Self::MalformedXml { details, .. }
            | Self::UnsupportedSyntax { details, .. }
            | Self::AmbiguousStructure { details, .. } => &details.backtrace,
        }
    }
}

fn review_hint(key: &str) -> RecoveryHint {
    RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
        key: key.to_owned(),
    })
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest { message, .. } => {
                write!(f, "invalid extraction request: {message}")
            }
            Self::MalformedXml { diagnostic, .. }
            | Self::UnsupportedSyntax { diagnostic, .. }
            | Self::AmbiguousStructure { diagnostic, .. } => {
                write!(f, "{}: {}", diagnostic.code.as_str(), diagnostic.message)?;
                if let Some(source) = self.source() {
                    write!(f, " (caused by: {source})")?;
                }
                Ok(())
            }
        }
    }
}

impl Error for ExtractError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidRequest { details, .. }
            | Self::MalformedXml { details, .. }
            | Self::UnsupportedSyntax { details, .. }
            | Self::AmbiguousStructure { details, .. } => details
                .source
                .as_deref()
                .map(|error| error as &(dyn Error + 'static)),
        }
    }
}
