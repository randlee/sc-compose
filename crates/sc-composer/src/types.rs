//! Foundational public value, request, and result types for `sc-composer`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::diagnostics::Diagnostic;
use crate::diagnostics::DiagnosticCode;

/// Caller-provided render input value.
pub type InputValue = serde_json::Value;

/// Default pass number applied to frontmatter and pass-policy entries.
#[must_use]
pub const fn default_pass_number() -> u8 {
    1
}

fn deserialize_pass_number<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let pass_number = u8::deserialize(deserializer)?;
    Ok(if pass_number == 0 {
        default_pass_number()
    } else {
        pass_number
    })
}

/// Validate a caller-provided input value against the supported render-context
/// model.
///
/// # Errors
///
/// Returns [`InvalidInputValueError`] when the value contains a shape that
/// cannot be represented by the structured-input contract.
pub fn validate_input_value(value: &InputValue) -> Result<(), InvalidInputValueError> {
    validate_input_value_at(value)
}

fn validate_input_value_at(value: &InputValue) -> Result<(), InvalidInputValueError> {
    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => Ok(()),
        serde_json::Value::Array(values) => values.iter().try_for_each(validate_input_value_at),
        serde_json::Value::Object(object) => object.values().try_for_each(validate_input_value_at),
    }
}

/// Convert a YAML value into a supported caller-provided input value.
///
/// # Errors
///
/// Returns [`InvalidInputValueError`] when the YAML value uses non-string
/// object keys or contains an unsupported scalar representation.
pub fn input_value_from_yaml(
    value: serde_yaml::Value,
) -> Result<InputValue, InvalidInputValueError> {
    match value {
        serde_yaml::Value::Null => Ok(serde_json::Value::Null),
        serde_yaml::Value::Bool(value) => Ok(serde_json::Value::Bool(value)),
        serde_yaml::Value::Number(value) => {
            match serde_json::from_str::<serde_json::Value>(&value.to_string()) {
                Ok(serde_json::Value::Number(number)) => Ok(serde_json::Value::Number(number)),
                _ => Err(InvalidInputValueError::new(
                    DiagnosticCode::ErrValObjectShape,
                    "expected a JSON-compatible number",
                )),
            }
        }
        serde_yaml::Value::String(value) => Ok(serde_json::Value::String(value)),
        serde_yaml::Value::Sequence(values) => values
            .into_iter()
            .map(input_value_from_yaml)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array)
            .and_then(|value| {
                validate_input_value(&value)?;
                Ok(value)
            }),
        serde_yaml::Value::Tagged(tagged) => input_value_from_yaml(tagged.value),
        serde_yaml::Value::Mapping(mapping) => mapping
            .into_iter()
            .map(|(key, value)| {
                let serde_yaml::Value::String(key) = key else {
                    return Err(InvalidInputValueError::new(
                        DiagnosticCode::ErrValObjectShape,
                        "expected object/map keys to be strings",
                    ));
                };
                let value = input_value_from_yaml(value)?;
                Ok((key, value))
            })
            .collect::<Result<serde_json::Map<String, serde_json::Value>, _>>()
            .map(serde_json::Value::Object)
            .and_then(|value| {
                validate_input_value(&value)?;
                Ok(value)
            }),
    }
}

/// Arbitrary metadata value used only for descriptive frontmatter fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetadataValue(serde_yaml::Value);

impl MetadataValue {
    /// Create a metadata value from an internal YAML representation.
    pub(crate) fn new(value: serde_yaml::Value) -> Self {
        Self(value)
    }

    /// Serialize the metadata value into a JSON-compatible representation.
    ///
    /// # Errors
    ///
    /// Returns [`serde_json::Error`] when the metadata value cannot be
    /// represented as JSON.
    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(&self.0)
    }
}

/// Validated variable identifier used in the public API.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VariableName(String);

