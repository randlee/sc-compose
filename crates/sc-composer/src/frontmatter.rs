//! Typed YAML frontmatter parsing and normalization.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::error::{ComposeError, ConfigError, ValidationError};
use crate::types::{InputValue, MetadataValue, VariableName, input_value_from_yaml};

/// Typed frontmatter normalized to explicit empty collections when present.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frontmatter {
    required_variables: Vec<VariableName>,
    defaults: BTreeMap<VariableName, InputValue>,
    metadata: BTreeMap<String, MetadataValue>,
    diagnostics: Vec<Diagnostic>,
}

impl Frontmatter {
    /// Create an empty frontmatter value.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
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
}

/// Parsed template document with optional frontmatter and the raw body.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedTemplate {
    frontmatter: Option<Frontmatter>,
    body: String,
}

impl ParsedTemplate {
    /// Borrow the parsed frontmatter if one existed.
    #[must_use]
    pub fn frontmatter(&self) -> Option<&Frontmatter> {
        self.frontmatter.as_ref()
    }

    /// Borrow the normalized body content without frontmatter delimiters.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Debug, Deserialize)]
struct RawFrontmatter {
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
    let Some((frontmatter_text, body)) = split_frontmatter(input)? else {
        return Ok(ParsedTemplate {
            frontmatter: None,
            body: input.to_owned(),
        });
    };

    let raw = serde_yaml::from_str::<RawFrontmatter>(frontmatter_text).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            "failed to parse YAML frontmatter",
        )
        .with_source(error)
    })?;

    let frontmatter = normalize_frontmatter(raw)?;

    Ok(ParsedTemplate {
        frontmatter: Some(frontmatter),
        body: body.to_owned(),
    })
}

fn split_frontmatter(input: &str) -> Result<Option<(&str, &str)>, ComposeError> {
    let delimiter_len = if input.starts_with("---\n") {
        4
    } else if input.starts_with("---\r\n") {
        5
    } else {
        return Ok(None);
    };

    let rest = &input[delimiter_len..];
    let mut scanned = 0usize;

    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if matches!(trimmed, "---" | "...") {
            let frontmatter_text = &rest[..scanned];
            let body = &rest[scanned + line.len()..];
            return Ok(Some((frontmatter_text, body)));
        }
        scanned += line.len();
    }

    if !rest.is_empty() {
        let trimmed = rest.trim_end_matches(['\n', '\r']);
        if matches!(trimmed, "---" | "...") {
            return Ok(Some(("", "")));
        }
    }

    Err(ConfigError::new(
        DiagnosticCode::ErrConfigParse,
        "frontmatter block started with `---` but no closing delimiter was found",
    )
    .into())
}

fn normalize_frontmatter(raw: RawFrontmatter) -> Result<Frontmatter, ComposeError> {
    let mut required_variables = Vec::with_capacity(raw.required_variables.len());
    let mut seen = BTreeSet::new();
    for variable in raw.required_variables {
        let variable = VariableName::new(variable).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("invalid frontmatter variable name: {error}"),
            )
        })?;
        if !seen.insert(variable.clone()) {
            return Err(ValidationError::duplicate_variable(&variable).into());
        }
        required_variables.push(variable);
    }

    let mut diagnostics = Vec::new();
    let mut defaults = BTreeMap::new();
    for (name, value) in raw.defaults {
        let variable = VariableName::new(name).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("invalid frontmatter default variable name: {error}"),
            )
        })?;
        let input_value = input_value_from_yaml(value)
            .map_err(|error| ValidationError::invalid_scalar(error.to_string()))?;
        defaults.insert(variable, input_value);
    }

    if !defaults.is_empty() && !raw.input_defaults.is_empty() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticCode::ErrValDuplicate,
            "frontmatter contains both `defaults` and `input_defaults`; `input_defaults` overrides overlapping keys",
        ));
    }

    for (name, value) in raw.input_defaults {
        let variable = VariableName::new(name).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("invalid frontmatter input_defaults variable name: {error}"),
            )
        })?;
        let input_value = input_value_from_yaml(value)
            .map_err(|error| ValidationError::invalid_scalar(error.to_string()))?;
        defaults.insert(variable, input_value);
    }

    let metadata = raw
        .metadata
        .into_iter()
        .map(|(key, value)| (key, MetadataValue::new(value)))
        .collect();

    Ok(Frontmatter {
        required_variables,
        defaults,
        metadata,
        diagnostics,
    })
}
