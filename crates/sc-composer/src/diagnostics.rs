//! Structured diagnostics and stable `ERR_*` codes.

use std::fmt;
use std::path::{Path, PathBuf};

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

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        };
        f.write_str(severity)
    }
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
    /// An include target or resolved template path exists but could not be
    /// read due to filesystem permissions.
    ErrIncludePermissionDenied,
    /// An include directive resolved to a directory instead of a file.
    ErrIncludeIsADirectory,
    /// An include chain traversed a filesystem symlink loop.
    ErrIncludeFilesystemLoop,
    /// The include graph re-entered an active file, forming a cycle.
    ErrIncludeCycle,
    /// The include graph exceeded the configured maximum depth.
    ErrIncludeDepth,
    /// Structured object input used an unsupported shape.
    ErrValObjectShape,
    /// Legacy code reserved for a retired nested-array validation restriction.
    ///
    /// Current recursive structured-input validation does not emit this code
    /// for JSON/YAML-compatible values, but it remains part of the public
    /// diagnostic enum for compatibility.
    ErrValNestedArrayUnsupported,
    /// Frontmatter declarations contained duplicate variables.
    ErrValDuplicate,
    /// Frontmatter used both defaults sections and `input_defaults` overrides them.
    WarnValConflictingDefaultSections,
    /// `--all` was requested for a template without stacked headers.
    WarnConfigSinglePassAllFallback,
    /// A template uses a redundant frontmatter/YAML safety filter chain.
    WarnLintRedundantFilterChain,
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
    /// A required variable consumed by a bare for-loop was not an array.
    ErrValArrayShapeMismatch,
    /// A referenced token was not declared in frontmatter.
    ErrValUndeclaredToken,
    /// A caller-provided variable was not declared or referenced.
    ErrValExtraInput,
    /// A referenced variable had no value binding at render time.
    ErrValUnboundVariable,
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
    /// A configuration or text file could not be read as valid text.
    ErrConfigRead,
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
    /// An extraction request violates the in-memory contract.
    ErrExtractInvalidRequest,
    /// The rendered extraction input is malformed XML.
    ErrExtractMalformed,
    /// The template uses syntax outside the supported extraction subset.
    ErrExtractUnsupported,
    /// The extraction result has more than one structural interpretation.
    ErrExtractAmbiguous,
    /// A declared extraction occurrence was not observed in the rendered XML.
    WarnExtractNotObserved,
    /// Extraction evidence is insufficient for a high-confidence report.
    WarnExtractLowConfidence,
    /// Bytes were removed from an approved rendered XML preamble.
    WarnExtractDirtyPrefixStripped,
    /// The rendered JSON input is malformed.
    ErrExtractJsonMalformed,
    /// A JSON object contains a duplicate key.
    ErrExtractJsonDuplicateKey,
    /// A known-template JSON path is absent from the rendered value.
    ErrExtractJsonPathMissing,
    /// A rendered JSON value differs from the known-template structure.
    ErrExtractJsonShapeMismatch,
    /// A JSON placeholder occurs in an unsupported value shape.
    ErrExtractJsonValueUnsupported,
    /// A JSON variable occurs at multiple distinct paths.
    ErrExtractJsonAmbiguous,
    /// A template expression is outside the supported known-template subset.
    ErrExtractTemplateUnsupported,
    /// A rendered XML element name differs from the known template.
    ErrExtractXmlElementMismatch,
    /// A rendered XML attribute shape differs from the known template.
    ErrExtractXmlAttributeMismatch,
    /// A rendered XML child-node shape differs from the known template.
    ErrExtractXmlChildStructureMismatch,
    /// XML structural matching encountered a static-content mismatch.
    ErrExtractXmlStaticMismatch,
    /// XML control-flow syntax cannot be reversed by known-template matching.
    ErrExtractXmlControlFlowUnsupported,
    /// XML element names contain unsupported dynamic expressions.
    ErrExtractXmlDynamicElementName,
    /// XML namespaces are outside the supported unambiguous subset.
    ErrExtractXmlNamespaceUnsupported,
    /// The requested extraction format is not supported by this surface.
    ErrExtractFormatUnsupported,
    /// The rendered YAML is malformed or not one valid document.
    ErrExtractYamlMalformed,
    /// A YAML mapping repeats a key.
    ErrExtractYamlDuplicateKey,
    /// A YAML alias or anchor is outside the extraction contract.
    ErrExtractYamlAliasUnsupported,
    /// More than one YAML document was supplied.
    ErrExtractYamlDocumentStream,
    /// A known-template YAML path is absent from the rendered document.
    ErrExtractYamlPathMissing,
    /// The rendered YAML shape differs from the known template.
    ErrExtractYamlShapeMismatch,
    /// A YAML placeholder occurs outside a supported string scalar.
    ErrExtractYamlValueUnsupported,
    /// A YAML variable occurs at multiple distinct paths.
    ErrExtractYamlAmbiguous,
    /// The rendered TOML input is malformed.
    ErrExtractTomlMalformed,
    /// An extraction input exceeded a configured size, depth, or occurrence bound.
    ErrExtractInputLimit,
    /// A TOML document or table contains a duplicate key.
    ErrExtractTomlDuplicateKey,
    /// A known-template TOML path is absent from the rendered document.
    ErrExtractTomlPathMissing,
    /// The rendered TOML shape differs from the known template.
    ErrExtractTomlShapeMismatch,
    /// A TOML placeholder occurs outside a supported string value.
    ErrExtractTomlValueUnsupported,
    /// A TOML variable occurs at multiple distinct paths.
    ErrExtractTomlAmbiguous,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilesystemErrorClass {
    InvalidData,
    PermissionDenied,
    IsADirectory,
    FilesystemLoop,
    NotFound,
}