impl VariableName {
    /// Create a validated variable name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidVariableNameError`] when the name is empty or contains
    /// characters outside the supported identifier set.
    pub fn new(name: impl Into<String>) -> Result<Self, InvalidVariableNameError> {
        let name = name.into();
        let is_valid = !name.is_empty()
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));

        if is_valid {
            Ok(Self(name))
        } else {
            Err(InvalidVariableNameError { name })
        }
    }

    /// Borrow the validated variable name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VariableName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for VariableName {
    type Error = InvalidVariableNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for VariableName {
    type Error = InvalidVariableNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Non-negative include depth bound.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct IncludeDepth(u16);

impl IncludeDepth {
    /// Create a bounded include depth value.
    #[must_use]
    pub const fn new(depth: u16) -> Self {
        Self(depth)
    }

    /// Return the underlying numeric depth bound.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Canonicalized root path used for confinement checks.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfiningRoot(PathBuf);

impl ConfiningRoot {
    /// Canonicalize and validate a confinement root.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] when the path cannot be canonicalized.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        std::fs::canonicalize(path).map(Self)
    }

    /// Create a confinement root from an already-canonical path.
    #[must_use]
    pub fn from_path_buf(path: PathBuf) -> Self {
        Self(path)
    }

    /// Borrow the confinement root as a path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Consume the newtype and return the inner path buffer.
    #[must_use]
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

/// Runtime family used for profile resolution policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    /// Claude runtime layout.
    Claude,
    /// Codex runtime layout.
    Codex,
    /// Gemini runtime layout.
    Gemini,
    /// `OpenCode` runtime layout.
    Opencode,
}

/// Logical profile kind resolved by shared or runtime-specific prompt lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileKind {
    /// Agent prompt profile.
    Agent,
    /// Command prompt profile.
    Command,
    /// Skill prompt profile.
    Skill,
}

/// Validated profile identifier used by profile-mode resolution.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileName(String);

impl ProfileName {
    /// Create a validated profile name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidProfileNameError`] when the name is empty or contains
    /// path separators.
    pub fn new(name: impl Into<String>) -> Result<Self, InvalidProfileNameError> {
        let name = name.into();
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            Err(InvalidProfileNameError { name })
        } else {
            Ok(Self(name))
        }
    }

    /// Borrow the validated profile name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ProfileName {
    type Error = InvalidProfileNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ProfileName {
    type Error = InvalidProfileNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
/// Variant-specific composition mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComposeMode {
    /// Profile-mode composition by kind and profile name.
    Profile {
        /// The logical profile kind being resolved.
        kind: ProfileKind,
        /// The profile name to resolve.
        name: ProfileName,
    },
    /// File-mode composition from an explicit template path.
    File {
        /// Path to the template file to compose.
        template_path: PathBuf,
    },
}

/// Policy values used for variable diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnknownVariablePolicy {
    /// Treat extra variables as a hard error.
    Error,
    /// Emit warnings for extra variables.
    Warn,
    /// Ignore extra variables.
    #[default]
    Ignore,
}

/// Data-driven resolver policy placeholder carried through request types.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolverPolicy {
    /// Ordered candidate directories used for resolution.
    pub candidate_directories: Vec<PathBuf>,
    /// Ordered filename probes used within each candidate directory.
    pub filename_probes: Vec<String>,
    /// Whether ambiguity without an explicit runtime is treated as an error.
    pub ambiguous_without_runtime_is_error: bool,
}

/// Per-pass configuration carried through the multi-pass composition pipeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PassConfig {
    /// The pass number this configuration applies to.
    #[serde(
        default = "default_pass_number",
        deserialize_with = "deserialize_pass_number"
    )]
    pub pass_number: u8,
    /// Variables required before this pass can render.
    pub required_variables: Vec<VariableName>,
    /// Default values injected for this pass.
    pub defaults: BTreeMap<VariableName, InputValue>,
    /// Descriptive metadata associated with this pass.
    #[serde(default)]
    pub metadata: BTreeMap<String, MetadataValue>,
}

