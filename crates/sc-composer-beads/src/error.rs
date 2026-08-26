//! Stable Beads composition failures.

use std::path::PathBuf;

use thiserror::Error;

/// Stable errors returned before or during Beads composition.
#[derive(Debug, Error)]
pub enum BeadComposeError {
    /// The JSON request did not deserialize into the versioned contract.
    #[error("invalid Beads composition request: {message}")]
    RequestDeserializationFailed {
        /// Serializer diagnostic retained for callers and logs.
        message: String,
    },
    /// The request selected an unsupported protocol schema.
    #[error("unsupported Beads composition schema `{actual}`")]
    UnknownSchema {
        /// Unsupported schema value supplied by the caller.
        actual: String,
    },
    /// A formula path was not a regular file.
    #[error("formula path is not a file: {path}")]
    FormulaPathNotFile {
        /// Path that did not resolve to a regular file.
        path: PathBuf,
    },
    /// A formula path has an unsupported extension.
    #[error("unsupported formula extension: {path}")]
    FormulaExtensionUnsupported {
        /// Path whose formula extension is unsupported.
        path: PathBuf,
    },
    /// A template path could not be normalized safely.
    #[error("invalid template path: {path}")]
    TemplatePathInvalid {
        /// Template or output path that could not be normalized safely.
        path: PathBuf,
    },
    /// A template path escaped the working directory.
    #[error("template path escapes working directory: {path}")]
    TemplateOutsideWorkingDirectory {
        /// Canonical template path outside the configured working directory.
        path: PathBuf,
    },
    /// A rendered output path escaped the working directory.
    #[error("output path escapes working directory: {path}")]
    OutputOutsideWorkingDirectory {
        /// Canonical ordinary output path outside the configured working directory.
        path: PathBuf,
    },
    /// A Beads variable key is malformed.
    #[error("invalid Beads variable key `{key}`")]
    BeadVariableKeyInvalid {
        /// Malformed runtime-variable key.
        key: String,
    },
    /// A Beads variable key was supplied more than once.
    #[error("duplicate Beads variable key `{key}`")]
    BeadVariableKeyDuplicate {
        /// Duplicate runtime-variable key.
        key: String,
    },
    /// Preview or persistent pour omitted the formula name.
    #[error("formula name is required for pour operations")]
    FormulaNameRequired,
    /// Persistent pour omitted the required authorization sentinel.
    #[error("persistent Beads creation requires explicit authorization")]
    PourAuthorizationRequired,
    /// Persistent pour supplied an unsupported authorization value.
    #[error("persistent Beads creation authorization is invalid")]
    PourAuthorizationInvalid,
    /// The configured `bd` executable could not be started.
    #[error("Beads executable is unavailable: {executable}")]
    BdUnavailable {
        /// Configured executable that could not be started.
        executable: PathBuf,
    },
    /// Formula rendering failed before Beads validation.
    #[error("formula rendering failed: {message}")]
    RenderFailed {
        /// Rendering failure details retained for diagnostics.
        message: String,
    },
    /// `bd cook --dry-run` failed.
    #[error("Beads formula validation failed")]
    CookFailed {
        /// Exit status returned by `bd cook`, if it started.
        exit_status: Option<i32>,
    },
    /// `bd where --json` failed or returned unusable output.
    #[error("active Beads registry resolution failed")]
    ActiveRegistryResolutionFailed {
        /// Exit status returned by `bd where`, if it started.
        exit_status: Option<i32>,
    },
    /// The rendered formula did not belong to the active registry.
    #[error("formula is outside the active Beads registry: {path}")]
    FormulaOutsideActiveRegistry {
        /// Rendered formula path outside the active Beads registry.
        path: PathBuf,
    },
    /// Both TOML and JSON formulas exist for the requested name.
    #[error("active Beads registry has ambiguous formula `{formula_name}`")]
    FormulaRegistryAmbiguous {
        /// Formula name with both TOML and JSON entries in the active registry.
        formula_name: String,
    },
    /// `bd mol pour --dry-run` failed.
    #[error("Beads pour preview failed")]
    PreviewPourFailed {
        /// Exit status returned by preview `bd mol pour`, if it started.
        exit_status: Option<i32>,
    },
    /// Authorized persistent `bd mol pour` failed.
    #[error("persistent Beads pour failed")]
    PourFailed {
        /// Exit status returned by persistent `bd mol pour`, if it started.
        exit_status: Option<i32>,
    },
}

impl BeadComposeError {
    /// Return the stable protocol code for this condition.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RequestDeserializationFailed { .. } => "BEADS_REQUEST_DESERIALIZATION_FAILED",
            Self::UnknownSchema { .. } => "BEADS_UNKNOWN_SCHEMA",
            Self::FormulaPathNotFile { .. } => "BEADS_FORMULA_NOT_FILE",
            Self::FormulaExtensionUnsupported { .. } => "BEADS_FORMULA_EXTENSION_UNSUPPORTED",
            Self::TemplatePathInvalid { .. } => "BEADS_TEMPLATE_PATH_INVALID",
            Self::TemplateOutsideWorkingDirectory { .. } => "BEADS_TEMPLATE_OUTSIDE_WORKING_DIR",
            Self::OutputOutsideWorkingDirectory { .. } => "BEADS_OUTPUT_OUTSIDE_WORKING_DIR",
            Self::BeadVariableKeyInvalid { .. } => "BEADS_VARIABLE_KEY_INVALID",
            Self::BeadVariableKeyDuplicate { .. } => "BEADS_VARIABLE_KEY_DUPLICATE",
            Self::FormulaNameRequired => "BEADS_FORMULA_NAME_REQUIRED",
            Self::PourAuthorizationRequired => "BEADS_POUR_AUTH_REQUIRED",
            Self::PourAuthorizationInvalid => "BEADS_POUR_AUTH_INVALID",
            Self::BdUnavailable { .. } => "BEADS_BD_UNAVAILABLE",
            Self::RenderFailed { .. } => "BEADS_RENDER_FAILED",
            Self::CookFailed { .. } => "BEADS_COOK_FAILED",
            Self::ActiveRegistryResolutionFailed { .. } => "BEADS_WHERE_FAILED",
            Self::FormulaOutsideActiveRegistry { .. } => "BEADS_FORMULA_OUTSIDE_ACTIVE_REGISTRY",
            Self::FormulaRegistryAmbiguous { .. } => "BEADS_FORMULA_REGISTRY_AMBIGUOUS",
            Self::PreviewPourFailed { .. } => "BEADS_PREVIEW_POUR_FAILED",
            Self::PourFailed { .. } => "BEADS_POUR_FAILED",
        }
    }
}
