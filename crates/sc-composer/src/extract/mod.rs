//! Pure known-template extraction contract and report model.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::diagnostics::DiagnosticCode;
use crate::types::VariableName;

mod error;
mod xml;

#[cfg(test)]
mod tests;

pub use error::ExtractError;
pub use xml::{ExtractionSource, XmlExtractionOccurrence, XmlExtractionReport, XmlPathSegment};

/// Output format supported by the initial extraction contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractFormat {
    /// Known-template XML output.
    Xml,
}

/// In-memory request for known-template extraction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractRequest<'a> {
    /// The known template source.
    pub template: &'a str,
    /// The rendered output source.
    pub rendered: &'a str,
    /// Format of the rendered output.
    pub format: ExtractFormat,
    /// Variables to include, or an empty slice for no include filter.
    pub include: &'a [VariableName],
    /// Variables to exclude, or an empty slice for no exclude filter.
    pub exclude: &'a [VariableName],
}

impl<'a> ExtractRequest<'a> {
    /// Create an in-memory extraction request.
    #[must_use]
    pub const fn new(
        template: &'a str,
        rendered: &'a str,
        format: ExtractFormat,
        include: &'a [VariableName],
        exclude: &'a [VariableName],
    ) -> Self {
        Self {
            template,
            rendered,
            format,
            include,
            exclude,
        }
    }

    /// Validate request shape without reading files or performing extraction.
    ///
    /// # Errors
    ///
    /// Returns [`ExtractError::InvalidRequest`] for empty sources, duplicate
    /// filters, or a variable present in both filters.
    pub fn validate(&self) -> Result<(), ExtractError> {
        if self.template.trim().is_empty() {
            return Err(ExtractError::invalid_request(
                "template source must not be empty",
            ));
        }
        if self.rendered.trim().is_empty() {
            return Err(ExtractError::invalid_request(
                "rendered source must not be empty",
            ));
        }

        let mut include = BTreeSet::new();
        for variable in self.include {
            if !include.insert(variable) {
                return Err(ExtractError::invalid_request(format!(
                    "duplicate include variable: {variable}"
                )));
            }
        }

        let mut exclude = BTreeSet::new();
        for variable in self.exclude {
            if !exclude.insert(variable) {
                return Err(ExtractError::invalid_request(format!(
                    "duplicate exclude variable: {variable}"
                )));
            }
            if include.contains(variable) {
                return Err(ExtractError::invalid_request(format!(
                    "variable appears in both include and exclude filters: {variable}"
                )));
            }
        }

        Ok(())
    }
}

/// Report returned by extraction, generic over format-specific path/source types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtractionReport<P = OccurrencePathSegment, S = OccurrenceSource> {
    /// Successfully recovered string values.
    pub values: BTreeMap<VariableName, String>,
    /// Structural evidence for observed variable occurrences.
    pub occurrences: Vec<ExtractionOccurrence<P, S>>,
    /// Report-level confidence in the closed range `0.0..=1.0`.
    pub confidence: f64,
    /// Warnings and non-fatal diagnostics attached to the report.
    pub diagnostics: Vec<ExtractionDiagnostic>,
}

impl<P, S> ExtractionReport<P, S> {
    /// Construct a validated extraction report.
    ///
    /// # Errors
    ///
    /// Returns [`ExtractError::InvalidRequest`] for a non-finite or out of
    /// range confidence. Repeated variables remain in `occurrences`, but are
    /// omitted from `values` and receive an `Ambiguous` diagnostic so callers
    /// can inspect the evidence without accidentally consuming a guessed value.
    /// Such a report is also capped below the high-confidence threshold.
    pub fn new(
        values: BTreeMap<VariableName, String>,
        occurrences: Vec<ExtractionOccurrence<P, S>>,
        mut confidence: f64,
        diagnostics: Vec<ExtractionDiagnostic>,
    ) -> Result<Self, ExtractError> {
        if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
            return Err(ExtractError::invalid_request(
                "confidence must be finite and within 0.0..=1.0",
            ));
        }