pub(crate) fn classify_filesystem_error(
    path: &Path,
    error: &std::io::Error,
) -> FilesystemErrorClass {
    if path.is_dir() {
        return FilesystemErrorClass::IsADirectory;
    }

    match error.kind() {
        std::io::ErrorKind::InvalidData => FilesystemErrorClass::InvalidData,
        std::io::ErrorKind::PermissionDenied => FilesystemErrorClass::PermissionDenied,
        std::io::ErrorKind::IsADirectory => FilesystemErrorClass::IsADirectory,
        _ if is_filesystem_loop(error) => FilesystemErrorClass::FilesystemLoop,
        _ => FilesystemErrorClass::NotFound,
    }
}

fn is_filesystem_loop(error: &std::io::Error) -> bool {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        error.raw_os_error() == Some(40)
    }

    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        error.raw_os_error() == Some(62)
    }

    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(1142 | 1921))
    }

    #[cfg(not(any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "windows"
    )))]
    {
        false
    }
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
            Self::ErrIncludePermissionDenied => "ERR_INCLUDE_PERMISSION_DENIED",
            Self::ErrIncludeIsADirectory => "ERR_INCLUDE_IS_A_DIRECTORY",
            Self::ErrIncludeFilesystemLoop => "ERR_INCLUDE_FILESYSTEM_LOOP",
            Self::ErrIncludeCycle => "ERR_INCLUDE_CYCLE",
            Self::ErrIncludeDepth => "ERR_INCLUDE_DEPTH",
            Self::ErrValObjectShape => "ERR_VAL_OBJECT_SHAPE",
            Self::ErrValNestedArrayUnsupported => "ERR_VAL_NESTED_ARRAY_UNSUPPORTED",
            Self::ErrValDuplicate => "ERR_VAL_DUPLICATE",
            Self::WarnValConflictingDefaultSections => "WARN_VAL_CONFLICTING_DEFAULT_SECTIONS",
            Self::WarnConfigSinglePassAllFallback => "WARN_CONFIG_SINGLE_PASS_ALL_FALLBACK",
            Self::WarnLintRedundantFilterChain => "WARN_LINT_REDUNDANT_FILTER_CHAIN",
            Self::ErrValEmpty => "ERR_VAL_EMPTY",
            Self::ErrValMissingFrontmatter => "ERR_VAL_MISSING_FRONTMATTER",
            Self::ErrValMissingRequired => "ERR_VAL_MISSING_REQUIRED",
            Self::ErrValMissingNestedField => "ERR_VAL_MISSING_NESTED_FIELD",
            Self::ErrValShapeMismatch => "ERR_VAL_SHAPE_MISMATCH",
            Self::ErrValArrayShapeMismatch => "ERR_VAL_ARRAY_SHAPE_MISMATCH",
            Self::ErrValUndeclaredToken => "ERR_VAL_UNDECLARED_TOKEN",
            Self::ErrValExtraInput => "ERR_VAL_EXTRA_INPUT",
            Self::ErrValUnboundVariable => "ERR_VAL_UNBOUND_VARIABLE",
            Self::InfoValDefaultUsed => "INFO_VAL_DEFAULT_USED",
            Self::ErrRenderStdinDoubleRead => "ERR_RENDER_STDIN_DOUBLE_READ",
            Self::ErrRenderWrite => "ERR_RENDER_WRITE",
            Self::ErrConfigReadonly => "ERR_CONFIG_READONLY",
            Self::ErrConfigMode => "ERR_CONFIG_MODE",
            Self::ErrConfigRead => "ERR_CONFIG_READ",
            Self::ErrConfigParse => "ERR_CONFIG_PARSE",
            Self::ErrConfigVarfile => "ERR_CONFIG_VARFILE",
            Self::ErrConfigPackNotFound => "ERR_CONFIG_PACK_NOT_FOUND",
            Self::ErrConfigPackNotRenderable => "ERR_CONFIG_PACK_NOT_RENDERABLE",
            Self::ErrConfigTemplateExists => "ERR_CONFIG_TEMPLATE_EXISTS",
            Self::ErrExtractInvalidRequest => "ERR_EXTRACT_INVALID_REQUEST",
            Self::ErrExtractMalformed => "ERR_EXTRACT_MALFORMED",
            Self::ErrExtractUnsupported => "ERR_EXTRACT_UNSUPPORTED",
            Self::ErrExtractAmbiguous => "ERR_EXTRACT_AMBIGUOUS",
            Self::WarnExtractNotObserved => "WARN_EXTRACT_NOT_OBSERVED",
            Self::WarnExtractLowConfidence => "WARN_EXTRACT_LOW_CONFIDENCE",
            Self::WarnExtractDirtyPrefixStripped => "WARN_EXTRACT_DIRTY_PREFIX_STRIPPED",
            Self::ErrExtractJsonMalformed => "ERR_EXTRACT_JSON_MALFORMED",
            Self::ErrExtractJsonDuplicateKey => "ERR_EXTRACT_JSON_DUPLICATE_KEY",
            Self::ErrExtractJsonPathMissing => "ERR_EXTRACT_JSON_PATH_MISSING",
            Self::ErrExtractJsonShapeMismatch => "ERR_EXTRACT_JSON_SHAPE_MISMATCH",
            Self::ErrExtractJsonValueUnsupported => "ERR_EXTRACT_JSON_VALUE_UNSUPPORTED",
            Self::ErrExtractJsonAmbiguous => "ERR_EXTRACT_JSON_AMBIGUOUS",
            Self::ErrExtractTemplateUnsupported => "ERR_EXTRACT_TEMPLATE_UNSUPPORTED",
            Self::ErrExtractXmlElementMismatch => "ERR_EXTRACT_XML_ELEMENT_MISMATCH",
            Self::ErrExtractXmlAttributeMismatch => "ERR_EXTRACT_XML_ATTRIBUTE_MISMATCH",
            Self::ErrExtractXmlChildStructureMismatch => "ERR_EXTRACT_XML_CHILD_STRUCTURE_MISMATCH",
            Self::ErrExtractXmlStaticMismatch => "ERR_EXTRACT_XML_STATIC_MISMATCH",
            Self::ErrExtractXmlControlFlowUnsupported => "ERR_EXTRACT_XML_CONTROL_FLOW_UNSUPPORTED",
            Self::ErrExtractXmlDynamicElementName => "ERR_EXTRACT_XML_DYNAMIC_ELEMENT_NAME",
            Self::ErrExtractXmlNamespaceUnsupported => "ERR_EXTRACT_XML_NAMESPACE_UNSUPPORTED",
            Self::ErrExtractFormatUnsupported => "ERR_EXTRACT_FORMAT_UNSUPPORTED",
            Self::ErrExtractYamlMalformed => "ERR_EXTRACT_YAML_MALFORMED",
            Self::ErrExtractYamlDuplicateKey => "ERR_EXTRACT_YAML_DUPLICATE_KEY",
            Self::ErrExtractYamlAliasUnsupported => "ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED",
            Self::ErrExtractYamlDocumentStream => "ERR_EXTRACT_YAML_DOCUMENT_STREAM",
            Self::ErrExtractYamlPathMissing => "ERR_EXTRACT_YAML_PATH_MISSING",
            Self::ErrExtractYamlShapeMismatch => "ERR_EXTRACT_YAML_SHAPE_MISMATCH",
            Self::ErrExtractYamlValueUnsupported => "ERR_EXTRACT_YAML_VALUE_UNSUPPORTED",
            Self::ErrExtractYamlAmbiguous => "ERR_EXTRACT_YAML_AMBIGUOUS",
            Self::ErrExtractTomlMalformed => "ERR_EXTRACT_TOML_MALFORMED",
            Self::ErrExtractInputLimit => "ERR_EXTRACT_INPUT_LIMIT",
            Self::ErrExtractTomlDuplicateKey => "ERR_EXTRACT_TOML_DUPLICATE_KEY",
            Self::ErrExtractTomlPathMissing => "ERR_EXTRACT_TOML_PATH_MISSING",
            Self::ErrExtractTomlShapeMismatch => "ERR_EXTRACT_TOML_SHAPE_MISMATCH",
            Self::ErrExtractTomlValueUnsupported => "ERR_EXTRACT_TOML_VALUE_UNSUPPORTED",
            Self::ErrExtractTomlAmbiguous => "ERR_EXTRACT_TOML_AMBIGUOUS",
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
