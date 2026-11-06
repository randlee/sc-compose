//! Public frontmatter models and the raw YAML schema consumed by normalization.

use std::collections::{BTreeMap, BTreeSet};

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
    pub(super) has_variable_schema: bool,
    pub(super) declared_variables: BTreeSet<VariableName>,
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
            has_variable_schema: false,
            declared_variables: BTreeSet::new(),
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

    /// Whether this header explicitly declares a variable schema, even if empty.
    #[must_use]
    pub const fn has_variable_schema(&self) -> bool {
        self.has_variable_schema
    }

    /// Borrow valid names explicitly declared in the variables mapping.
    #[must_use]
    pub fn declared_variables(&self) -> &BTreeSet<VariableName> {
        &self.declared_variables
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
    pub(super) required_variables: Declared<Vec<String>>,
    #[serde(default)]
    pub(super) variables: Declared<BTreeMap<String, RawVariable>>,
    #[serde(default)]
    pub(super) defaults: Declared<BTreeMap<String, serde_yaml::Value>>,
    #[serde(default)]
    pub(super) input_defaults: Declared<BTreeMap<String, serde_yaml::Value>>,
    #[serde(default)]
    pub(super) metadata: BTreeMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub(super) json_escape_mode: Option<JsonEscapeMode>,
}

#[derive(Debug)]
pub(super) struct Declared<T> {
    pub(super) present: bool,
    pub(super) value: T,
}

impl<T: Default> Default for Declared<T> {
    fn default() -> Self {
        Self {
            present: false,
            value: T::default(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Declared<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(|value| Self {
            present: true,
            value,
        })
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct RawVariable {
    #[serde(default)]
    pub(super) required: bool,
}
