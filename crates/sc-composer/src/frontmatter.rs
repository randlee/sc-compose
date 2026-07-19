//! Typed YAML frontmatter parsing and normalization.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::error::{ComposeError, ConfigError, RecoveryHint, RecoveryHintKind, ValidationError};
use crate::types::{
    InputValue, MetadataValue, VariableName, default_pass_number, input_value_from_yaml,
};

/// Typed frontmatter normalized to explicit empty collections when present.
#[derive(Clone, Debug, PartialEq)]
pub struct Frontmatter {
    pass_number: u8,
    has_explicit_pass_number: bool,
    required_variables: Vec<VariableName>,
    defaults: BTreeMap<VariableName, InputValue>,
    metadata: BTreeMap<String, MetadataValue>,
    diagnostics: Vec<Diagnostic>,
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self {
            pass_number: default_pass_number(),
            has_explicit_pass_number: false,
            required_variables: Vec::new(),
            defaults: BTreeMap::new(),
            metadata: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl Frontmatter {
    /// Create an empty frontmatter value.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return the declared pass number for this header.
    #[must_use]
    pub fn pass_number(&self) -> u8 {
        self.pass_number
    }

    /// Borrow the normalized required-variable declarations.
    #[must_use]
    pub fn required_variables(&self) -> &[VariableName] {
        &self.required_variables
    }

    /// Borrow normalized default values.
    #[must_use]
    pub fn defaults(&self) -> &BTreeMap<VariableName, InputValue> {
        &self.defaults
    }

    /// Borrow descriptive metadata values.
    #[must_use]
    pub fn metadata(&self) -> &BTreeMap<String, MetadataValue> {
        &self.metadata
    }

    /// Borrow non-fatal diagnostics produced while normalizing frontmatter.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub(crate) const fn has_explicit_pass_number(&self) -> bool {
        self.has_explicit_pass_number
    }
}

/// Parsed template document with stacked frontmatter passes and the raw body.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedTemplate {
    passes: Vec<Frontmatter>,
    body: String,
}

impl ParsedTemplate {
    /// Construct a parsed template from already-normalized parts.
    #[must_use]
    pub(crate) fn from_parts(passes: Vec<Frontmatter>, body: String) -> Self {
        Self { passes, body }
    }

    /// Construct a parsed template from normalized parts after re-validating
    /// duplicate explicit pass numbers.
    ///
    /// # Errors
    ///
    /// Returns [`ComposeError`] when the provided passes contain duplicate
    /// explicit `pass` declarations that would have been rejected by
    /// [`parse_template_document`].
    pub fn from_parts_validated(
        passes: Vec<Frontmatter>,
        body: String,
    ) -> Result<Self, ComposeError> {
        validate_explicit_pass_numbers(&passes)?;
        Ok(Self { passes, body })
    }

    /// Borrow the outermost parsed frontmatter if one existed.
    ///
    /// This preserves the backward-compatible single-header accessor surface.
    #[must_use]
    pub fn frontmatter(&self) -> Option<&Frontmatter> {
        self.passes.first()
    }

    /// Borrow all parsed stacked frontmatter passes in outer-to-inner order.
    #[must_use]
    pub fn passes(&self) -> &[Frontmatter] {
        &self.passes
    }

    /// Borrow the normalized body content without frontmatter delimiters.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    pass: Option<u8>,
    #[serde(default)]
    required_variables: Vec<String>,
    #[serde(default)]
    defaults: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    input_defaults: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    metadata: BTreeMap<String, serde_yaml::Value>,
}

/// Parse a full template document and normalize its frontmatter if present.
///
/// # Errors
///
/// Returns [`ComposeError`] when the frontmatter block is malformed, missing a
/// terminating delimiter, or contains values outside the supported Sprint 2
/// schema.
pub fn parse_template_document(input: &str) -> Result<ParsedTemplate, ComposeError> {
    let Some((frontmatter_texts, body)) = split_frontmatter(input)? else {
        return Ok(ParsedTemplate {
            passes: Vec::new(),
            body: input.to_owned(),
        });
    };

    let mut passes = Vec::with_capacity(frontmatter_texts.len());
    for frontmatter_text in frontmatter_texts {
        let raw = serde_yaml::from_str::<RawFrontmatter>(frontmatter_text).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                "failed to parse YAML frontmatter",
            )
            .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "frontmatter".to_owned(),
            }))
            .with_source(error)
        })?;
        let frontmatter = normalize_frontmatter(raw)?;
        passes.push(frontmatter);
    }

    validate_explicit_pass_numbers(&passes)?;

    Ok(ParsedTemplate {
        passes,
        body: body.to_owned(),
    })
}

