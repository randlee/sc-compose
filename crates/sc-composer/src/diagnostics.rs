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
    /// Frontmatter declarations contained duplicate variables.
    ErrValDuplicate,
    /// A template body was empty when content was required.
    ErrValEmpty,
    /// A required variable was still missing after context merge.
    ErrValMissingRequired,
    /// A referenced token was not declared in frontmatter.
    ErrValUndeclaredToken,
    /// A caller-provided variable was not declared or referenced.
    ErrValExtraInput,
    /// The CLI attempted to read stdin twice for incompatible inputs.
    ErrRenderStdinDoubleRead,
    /// Output writing or materialization failed.
    ErrRenderWrite,
    /// A write was refused because the target was read-only.
    ErrConfigReadonly,
    /// Configuration or YAML parsing failed.
    ErrConfigParse,
    /// A var-file contained an unsupported structure.
    ErrConfigVarfile,
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
            Self::ErrValDuplicate => "ERR_VAL_DUPLICATE",
            Self::ErrValEmpty => "ERR_VAL_EMPTY",
            Self::ErrValMissingRequired => "ERR_VAL_MISSING_REQUIRED",
            Self::ErrValUndeclaredToken => "ERR_VAL_UNDECLARED_TOKEN",
            Self::ErrValExtraInput => "ERR_VAL_EXTRA_INPUT",
            Self::ErrRenderStdinDoubleRead => "ERR_RENDER_STDIN_DOUBLE_READ",
            Self::ErrRenderWrite => "ERR_RENDER_WRITE",
            Self::ErrConfigReadonly => "ERR_CONFIG_READONLY",
            Self::ErrConfigParse => "ERR_CONFIG_PARSE",
            Self::ErrConfigVarfile => "ERR_CONFIG_VARFILE",
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
