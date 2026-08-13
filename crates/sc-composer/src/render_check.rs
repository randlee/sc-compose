//! Format-aware checks that must complete before rendered output is emitted.

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::renderer::JsonEscapeMode;

/// Output formats understood by the checked-render contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// A complete JSON document is required.
    Json,
    /// A text format whose semantics are unchanged by this checker.
    Text,
}

impl OutputFormat {
    /// Infer the output format from a template path.
    #[must_use]
    pub fn from_template_path(path: &Path) -> Self {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let normalized = strip_all_template_suffixes(file_name);
        if Path::new(normalized)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            Self::Json
        } else {
            Self::Text
        }
    }
}

/// Metadata identifying the template and effective rendering contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RenderCheckMeta {
    /// Template path used for the check.
    pub template: PathBuf,
    /// Detected output format.
    pub output_format: OutputFormat,
    /// Effective JSON interpolation mode, when applicable.
    pub json_escape_mode: Option<JsonEscapeMode>,
}

impl RenderCheckMeta {
    /// Build metadata for a template path with no explicit mode override.
    #[must_use]
    pub fn for_template(template: impl Into<PathBuf>) -> Self {
        let template = template.into();
        let output_format = OutputFormat::from_template_path(&template);
        Self {
            template,
            output_format,
            json_escape_mode: None,
        }
    }

    /// Set the effective JSON mode for this report.
    #[must_use]
    pub fn with_json_escape_mode(mut self, mode: Option<JsonEscapeMode>) -> Self {
        self.json_escape_mode = mode;
        self
    }
}

/// A summary of the exact context used for a checked render.
pub type ContextSummary = String;

/// Machine-readable result states for static and context-specific checks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RenderCheckReport {
    /// Static validation completed without claiming a rendered output.
    StaticOnly {
        /// Template and mode metadata.
        #[serde(flatten)]
        meta: RenderCheckMeta,
        /// Diagnostics from static validation.
        diagnostics: Vec<Diagnostic>,
    },
    /// Static contract validation failed before rendering.
    ContractInvalid {
        /// Template and mode metadata.
        #[serde(flatten)]
        meta: RenderCheckMeta,
        /// Diagnostics explaining the contract failure.
        diagnostics: Vec<Diagnostic>,
    },
    /// Rendering needs a caller context that was not supplied or was incomplete.
    ContextRequired {
        /// Template and mode metadata.
        #[serde(flatten)]
        meta: RenderCheckMeta,
        /// Diagnostics explaining the missing context.
        diagnostics: Vec<Diagnostic>,
    },
    /// Rendering completed but the output failed its format check.
    RenderInvalid {
        /// Template and mode metadata.
        #[serde(flatten)]
        meta: RenderCheckMeta,
        /// Diagnostics explaining why emission was refused.
        diagnostics: Vec<Diagnostic>,
    },
    /// Rendering completed and the exact output passed its format check.
    RenderChecked {
        /// Template and mode metadata.
        #[serde(flatten)]
        meta: RenderCheckMeta,
        /// Redacted summary of the exact context used.
        checked_context: ContextSummary,
        /// Advisory diagnostics collected during the check.
        diagnostics: Vec<Diagnostic>,
    },
}

impl RenderCheckReport {
    /// Return the report diagnostics without exposing a mutable internal state.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::StaticOnly { diagnostics, .. }
            | Self::ContractInvalid { diagnostics, .. }
            | Self::ContextRequired { diagnostics, .. }
            | Self::RenderInvalid { diagnostics, .. }
            | Self::RenderChecked { diagnostics, .. } => diagnostics,
        }
    }

    /// Return whether this report authorizes output emission.
    #[must_use]
    pub const fn permits_emission(&self) -> bool {
        matches!(self, Self::RenderChecked { .. })
    }
}

/// Output that passed the format checker and is safe to emit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedOutput {
    body: String,
    /// Template and mode metadata for this checked output.
    pub meta: RenderCheckMeta,
}

