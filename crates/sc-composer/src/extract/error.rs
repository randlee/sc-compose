//! Error types for the known-template extraction contract.

use std::error::Error;
use std::fmt;

use crate::diagnostics::DiagnosticCode;
use crate::error::{RecoveryHint, RecoveryHintKind};

use super::{ExtractionDiagnostic, ExtractionDiagnosticKind};

/// Retained display information for a parser or decoder source error.
///
/// Extraction errors retain source text rather than the parser's concrete
/// error type so the public error remains cloneable and comparable while
/// still implementing the standard [`Error::source`] chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractErrorSource {
    message: String,
}

impl ExtractErrorSource {
    fn from_error(error: &impl Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl fmt::Display for ExtractErrorSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ExtractErrorSource {}

/// Contract-level failures returned by the extraction API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtractError {
    /// The request violates an input or report-construction invariant.
    InvalidRequest {
        /// Stable diagnostic code for the invalid request.
        code: DiagnosticCode,
        /// Explanation of the invalid request.
        message: String,
        /// Structured suggestions for correcting the request.
        recovery_hints: Vec<RecoveryHint>,
        /// Retained underlying parser or decoder source, when present.
        source: Option<ExtractErrorSource>,
    },
    /// The rendered input is malformed XML.
    MalformedXml {
        /// Structured evidence for the malformed input.
        diagnostic: ExtractionDiagnostic,
        /// Structured suggestions for correcting the input.
        recovery_hints: Vec<RecoveryHint>,
        /// Retained underlying parser or decoder source, when present.
        source: Option<ExtractErrorSource>,
    },
    /// The template uses syntax outside the reversible contract.
    UnsupportedSyntax {
        /// Structured evidence for the unsupported syntax.
        diagnostic: ExtractionDiagnostic,
        /// Structured suggestions for selecting supported syntax.
        recovery_hints: Vec<RecoveryHint>,
        /// Retained underlying parser or decoder source, when present.
        source: Option<ExtractErrorSource>,
    },
    /// The input cannot be mapped to one unambiguous extraction result.
    AmbiguousStructure {
        /// Structured evidence for the ambiguity.
        diagnostic: ExtractionDiagnostic,
        /// Structured suggestions for disambiguating the result.
        recovery_hints: Vec<RecoveryHint>,
        /// Retained underlying parser or decoder source, when present.
        source: Option<ExtractErrorSource>,
    },
}

impl ExtractError {
    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            code: DiagnosticCode::ErrExtractInvalidRequest,
            message: message.into(),
            recovery_hints: vec![review_configuration(
                "extraction request sources and filters",
            )],
            source: None,
        }
    }

    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::unsupported_with_boxed(message.into(), None)
    }

    pub(crate) fn unsupported_with_source(message: impl Into<String>, source: impl Error) -> Self {
        Self::unsupported_with_boxed(
            message.into(),
            Some(ExtractErrorSource::from_error(&source)),
        )
    }

    fn unsupported_with_boxed(message: String, source: Option<ExtractErrorSource>) -> Self {
        Self::UnsupportedSyntax {
            diagnostic: ExtractionDiagnostic::new(
                DiagnosticCode::ErrExtractUnsupported,
                ExtractionDiagnosticKind::Unsupported,
                message,
                None,
            ),
            recovery_hints: vec![review_configuration(
                "supported XML scalar syntax and filters",
            )],
            source,
        }
    }

    pub(crate) fn malformed(message: impl Into<String>) -> Self {
        Self::malformed_with_boxed(message.into(), None)
    }

    pub(crate) fn malformed_with_source(message: impl Into<String>, source: impl Error) -> Self {
        Self::malformed_with_boxed(
            message.into(),
            Some(ExtractErrorSource::from_error(&source)),
        )
    }

    fn malformed_with_boxed(message: String, source: Option<ExtractErrorSource>) -> Self {
        Self::MalformedXml {
            diagnostic: ExtractionDiagnostic::new(
                DiagnosticCode::ErrExtractMalformed,
                ExtractionDiagnosticKind::Malformed,
                message,
                None,
            ),
            recovery_hints: vec![RecoveryHint::new(RecoveryHintKind::InspectInput {
                description: "inspect the rendered XML for well-formed elements and entities"
                    .to_owned(),
            })],
            source,
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
            recovery_hints: vec![RecoveryHint::new(
                RecoveryHintKind::DisambiguateOccurrences {
                    description: "review XML occurrence paths and include/exclude selection"
                        .to_owned(),
                },
            )],
            source: None,
        }
    }

    fn recovery_hints_for(&self) -> &[RecoveryHint] {
        match self {
            Self::InvalidRequest { recovery_hints, .. }
            | Self::MalformedXml { recovery_hints, .. }
            | Self::UnsupportedSyntax { recovery_hints, .. }
            | Self::AmbiguousStructure { recovery_hints, .. } => recovery_hints,
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
}

fn review_configuration(key: &str) -> RecoveryHint {
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
        let source = match self {
            Self::InvalidRequest { source, .. }
            | Self::MalformedXml { source, .. }
            | Self::UnsupportedSyntax { source, .. }
            | Self::AmbiguousStructure { source, .. } => source.as_ref(),
        }?;
        Some(source)
    }
}