impl Default for PassConfig {
    fn default() -> Self {
        Self {
            pass_number: default_pass_number(),
            required_variables: Vec::new(),
            defaults: BTreeMap::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Policy bundle for the composition pipeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComposePolicy {
    /// Whether undeclared referenced variables are fatal.
    pub strict_undeclared_variables: bool,
    /// How to handle caller-provided variables that are not declared or used.
    pub unknown_variable_policy: UnknownVariablePolicy,
    /// How to handle referenced variables with no value at render time.
    ///
    /// `None` preserves compatibility with callers that only set
    /// `unknown_variable_policy`: the effective unbound-variable policy then
    /// inherits that value. Set this explicitly when the two diagnostic axes
    /// need different behavior.
    #[serde(default)]
    pub unbound_variable_policy: Option<UnknownVariablePolicy>,
    /// Maximum include depth allowed by the pipeline.
    pub max_include_depth: IncludeDepth,
    /// Allowed confinement roots for file access.
    pub allowed_roots: Vec<ConfiningRoot>,
    /// Resolver configuration carried into later sprints.
    pub resolver_policy: ResolverPolicy,
    /// Per-pass policy extensions for multi-pass templates.
    pub passes: Vec<PassConfig>,
}

impl Default for ComposePolicy {
    fn default() -> Self {
        Self {
            strict_undeclared_variables: false,
            unknown_variable_policy: UnknownVariablePolicy::Ignore,
            unbound_variable_policy: None,
            max_include_depth: IncludeDepth::new(32),
            allowed_roots: Vec::new(),
            resolver_policy: ResolverPolicy::default(),
            passes: Vec::new(),
        }
    }
}

/// Top-level library request for compose and validate entry points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComposeRequest {
    /// Optional runtime used for profile resolution policy.
    pub runtime: Option<RuntimeKind>,
    /// Variant-specific composition mode.
    pub mode: ComposeMode,
    /// Confinement root for template resolution and include checks.
    pub root: ConfiningRoot,
    /// Explicit caller-provided variables.
    pub vars_input: BTreeMap<VariableName, InputValue>,
    /// Environment-derived variables.
    pub vars_env: BTreeMap<VariableName, InputValue>,
    /// Pack-level default variables supplied by the CLI layer.
    pub vars_defaults: BTreeMap<VariableName, InputValue>,
    /// Optional guidance block appended by higher-level callers.
    pub guidance_block: Option<String>,
    /// Optional user prompt block appended by higher-level callers.
    pub user_prompt: Option<String>,
    /// Validation and resolution policy bundle.
    pub policy: ComposePolicy,
}

/// Source trace for a resolved variable value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableSource {
    /// Explicit caller-supplied input.
    ExplicitInput,
    /// Environment-derived input.
    Environment,
    /// Built-in render-context variable injected by the runtime.
    Builtin,
    /// Default supplied by a user template pack.
    TemplateInputDefault,
    /// Default declared in the root document frontmatter.
    FrontmatterDefault,
    /// Default declared in an included document frontmatter.
    IncludedDefault,
}

/// Resolve trace returned from profile or file resolution.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveResult {
    /// The final resolved template path.
    pub resolved_path: PathBuf,
    /// All attempted candidate paths.
    pub attempted_paths: Vec<PathBuf>,
    /// Ambiguity candidates collected during resolution.
    pub ambiguity_candidates: Vec<PathBuf>,
}

/// Final successful composition result.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComposeResult {
    /// Final rendered text output.
    pub rendered_text: String,
    /// Files resolved during composition.
    pub resolved_files: Vec<PathBuf>,
    /// Detailed resolve trace.
    pub resolve_result: ResolveResult,
    /// Provenance for resolved variable values.
    pub variable_sources: BTreeMap<VariableName, VariableSource>,
    /// Non-fatal diagnostics emitted during composition.
    pub warnings: Vec<Diagnostic>,
}

/// Structured validation result without rendered output.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether the validation phase completed without errors.
    pub ok: bool,
    /// Non-fatal diagnostics emitted during validation.
    pub warnings: Vec<Diagnostic>,
    /// Fatal diagnostics emitted during validation.
    pub errors: Vec<Diagnostic>,
    /// Detailed resolve trace.
    pub resolve_result: ResolveResult,
}