impl CheckedOutput {
    /// Borrow the checked body without allowing callers to bypass the check.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Emit the checked body to a writer.
    ///
    /// Only a [`CheckedOutput`] can call this method, keeping parser failures
    /// out of the emission path.
    ///
    /// # Errors
    ///
    /// Returns the writer's I/O error when the output cannot be written in
    /// full.
    pub fn emit<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        writer.write_all(self.body.as_bytes())
    }
}

/// Typed failure returned by [`check_rendered_output`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OutputCheckError {
    /// Machine-readable reason for the rejection.
    pub reason: OutputCheckReason,
    /// Redacted diagnostics suitable for CLI or adapter output.
    pub diagnostics: Vec<Diagnostic>,
}

impl OutputCheckError {
    /// Attach the pass whose final output was rejected, when rendering used
    /// stacked passes. The parser location remains the authoritative source
    /// location; this context identifies which pass produced that body.
    #[must_use]
    pub fn with_failing_pass(mut self, pass_number: Option<u8>) -> Self {
        if let Some(pass_number) = pass_number {
            for diagnostic in &mut self.diagnostics {
                diagnostic.message =
                    format!("{} (after render pass {pass_number})", diagnostic.message);
            }
        }
        self
    }
}

impl fmt::Display for OutputCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("rendered output failed the checked-render contract")
    }
}

impl std::error::Error for OutputCheckError {}

/// Stable reason for a checked-render failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputCheckReason {
    /// JSON parsing failed at the reported location.
    InvalidJson {
        /// One-based source line.
        line: usize,
        /// One-based source column.
        column: usize,
        /// Zero-based byte offset.
        byte_offset: usize,
    },
    /// The requested format contract was violated.
    ContractViolation,
    /// Rendering itself failed before output checking.
    RenderFailure,
}

/// Check a complete rendered body before it reaches stdout or a file.
///
/// JSON is parsed as a complete document. Other formats are currently a
/// deliberate no-op and still return a checked value so their existing output
/// behavior remains unchanged.
///
/// # Errors
///
/// Returns [`OutputCheckError`] when a JSON body is not one complete valid JSON
/// document. The error includes a stable diagnostic location and never echoes
/// rendered values.
pub fn check_rendered_output(
    format: OutputFormat,
    template: &Path,
    rendered: &str,
) -> Result<CheckedOutput, OutputCheckError> {
    let meta = RenderCheckMeta::for_template(template);
    if format != OutputFormat::Json {
        return Ok(CheckedOutput {
            body: rendered.to_owned(),
            meta,
        });
    }

    let Err(error) = serde_json::from_str::<serde_json::Value>(rendered) else {
        return Ok(CheckedOutput {
            body: rendered.to_owned(),
            meta,
        });
    };
    let line = error.line();
    let column = error.column();
    let byte_offset = byte_offset_at(rendered, line, column);
    let diagnostic = Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::ErrRenderJsonMalformed,
        format!(
            "rendered JSON is invalid at line {line}, column {column}, byte offset {byte_offset}"
        ),
    )
    .with_path(template)
    .with_location(line, column);
    Err(OutputCheckError {
        reason: OutputCheckReason::InvalidJson {
            line,
            column,
            byte_offset,
        },
        diagnostics: vec![diagnostic],
    })
}

fn strip_all_template_suffixes(mut name: &str) -> &str {
    while let Some(stripped) = strip_one_template_suffix_case_insensitive(name) {
        name = stripped;
    }
    name
}

fn strip_one_template_suffix_case_insensitive(name: &str) -> Option<&str> {
    [".j2", ".jinja2", ".jinja"].iter().find_map(|suffix| {
        let start = name.len().checked_sub(suffix.len())?;
        let candidate = name.get(start..)?;
        candidate
            .eq_ignore_ascii_case(suffix)
            .then_some(&name[..start])
    })
}