        let mut values = values;
        let mut diagnostics = diagnostics;
        let mut seen = BTreeSet::new();
        let mut duplicate_diagnostics = Vec::new();
        let mut has_duplicates = false;
        for (index, occurrence) in occurrences.iter().enumerate() {
            if !seen.insert(&occurrence.variable) {
                has_duplicates = true;
                values.remove(&occurrence.variable);
                duplicate_diagnostics.push(ExtractionDiagnostic::new(
                    DiagnosticCode::ErrExtractAmbiguous,
                    ExtractionDiagnosticKind::Ambiguous,
                    format!(
                        "variable has multiple structural occurrences: {}",
                        occurrence.variable
                    ),
                    Some(OccurrenceIndex(index)),
                ));
            }
        }
        diagnostics.extend(duplicate_diagnostics);
        if has_duplicates {
            confidence = confidence.min(0.5);
            if !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::WarnExtractLowConfidence)
            {
                diagnostics.push(ExtractionDiagnostic::new(
                    DiagnosticCode::WarnExtractLowConfidence,
                    ExtractionDiagnosticKind::NotObserved,
                    "duplicate variable occurrences prevent a high-confidence extraction",
                    None,
                ));
            }
        }

        Ok(Self {
            values,
            occurrences,
            confidence,
            diagnostics,
        })
    }
}

/// Evidence for one variable occurrence in a structured output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtractionOccurrence<P = OccurrencePathSegment, S = OccurrenceSource> {
    /// Variable recovered at this occurrence.
    pub variable: VariableName,
    /// Structural path identifying this occurrence.
    pub path: Vec<P>,
    /// Format-specific source information.
    pub source: S,
    /// Rendered string observed at this occurrence.
    pub rendered_text: Option<String>,
}

/// Generic structural path segment before a format specializes the model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OccurrencePathSegment {
    /// A named container and its sibling ordinal.
    Node {
        /// Stable container label.
        label: String,
        /// Zero-based sibling ordinal.
        ordinal: usize,
    },
    /// A value leaf and its sibling ordinal.
    Value {
        /// Optional value label.
        label: Option<String>,
        /// Zero-based sibling ordinal.
        ordinal: usize,
    },
}

/// Generic source information before a format specializes the model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OccurrenceSource {
    /// A named source kind and optional label.
    Named {
        /// Format-specific source kind.
        kind: String,
        /// Optional source label.
        label: Option<String>,
    },
}

/// Stable diagnostic attached to a report or extraction error.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionDiagnostic {
    /// Stable diagnostic code.
    pub code: DiagnosticCode,
    /// Semantic diagnostic category.
    pub kind: ExtractionDiagnosticKind,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Occurrence index associated with the diagnostic, when applicable.
    pub occurrence: Option<OccurrenceIndex>,
}

impl ExtractionDiagnostic {
    /// Construct a diagnostic with stable code, category, and message.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        kind: ExtractionDiagnosticKind,
        message: impl Into<String>,
        occurrence: Option<OccurrenceIndex>,
    ) -> Self {
        Self {
            code,
            kind,
            message: message.into(),
            occurrence,
        }
    }
}

/// Semantic category for extraction diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionDiagnosticKind {
    /// The template or format is outside the supported reversible subset.
    Unsupported,
    /// Multiple structural interpretations remain possible.
    Ambiguous,
    /// A declared occurrence was not observed in the rendered output.
    NotObserved,
    /// The template or rendered output is malformed.
    Malformed,
}

/// Stable zero-based index into a report's occurrence vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OccurrenceIndex(pub usize);

/// Validate a request and reserve the extraction entry point for G.2.
///
/// # Errors
///
/// Invalid requests return [`ExtractError::InvalidRequest`]. XML requests are
/// evaluated against the known-template structural extraction contract.
pub fn extract(request: &ExtractRequest<'_>) -> Result<xml::XmlExtractionReport, ExtractError> {
    request.validate()?;
    match request.format {
        ExtractFormat::Xml => xml::extract_xml(request),
    }
}
