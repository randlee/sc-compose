//! Error types for the known-template extraction contract.

use std::error::Error;
use std::fmt;

use super::{ExtractionDiagnostic, ExtractionDiagnosticKind};

/// Contract-level failures returned by the extraction API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtractError {
    /// The request violates an input or report-construction invariant.
    InvalidRequest {
        /// Explanation of the invalid request.
        message: String,
    },
    /// The rendered input is malformed XML.
    MalformedXml {
        /// Structured evidence for the malformed input.
        diagnostic: ExtractionDiagnostic,
    },
    /// The template uses syntax outside the reversible contract.
    UnsupportedSyntax {
        /// Structured evidence for the unsupported syntax.
        diagnostic: ExtractionDiagnostic,
    },
    /// The input cannot be mapped to one unambiguous extraction result.
    AmbiguousStructure {
        /// Structured evidence for the ambiguity.
        diagnostic: ExtractionDiagnostic,
    },
}

impl ExtractError {
    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::UnsupportedSyntax {
            diagnostic: ExtractionDiagnostic::new(
                "ERR_EXTRACT_UNSUPPORTED",
                ExtractionDiagnosticKind::Unsupported,
                message,
                None,
            ),
        }
    }

    pub(crate) fn ambiguous(
        message: impl Into<String>,
        occurrence: Option<super::OccurrenceIndex>,
    ) -> Self {
        Self::AmbiguousStructure {
            diagnostic: ExtractionDiagnostic::new(
                "ERR_EXTRACT_AMBIGUOUS",
                ExtractionDiagnosticKind::Ambiguous,
                message,
                occurrence,
            ),
        }
    }

    /// Return the structured diagnostic carried by this error, when present.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&ExtractionDiagnostic> {
        match self {
            Self::InvalidRequest { .. } => None,
            Self::MalformedXml { diagnostic }
            | Self::UnsupportedSyntax { diagnostic }
            | Self::AmbiguousStructure { diagnostic } => Some(diagnostic),
        }
    }
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest { message } => write!(f, "invalid extraction request: {message}"),
            Self::MalformedXml { diagnostic }
            | Self::UnsupportedSyntax { diagnostic }
            | Self::AmbiguousStructure { diagnostic } => {
                write!(f, "{}: {}", diagnostic.code, diagnostic.message)
            }
        }
    }
}

impl Error for ExtractError {}
