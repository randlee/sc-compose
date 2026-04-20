//! Structured diagnostics and stable `ERR_*` codes.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Stable schema version for diagnostics and machine-readable result payloads.
pub const DIAGNOSTIC_SCHEMA_VERSION: &str = "1";

/// Severity assigned to a diagnostic record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    /// Fatal condition.
    Error,
    /// Non-fatal condition requiring user attention.
    Warning,
    /// Informational diagnostic.
    Info,
}

/// Canonical stable diagnostic code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticCode {
    /// No matching template or profile was found.
    ErrResolveNotFound,
    /// Multiple matching templates or profiles were found.
    ErrResolveAmbiguous,
    /// An include path escaped the configured confinement root.
    ErrIncludeEscape,
    /// An include target could not be resolved.
    ErrIncludeNotFound,
    /// The include graph re-entered an active file, forming a cycle.
    ErrIncludeCycle,
    /// The include graph exceeded the configured maximum depth.
    ErrIncludeDepth,
    /// A variable had an invalid scalar type or shape.
    ErrValType,
    /// Structured object input used an unsupported shape.
    ErrValObjectShape,
    /// Structured input used a nested array shape that H2 still forbids.
    ErrValNestedArrayUnsupported,
    /// Frontmatter declarations contained duplicate variables.
    ErrValDuplicate,
    /// Frontmatter used both defaults sections and `input_defaults` overrides them.
    WarnValConflictingDefaultSections,
    /// A template body was empty when content was required.
    ErrValEmpty,
    /// The root template omitted a frontmatter block.
    ErrValMissingFrontmatter,
    /// A required variable was still missing after context merge.
    ErrValMissingRequired,
    /// A required nested field path was missing inside a present object.
    ErrValMissingNestedField,
    /// Nested required-path traversal encountered the wrong intermediate shape.
    ErrValShapeMismatch,
    /// A referenced token was not declared in frontmatter.
    ErrValUndeclaredToken,
    /// A caller-provided variable was not declared or referenced.
    ErrValExtraInput,
    /// A variable was not provided explicitly and a default value was used.
    InfoValDefaultUsed,
    /// The CLI attempted to read stdin twice for incompatible inputs.
    ErrRenderStdinDoubleRead,
    /// Output writing or materialization failed.
    ErrRenderWrite,
    /// A write was refused because the target was read-only.
    ErrConfigReadonly,
    /// A command or helper was invoked in an incompatible mode.
    ErrConfigMode,
    /// Configuration or YAML parsing failed.
    ErrConfigParse,
    /// A var-file contained an unsupported structure.
    ErrConfigVarfile,
    /// A named example or template pack could not be found.
    ErrConfigPackNotFound,
    /// A named template pack was not renderable by name.
    ErrConfigPackNotRenderable,
    /// A template import target already exists.
    ErrConfigTemplateExists,
}

impl DiagnosticCode {
    /// Return the stable string representation of the code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ErrResolveNotFound => "ERR_RESOLVE_NOT_FOUND",
            Self::ErrResolveAmbiguous => "ERR_RESOLVE_AMBIGUOUS",
            Self::ErrIncludeEscape => "ERR_INCLUDE_ESCAPE",
            Self::ErrIncludeNotFound => "ERR_INCLUDE_NOT_FOUND",
            Self::ErrIncludeCycle => "ERR_INCLUDE_CYCLE",
            Self::ErrIncludeDepth => "ERR_INCLUDE_DEPTH",
            Self::ErrValType => "ERR_VAL_TYPE",
            Self::ErrValObjectShape => "ERR_VAL_OBJECT_SHAPE",
            Self::ErrValNestedArrayUnsupported => "ERR_VAL_NESTED_ARRAY_UNSUPPORTED",
            Self::ErrValDuplicate => "ERR_VAL_DUPLICATE",
            Self::WarnValConflictingDefaultSections => "WARN_VAL_CONFLICTING_DEFAULT_SECTIONS",
            Self::ErrValEmpty => "ERR_VAL_EMPTY",
            Self::ErrValMissingFrontmatter => "ERR_VAL_MISSING_FRONTMATTER",
            Self::ErrValMissingRequired => "ERR_VAL_MISSING_REQUIRED",
            Self::ErrValMissingNestedField => "ERR_VAL_MISSING_NESTED_FIELD",
            Self::ErrValShapeMismatch => "ERR_VAL_SHAPE_MISMATCH",
            Self::ErrValUndeclaredToken => "ERR_VAL_UNDECLARED_TOKEN",
            Self::ErrValExtraInput => "ERR_VAL_EXTRA_INPUT",
            Self::InfoValDefaultUsed => "INFO_VAL_DEFAULT_USED",
            Self::ErrRenderStdinDoubleRead => "ERR_RENDER_STDIN_DOUBLE_READ",
            Self::ErrRenderWrite => "ERR_RENDER_WRITE",
            Self::ErrConfigReadonly => "ERR_CONFIG_READONLY",
            Self::ErrConfigMode => "ERR_CONFIG_MODE",
            Self::ErrConfigParse => "ERR_CONFIG_PARSE",
            Self::ErrConfigVarfile => "ERR_CONFIG_VARFILE",
            Self::ErrConfigPackNotFound => "ERR_CONFIG_PACK_NOT_FOUND",
            Self::ErrConfigPackNotRenderable => "ERR_CONFIG_PACK_NOT_RENDERABLE",
            Self::ErrConfigTemplateExists => "ERR_CONFIG_TEMPLATE_EXISTS",
        }
    }
}

/// Concrete diagnostic record emitted by the library.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable severity assigned to the record.
    pub severity: DiagnosticSeverity,
    /// Stable machine-readable code.
    pub code: DiagnosticCode,
    /// Human-readable message.
    pub message: String,
    /// Source path when known.
    pub path: Option<PathBuf>,
    /// One-based line number when known.
    pub line: Option<usize>,
    /// One-based column number when known.
    pub column: Option<usize>,
    /// Include chain involved in producing the diagnostic.
    pub include_chain: Vec<PathBuf>,
}

impl Diagnostic {
    /// Create a new diagnostic with the required stable fields.
    #[must_use]
    pub fn new(
        severity: DiagnosticSeverity,
        code: DiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            path: None,
            line: None,
            column: None,
            include_chain: Vec::new(),
        }
    }

    /// Attach a source path.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Attach a line and column location.
    #[must_use]
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Attach an include chain.
    #[must_use]
    pub fn with_include_chain(mut self, include_chain: Vec<PathBuf>) -> Self {
        self.include_chain = include_chain;
        self
    }
}

/// Versioned top-level diagnostics envelope used by JSON outputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEnvelope<T> {
    /// Stable schema version string.
    pub schema_version: String,
    /// Envelope payload supplied by the caller.
    pub payload: T,
    /// Diagnostics emitted alongside the payload.
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> DiagnosticEnvelope<T> {
    /// Create a versioned diagnostics envelope for a payload.
    #[must_use]
    pub fn new(payload: T, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            schema_version: DIAGNOSTIC_SCHEMA_VERSION.to_owned(),
            payload,
            diagnostics,
        }
    }
}