fn byte_offset_at(text: &str, line: usize, column: usize) -> usize {
    let line_start = text
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let remaining = text.get(line_start..).unwrap_or_default();
    let line_text = remaining
        .split_once('\n')
        .map_or(remaining, |(line, _)| line);
    let character_offset = column.saturating_sub(1);
    let byte_offset_in_line = line_text
        .char_indices()
        .nth(character_offset)
        .map_or(line_text.len(), |(offset, _)| offset);
    line_start
        .saturating_add(byte_offset_in_line)
        .min(text.len())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{OutputFormat, RenderCheckMeta, RenderCheckReport, check_rendered_output};

    #[test]
    fn parser_accepts_json_document_shapes_and_whitespace() {
        for body in [" {\"name\":\"Ada\"} ", "[1, true, null]", "42", "\"text\""] {
            let checked =
                check_rendered_output(OutputFormat::Json, Path::new("payload.json.j2"), body)
                    .expect("valid JSON");
            assert_eq!(checked.body(), body);
        }
    }

    #[test]
    fn output_format_detects_stacked_and_case_variant_json_suffixes() {
        for path in ["payload.json.j2.j2", "payload.JSON.j2"] {
            assert_eq!(
                OutputFormat::from_template_path(Path::new(path)),
                OutputFormat::Json,
                "path={path}"
            );
        }
    }

    #[test]
    fn parser_reports_location_without_echoing_secret_payload() {
        let error = check_rendered_output(
            OutputFormat::Json,
            Path::new("payload.json.j2"),
            "{\"secret\": \"TOP-SECRET\"",
        )
        .expect_err("malformed JSON");
        assert!(error.diagnostics[0].message.contains("line 1"));
        assert!(!error.diagnostics[0].message.contains("TOP-SECRET"));
        assert!(matches!(
            error.reason,
            super::OutputCheckReason::InvalidJson { line: 1, .. }
        ));
    }

    #[test]
    fn parser_error_can_identify_the_failing_multi_pass() {
        let error = check_rendered_output(
            OutputFormat::Json,
            Path::new("payload.json.j2"),
            "{\"value\": \"broken",
        )
        .expect_err("malformed JSON");
        let error = error.with_failing_pass(Some(2));
        assert!(error.diagnostics[0].message.contains("after render pass 2"));
    }

    #[test]
    fn non_json_check_preserves_body() {
        let checked = check_rendered_output(OutputFormat::Text, Path::new("notes.md.j2"), "body")
            .expect("text output is unchanged");
        assert_eq!(checked.body(), "body");
    }

    #[test]
    fn invalid_json_byte_offset_lands_on_a_char_boundary_with_multibyte_prefix() {
        let rendered = "\"\u{00e9}";
        let error =
            check_rendered_output(OutputFormat::Json, Path::new("payload.json.j2"), rendered)
                .expect_err("unterminated string is malformed JSON");
        let super::OutputCheckReason::InvalidJson { byte_offset, .. } = error.reason else {
            panic!("expected InvalidJson reason, got {:?}", error.reason);
        };
        assert!(byte_offset <= rendered.len());
        assert!(rendered.is_char_boundary(byte_offset));
        assert_eq!(byte_offset, rendered.len());
    }

    #[test]
    fn report_states_are_explicit_and_only_checked_permits_emission() {
        let meta = RenderCheckMeta::for_template("payload.json.j2");
        let static_only = RenderCheckReport::StaticOnly {
            meta: meta.clone(),
            diagnostics: Vec::new(),
        };
        let checked = RenderCheckReport::RenderChecked {
            meta,
            checked_context: "2 variables".to_owned(),
            diagnostics: Vec::new(),
        };
        assert!(!static_only.permits_emission());
        assert!(checked.permits_emission());
        assert_eq!(
            serde_json::to_value(checked).unwrap()["state"],
            "render_checked"
        );
    }
}
