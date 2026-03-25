//! Canonical crate-owned error families.

use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;

use crate::diagnostics::DiagnosticCode;
use crate::types::VariableName;

type BoxedError = Box<dyn StdError + Send + Sync + 'static>;

/// Structured recovery hint attached to configuration or validation failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryHint {
    /// Stable kind describing the hint payload.
    pub kind: RecoveryHintKind,
}

impl RecoveryHint {
    /// Create a structured recovery hint.
    #[must_use]
    pub const fn new(kind: RecoveryHintKind) -> Self {
        Self { kind }
    }
}

/// Structured recovery-hint payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryHintKind {
    /// Suggest a follow-up command.
    RunCommand {
        /// Command to execute.
        command: String,
    },
    /// Suggest reviewing a path.
    InspectPath {
        /// Path to inspect.
        path: PathBuf,
    },
    /// Suggest supplying a missing variable.
    ProvideVariable {
        /// Variable to provide.
        variable: VariableName,
    },
    /// Suggest correcting a configuration key.
    ReviewConfiguration {
        /// Configuration key to revisit.
        key: String,
    },
}

/// Canonical resolver error family.
#[derive(Debug)]
pub struct ResolveError {
    code: DiagnosticCode,
    message: String,
    attempted_paths: Vec<PathBuf>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ResolveError {
    /// Create a new resolver error without an underlying source.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        attempted_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            attempted_paths,
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Attach an underlying source error.
    #[must_use]
    pub fn with_source(
        mut self,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Return the stable diagnostic code when one is available.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        Some(self.code)
    }

    /// Return the attempted paths recorded for this failure.
    #[must_use]
    pub fn attempted_paths(&self) -> &[PathBuf] {
        &self.attempted_paths
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for ResolveError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|error| error as &(dyn StdError + 'static))
    }
}

/// Canonical include-processing error family.
#[derive(Debug)]
pub struct IncludeError {
    code: DiagnosticCode,
    message: String,
    include_chain: Vec<PathBuf>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl IncludeError {
    /// Create a new include error.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        include_chain: Vec<PathBuf>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            include_chain,
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Attach an underlying source error.
    #[must_use]
    pub fn with_source(
        mut self,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Return the stable diagnostic code when one is available.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        Some(self.code)
    }

    /// Return the include chain captured for the failure.
    #[must_use]
    pub fn include_chain(&self) -> &[PathBuf] {
        &self.include_chain
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for IncludeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for IncludeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|error| error as &(dyn StdError + 'static))
    }
}

/// Canonical validation error family.
#[derive(Debug)]
pub struct ValidationError {
    code: DiagnosticCode,
    message: String,
    recovery_hints: Vec<RecoveryHint>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ValidationError {
    /// Create a new validation error.
    #[must_use]
    pub fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            recovery_hints: Vec::new(),
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Create a duplicate-frontmatter validation error.
    #[must_use]
    pub fn duplicate_variable(variable: &VariableName) -> Self {
        Self::new(
            DiagnosticCode::ErrValDuplicate,
            format!("duplicate frontmatter variable declaration: {variable}"),
        )
    }

    /// Create a scalar-type validation error.
    #[must_use]
    pub fn invalid_scalar(message: impl Into<String>) -> Self {
        Self::new(DiagnosticCode::ErrValType, message)
    }

    /// Attach structured recovery hints.
    #[must_use]
    pub fn with_recovery_hints(mut self, recovery_hints: Vec<RecoveryHint>) -> Self {
        self.recovery_hints = recovery_hints;
        self
    }

    /// Attach an underlying source error.
    #[must_use]
    pub fn with_source(
        mut self,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Return the stable diagnostic code when one is available.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        Some(self.code)
    }

    /// Return structured recovery hints.
    #[must_use]
    pub fn recovery_hints(&self) -> &[RecoveryHint] {
        &self.recovery_hints
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for ValidationError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|error| error as &(dyn StdError + 'static))
    }
}

/// Canonical render error for template compilation and rendering failures.
///
/// This type is only constructed by the library; callers receive it as an
/// opaque error value by design.
#[derive(Debug)]
pub struct RenderError {
    source: BoxedError,
    backtrace: Backtrace,
}

impl RenderError {
    /// Construct a canonical render error from an underlying render cause.
    ///
    /// This constructor exists so the library can erase engine-specific error
    /// types at the public API boundary.
    #[must_use]
    pub fn render(source: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
            backtrace: Backtrace::capture(),
        }
    }

    /// Return the captured backtrace for the render failure.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    /// Render failures produced directly from the template engine do not have a
    /// stable `ERR_*` code in Sprint 2.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        None
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "template rendering failed: {}", self.source)
    }
}

impl StdError for RenderError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Canonical configuration and parsing error family.
#[derive(Debug)]
pub struct ConfigError {
    code: DiagnosticCode,
    message: String,
    recovery_hints: Vec<RecoveryHint>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ConfigError {
    /// Create a new configuration error.
    #[must_use]
    pub fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            recovery_hints: Vec::new(),
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Attach structured recovery hints.
    #[must_use]
    pub fn with_recovery_hints(mut self, recovery_hints: Vec<RecoveryHint>) -> Self {
        self.recovery_hints = recovery_hints;
        self
    }

    /// Attach an underlying source error.
    #[must_use]
    pub fn with_source(
        mut self,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Return the stable diagnostic code when one is available.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        Some(self.code)
    }

    /// Return structured recovery hints.
    #[must_use]
    pub fn recovery_hints(&self) -> &[RecoveryHint] {
        &self.recovery_hints
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for ConfigError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|error| error as &(dyn StdError + 'static))
    }
}

/// Top-level failure returned from compose, validate, and helper entry points.
#[derive(Debug)]
pub enum ComposeError {
    /// Profile or file resolution failed.
    Resolve(ResolveError),
    /// Include expansion failed.
    Include(IncludeError),
    /// Validation failed.
    Validation(ValidationError),
    /// Rendering failed.
    Render(RenderError),
    /// Configuration or parsing failed.
    Config(ConfigError),
}

impl ComposeError {
    /// Return the stable diagnostic code when one is available.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        match self {
            Self::Resolve(error) => error.code(),
            Self::Include(error) => error.code(),
            Self::Validation(error) => error.code(),
            Self::Render(error) => error.code(),
            Self::Config(error) => error.code(),
        }
    }
}

impl fmt::Display for ComposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolve(error) => fmt::Display::fmt(error, f),
            Self::Include(error) => fmt::Display::fmt(error, f),
            Self::Validation(error) => fmt::Display::fmt(error, f),
            Self::Render(error) => fmt::Display::fmt(error, f),
            Self::Config(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl StdError for ComposeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Resolve(error) => error.source(),
            Self::Include(error) => error.source(),
            Self::Validation(error) => error.source(),
            Self::Render(error) => error.source(),
            Self::Config(error) => error.source(),
        }
    }
}

impl From<ResolveError> for ComposeError {
    fn from(value: ResolveError) -> Self {
        Self::Resolve(value)
    }
}

impl From<IncludeError> for ComposeError {
    fn from(value: IncludeError) -> Self {
        Self::Include(value)
    }
}

impl From<ValidationError> for ComposeError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

impl From<RenderError> for ComposeError {
    fn from(value: RenderError) -> Self {
        Self::Render(value)
    }
}

impl From<ConfigError> for ComposeError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}