fn split_frontmatter(input: &str) -> Result<Option<(Vec<&str>, &str)>, ComposeError> {
    let mut cursor = 0usize;
    let mut headers = Vec::new();

    while let Some(open_len) = opening_delimiter_len(input, cursor) {
        let content_start = cursor + open_len;
        let mut line_cursor = content_start;
        let mut closing = None;

        while line_cursor < input.len() {
            let line_end = next_line_end(input, line_cursor);
            let line = &input[line_cursor..line_end];
            let trimmed = line.trim_end_matches(['\n', '\r']);
            if matches!(trimmed, "---" | "...") {
                closing = Some((line_cursor, line_end));
                break;
            }
            line_cursor = line_end;
        }

        let Some((content_end, after_close)) = closing else {
            return Err(ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                "frontmatter block started with `---` but no closing delimiter was found",
            )
            .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "frontmatter".to_owned(),
            }))
            .into());
        };

        headers.push(&input[content_start..content_end]);
        cursor = after_close;
    }

    if headers.is_empty() {
        Ok(None)
    } else {
        Ok(Some((headers, &input[cursor..])))
    }
}

fn opening_delimiter_len(input: &str, cursor: usize) -> Option<usize> {
    let remainder = input.get(cursor..)?;
    if remainder.starts_with("---\r\n") {
        Some(5)
    } else if remainder.starts_with("---\n") {
        Some(4)
    } else if remainder == "---" {
        Some(3)
    } else {
        None
    }
}

fn next_line_end(input: &str, cursor: usize) -> usize {
    match input[cursor..].find('\n') {
        Some(offset) => cursor + offset + 1,
        None => input.len(),
    }
}

fn normalize_frontmatter(raw: RawFrontmatter) -> Result<Frontmatter, ComposeError> {
    let parse_default_entry = |section_name: &str,
                               name: String,
                               value: serde_yaml::Value|
     -> Result<(VariableName, InputValue), ComposeError> {
        let variable = VariableName::new(name).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("invalid frontmatter {section_name} variable name: {error}"),
            )
            .with_recovery_hint(RecoveryHint::new(
                RecoveryHintKind::ReviewConfiguration {
                    key: section_name.to_owned(),
                },
            ))
        })?;
        let input_value = input_value_from_yaml(value).map_err(|error| {
            ValidationError::invalid_input_value(error.code(), error.to_string())
        })?;
        Ok((variable, input_value))
    };

    let RawFrontmatter {
        pass,
        required_variables: raw_required_variables,
        defaults: raw_defaults,
        input_defaults: raw_input_defaults,
        metadata: raw_metadata,
    } = raw;

    let mut required_variables = Vec::with_capacity(raw_required_variables.len());
    let mut seen = BTreeSet::new();
    for variable in raw_required_variables {
        let variable = VariableName::new(variable).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("invalid frontmatter variable name: {error}"),
            )
            .with_recovery_hint(RecoveryHint::new(
                RecoveryHintKind::ReviewConfiguration {
                    key: "required_variables".to_owned(),
                },
            ))
        })?;
        if !seen.insert(variable.clone()) {
            return Err(ValidationError::duplicate_variable(&variable).into());
        }
        required_variables.push(variable);
    }

    let mut diagnostics = Vec::new();
    let mut defaults = BTreeMap::new();
    for (name, value) in raw_defaults {
        let (variable, input_value) = parse_default_entry("default", name, value)?;
        defaults.insert(variable, input_value);
    }

    if !defaults.is_empty() && !raw_input_defaults.is_empty() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticCode::WarnValConflictingDefaultSections,
            "frontmatter contains both `defaults` and `input_defaults`; `input_defaults` overrides overlapping keys",
        ));
    }

    for (name, value) in raw_input_defaults {
        let (variable, input_value) = parse_default_entry("input_defaults", name, value)?;
        defaults.insert(variable, input_value);
    }

    let metadata = raw_metadata
        .into_iter()
        .map(|(key, value)| (key, MetadataValue::new(value)))
        .collect();

    Ok(Frontmatter {
        pass_number: match pass {
            Some(0) | None => default_pass_number(),
            Some(pass_number) => pass_number,
        },
        has_explicit_pass_number: pass.is_some(),
        required_variables,
        defaults,
        metadata,
        diagnostics,
    })
}

