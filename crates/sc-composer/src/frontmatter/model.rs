//! Public frontmatter models and the raw YAML schema consumed by normalization.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::diagnostics::Diagnostic;
use crate::error::ComposeError;
use crate::renderer::JsonEscapeMode;
use crate::types::{InputValue, MetadataValue, VariableName, default_pass_number};

/// Typed YAML frontmatter parsed from a template header.
#[derive(Clone, Debug, PartialEq)]
pub struct Frontmatter {
    pub(super) pass_number: u8,
    pub(super) has_explicit_pass_number: bool,
    pub(super) required_variables: Vec<VariableName>,
    pub(super) defaults: BTreeMap<VariableName, InputValue>,
    pub(super) metadata: BTreeMap<String, MetadataValue>,
    pub(super) json_escape_mode: Option<JsonEscapeMode>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self {
            pass_number: default_pass_number(),
            has_explicit_pass_number: false,
            required_variables: Vec::new(),
            defaults: BTreeMap::new(),
            metadata: BTreeMap::new(),
            json_escape_mode: None,
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

    /// Return the optional JSON interpolation mode declared by this header.
    #[must_use]
    pub fn json_escape_mode(&self) -> Option<JsonEscapeMode> {
        self.json_escape_mode
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
    pub(super) passes: Vec<Frontmatter>,
    pub(super) body: String,
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
    /// [`crate::frontmatter::parse_template_document`].
    pub fn from_parts_validated(
        passes: Vec<Frontmatter>,
        body: String,
    ) -> Result<Self, ComposeError> {
        super::normalizer::validate_explicit_pass_numbers(&passes)?;
        Ok(Self { passes, body })
    }

    /// Borrow the outermost parsed frontmatter if one existed.
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
pub(super) struct RawFrontmatter {
    pub(super) pass: Option<u8>,
    #[serde(default)]
    pub(super) required_variables: Vec<String>,
    #[serde(default)]
    pub(super) variables: BTreeMap<String, RawVariable>,
    #[serde(default)]
    pub(super) defaults: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub(super) input_defaults: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub(super) metadata: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub(super) json_escape_mode: Option<JsonEscapeMode>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawVariable {
    #[serde(default)]
    pub(super) required: bool,
}