/// Result returned by the future `frontmatter-init` helper.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontmatterInitResult {
    /// Target file to rewrite or materialize.
    pub target_path: PathBuf,
    /// Frontmatter text that would be written.
    pub frontmatter_text: String,
    /// Full template text that would be written.
    pub template_text: String,
    /// Variables discovered during analysis.
    pub discovered_variables: Vec<VariableName>,
    /// Whether the target file changed on disk.
    pub changed: bool,
    /// Whether the operation would rewrite the target if allowed to write.
    pub would_change: bool,
}

/// Result returned by template drift verification.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VerifyResult {
    /// Whether the rendered template matches the deployed file exactly.
    pub clean: bool,
    /// Final resolved template path used for rendering.
    pub resolved_template_path: PathBuf,
    /// Deployed file compared against the rendered template output.
    pub deployed_path: PathBuf,
    /// Rendered template output used for comparison.
    pub rendered_text: String,
    /// Concrete deployed file contents.
    pub deployed_text: String,
    /// Unified diff when drift is detected.
    pub diff: Option<String>,
    /// Non-fatal diagnostics emitted while rendering the template.
    pub warnings: Vec<Diagnostic>,
}

/// Result returned by the future `init` helper.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InitResult {
    /// Path to the prompts directory.
    pub prompts_dir: PathBuf,
    /// Whether `.gitignore` was updated.
    pub gitignore_updated: bool,
    /// Templates scanned during initialization.
    pub scanned_templates: Vec<PathBuf>,
    /// Recommendations emitted during scanning.
    pub recommendations: Vec<Diagnostic>,
    /// Whether scanned templates passed validation.
    pub validation_passed: bool,
}

/// Error returned when a render-context value uses an unsupported input shape.
#[derive(Clone, Debug, thiserror::Error)]
#[error("{message}")]
pub struct InvalidInputValueError {
    code: DiagnosticCode,
    message: String,
}

impl InvalidInputValueError {
    /// Create a new input-value validation error.
    #[must_use]
    pub fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Return the stable diagnostic code describing the invalid shape.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Return the human-readable error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Error returned when a variable name fails public API validation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid variable name `{name}`")]
pub struct InvalidVariableNameError {
    name: String,
}

impl InvalidVariableNameError {
    /// Return the rejected variable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Error returned when a profile name fails public API validation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid profile name `{name}`")]
pub struct InvalidProfileNameError {
    name: String,
}

impl InvalidProfileNameError {
    /// Return the rejected profile name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use serde_yaml::from_str;

    use crate::DiagnosticCode;

    use super::{
        PassConfig, ProfileName, VariableName, default_pass_number, input_value_from_yaml,
        validate_input_value,
    };

    #[test]
    fn variable_name_round_trips_for_valid_identifier() {
        let variable = VariableName::new("profile.name_1").unwrap();
        assert_eq!(variable.as_str(), "profile.name_1");
        assert_eq!(variable.to_string(), "profile.name_1");
    }

    #[test]
    fn variable_name_rejects_empty_string() {
        let error = VariableName::new("").unwrap_err();
        assert_eq!(error.name(), "");
    }

    #[test]
    fn variable_name_rejects_invalid_characters() {
        let error = VariableName::new("bad name!").unwrap_err();
        assert_eq!(error.name(), "bad name!");
    }

    #[test]
    fn variable_name_display_matches_inner_string() {
        let variable = VariableName::new("agent.name").unwrap();
        assert_eq!(format!("{variable}"), "agent.name");
    }

    #[test]
    fn validate_input_value_accepts_serde_json_object() {
        let value = json!({
            "pr": {
                "number": 43,
                "url": "https://example.test/pr/43",
                "labels": ["bug", "release"]
            }
        });

        validate_input_value(&value).unwrap();
    }