fn validate_explicit_pass_numbers(passes: &[Frontmatter]) -> Result<(), ComposeError> {
    let mut seen_explicit_pass_numbers = BTreeSet::new();
    for frontmatter in passes {
        if frontmatter.has_explicit_pass_number()
            && !seen_explicit_pass_numbers.insert(frontmatter.pass_number())
        {
            return Err(ValidationError::invalid_input_value(
                DiagnosticCode::ErrConfigParse,
                format!(
                    "duplicate explicit pass number in stacked frontmatter: {}",
                    frontmatter.pass_number()
                ),
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::parse_template_document;

    #[test]
    fn parses_document_without_frontmatter() {
        let parsed = parse_template_document("hello world").unwrap();

        assert!(parsed.passes().is_empty());
        assert_eq!(parsed.body(), "hello world");
        assert!(parsed.frontmatter().is_none());
    }

    #[test]
    fn parses_single_header_with_explicit_pass() {
        let parsed = parse_template_document("---\npass: 2\n---\nbody").unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(parsed.passes()[0].pass_number(), 2);
        assert_eq!(parsed.frontmatter().unwrap().pass_number(), 2);
        assert_eq!(parsed.body(), "body");
    }

    #[test]
    fn parses_stacked_empty_headers_with_default_pass_numbers() {
        let parsed = parse_template_document("---\n---\n---\n---\nbody").unwrap();

        assert_eq!(parsed.passes().len(), 2);
        assert_eq!(parsed.passes()[0].pass_number(), 1);
        assert_eq!(parsed.passes()[1].pass_number(), 1);
        assert_eq!(parsed.body(), "body");
    }

    #[test]
    fn supports_dot_delimiter_in_stacked_headers() {
        let parsed = parse_template_document("---\n...\n---\n...\nbody").unwrap();

        assert_eq!(parsed.passes().len(), 2);
        assert_eq!(parsed.body(), "body");
    }

    #[test]
    fn preserves_later_body_delimiters_after_leading_header_stack() {
        let parsed =
            parse_template_document("---\ndefaults: {name: world}\n---\nhello\n---\nrule").unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(parsed.body(), "hello\n---\nrule");
    }

    #[test]
    fn malformed_yaml_fails_closed() {
        let error = parse_template_document("---\ndefaults: [\n---\nbody").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("failed to parse YAML frontmatter")
        );
    }

    #[test]
    fn duplicate_explicit_pass_numbers_fail_closed() {
        let error =
            parse_template_document("---\npass: 2\n---\n---\npass: 2\n---\nbody").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("duplicate explicit pass number in stacked frontmatter")
        );
    }

    #[test]
    fn from_parts_validated_allows_omitted_default_pass_duplicates() {
        let parsed = parse_template_document("---\n---\n---\n---\nbody").unwrap();

        let reparsed = super::ParsedTemplate::from_parts_validated(
            parsed.passes().to_vec(),
            "body".to_owned(),
        )
        .unwrap();

        assert_eq!(reparsed.passes().len(), 2);
        assert_eq!(reparsed.passes()[0].pass_number(), 1);
        assert_eq!(reparsed.passes()[1].pass_number(), 1);
    }

    #[test]
    fn from_parts_validated_rejects_duplicate_explicit_pass_numbers() {
        let explicit = super::Frontmatter {
            pass_number: 2,
            has_explicit_pass_number: true,
            required_variables: Vec::new(),
            defaults: BTreeMap::new(),
            metadata: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        let error = super::ParsedTemplate::from_parts_validated(
            vec![explicit.clone(), explicit],
            "body".to_owned(),
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("duplicate explicit pass number in stacked frontmatter")
        );
    }
}