    #[test]
    fn input_value_from_yaml_mapping_becomes_object() {
        let yaml = from_str::<serde_yaml::Value>(
            "pr:\n  number: 43\n  url: https://example.test/pr/43\n  labels:\n    - bug\n    - release\n",
        )
        .unwrap();

        let value = input_value_from_yaml(yaml).unwrap();

        assert_eq!(
            value,
            json!({
                "pr": {
                    "number": 43,
                    "url": "https://example.test/pr/43",
                    "labels": ["bug", "release"]
                }
            })
        );
    }

    #[test]
    fn validate_input_value_accepts_array_of_objects() {
        let value = json!([
            {
                "id": "S1",
                "stage": "qa",
                "pr": { "number": 43 }
            },
            {
                "id": "S2",
                "stage": "merged",
                "pr": { "number": 44 }
            }
        ]);

        validate_input_value(&value).unwrap();
    }

    #[test]
    fn input_value_from_yaml_sequence_of_objects_becomes_array() {
        let yaml = from_str::<serde_yaml::Value>(
            "- id: S1\n  stage: qa\n  pr:\n    number: 43\n- id: S2\n  stage: merged\n  pr:\n    number: 44\n",
        )
        .unwrap();

        let value = input_value_from_yaml(yaml).unwrap();

        assert_eq!(
            value,
            json!([
                { "id": "S1", "stage": "qa", "pr": { "number": 43 } },
                { "id": "S2", "stage": "merged", "pr": { "number": 44 } }
            ])
        );
    }

    #[test]
    fn validate_input_value_accepts_recursive_arrays_and_objects() {
        let value = json!({
            "categories": [
                {
                    "name": "Added",
                    "items": [
                        {"summary": "new feature", "prs": [588, 589]},
                        {"summary": "documentation", "prs": []}
                    ]
                }
            ],
            "rows": [[1, 2, 3], [4, 5]],
            "mixed": [null, true, {"nested": [["value"]]}]
        });

        validate_input_value(&value).unwrap();
    }

    #[test]
    fn input_value_from_yaml_accepts_recursive_arrays_and_objects() {
        let yaml = from_str::<serde_yaml::Value>(
            "categories:\n  - name: Added\n    items:\n      - summary: new feature\n        prs: [588, 589]\nrows:\n  - [1, 2, 3]\n  - [4, 5]\n",
        )
        .unwrap();

        let value = input_value_from_yaml(yaml).unwrap();

        assert_eq!(
            value,
            json!({
                "categories": [{
                    "name": "Added",
                    "items": [{"summary": "new feature", "prs": [588, 589]}]
                }],
                "rows": [[1, 2, 3], [4, 5]]
            })
        );
    }

    #[test]
    fn input_value_from_yaml_rejects_non_string_map_keys() {
        let yaml = from_str::<serde_yaml::Value>("1: invalid-key\n").unwrap();

        let error = input_value_from_yaml(yaml).unwrap_err();

        assert_eq!(error.code(), DiagnosticCode::ErrValObjectShape);
    }

    #[test]
    fn profile_name_round_trips_for_valid_identifier() {
        let profile = ProfileName::new("agent-name").unwrap();
        assert_eq!(profile.as_str(), "agent-name");
        assert_eq!(profile.to_string(), "agent-name");
    }

    #[test]
    fn profile_name_rejects_empty_string() {
        let error = ProfileName::new("").unwrap_err();
        assert_eq!(error.name(), "");
    }

    #[test]
    fn profile_name_rejects_path_separators() {
        let error = ProfileName::new("agent/name").unwrap_err();
        assert_eq!(error.name(), "agent/name");
    }

    #[test]
    fn pass_config_default_pass_number_matches_frontmatter_default() {
        assert_eq!(PassConfig::default().pass_number, default_pass_number());
    }

    #[test]
    fn pass_config_explicit_zero_normalizes_to_default_pass_number() {
        let pass_config = serde_yaml::from_str::<PassConfig>(
            "pass_number: 0\nrequired_variables: []\ndefaults: {}\n",
        )
        .unwrap();

        assert_eq!(pass_config.pass_number, default_pass_number());
    }
}
