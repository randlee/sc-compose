//! Canonical crate-owned error families.

use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;

use crate::Diagnostic;
use crate::diagnostics::DiagnosticCode;
use crate::types::VariableName;

type BoxedError = Box<dyn StdError + Send + Sync + 'static>;

fn write_error_display(
    f: &mut fmt::Formatter<'_>,
    message: &str,
    source: Option<&(dyn StdError + 'static)>,
    backtrace: &Backtrace,
) -> fmt::Result {
    write!(f, "{message}")?;
    if let Some(source) = source {
        writeln!(f)?;
        write!(f, "caused by:")?;
        let mut current = Some(source);
        while let Some(error) = current {
            write!(f, "\n- {error}")?;
            current = error.source();
        }
    }
    write!(f, "\nbacktrace:\n{backtrace}")
}

fn format_diagnostic_message(diagnostic: &Diagnostic) -> String {
    let mut parts = vec![format!(
        "{}: {}",
        diagnostic.code.as_str(),
        diagnostic.message
    )];
    if let Some(path) = &diagnostic.path {
        let location = match (diagnostic.line, diagnostic.column) {
            (Some(line), Some(column)) => format!("{}:{line}:{column}", path.display()),
            _ => path.display().to_string(),
        };
        parts.push(format!("location={location}"));
    }
    if !diagnostic.include_chain.is_empty() {
        parts.push(format!(
            "include_chain={}",
            diagnostic
                .include_chain
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ")
        ));
    }
    parts.join(" | ")
}

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
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn new(
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
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
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

    /// Return the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for ResolveError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
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
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn new(
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
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
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

    /// Return the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for IncludeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for IncludeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
    }
}

/// Canonical validation error family.
#[derive(Debug)]
pub struct ValidationError {
    code: DiagnosticCode,
    message: String,
    diagnostics: Vec<Diagnostic>,
    recovery_hints: Vec<RecoveryHint>,
    source: Option<BoxedError>,
    backtrace: Backtrace,
}

impl ValidationError {
    /// Create a new validation error.
    #[must_use]
    pub(crate) fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            diagnostics: Vec::new(),
            recovery_hints: Vec::new(),
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Create a validation error from a full diagnostics set.
    #[must_use]
    pub(crate) fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        let code = diagnostics
            .first()
            .map_or(DiagnosticCode::ErrValEmpty, |diagnostic| diagnostic.code);
        let message = diagnostics
            .iter()
            .map(format_diagnostic_message)
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            code,
            message,
            diagnostics,
            recovery_hints: Vec::new(),
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Create a duplicate-frontmatter validation error.
    #[must_use]
    pub(crate) fn duplicate_variable(variable: &VariableName) -> Self {
        Self::new(
            DiagnosticCode::ErrValDuplicate,
            format!("duplicate frontmatter variable declaration: {variable}"),
        )
    }

    /// Create a scalar-type validation error.
    #[must_use]
    pub(crate) fn invalid_scalar(message: impl Into<String>) -> Self {
        Self::new(DiagnosticCode::ErrValType, message)
    }

    /// Attach structured recovery hints.
    #[must_use]
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn with_recovery_hints(mut self, recovery_hints: Vec<RecoveryHint>) -> Self {
        self.recovery_hints = recovery_hints;
        self
    }

    /// Attach an underlying source error.
    #[must_use]
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
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

    /// Return the diagnostics preserved for this validation failure.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Return the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for ValidationError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
    }
}

/// Canonical render error for template compilation and rendering failures.
///
/// This type is only constructed by the library; callers receive it as an
/// opaque error value by design.
#[derive(Debug)]
pub struct RenderError {
    code: Option<DiagnosticCode>,
    source: BoxedError,
    backtrace: Backtrace,
}

impl RenderError {
    /// Construct a canonical render error from an underlying render cause.
    ///
    /// This constructor exists so the library can erase engine-specific error
    /// types at the public API boundary.
    #[must_use]
    pub(crate) fn render(source: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            code: None,
            source: Box::new(source),
            backtrace: Backtrace::capture(),
        }
    }

    /// Attach a stable render code when the calling layer knows one.
    #[must_use]
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
    }

    /// Return the captured backtrace for the render failure.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    /// Return the stable diagnostic code when one was attached by the caller.
    #[must_use]
    pub const fn code(&self) -> Option<DiagnosticCode> {
        self.code
    }

    /// Return the render-failure message.
    #[must_use]
    pub fn message(&self) -> String {
        self.source.to_string()
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
    pub(crate) fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
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
    #[allow(
        unfulfilled_lint_expectations,
        reason = "Constructor is currently used; keep the explicit dead_code expectation tracked."
    )]
    #[expect(
        dead_code,
        reason = "Sprint 2 seeds constructors that later pipeline modules call."
    )]
    pub(crate) fn with_recovery_hints(mut self, recovery_hints: Vec<RecoveryHint>) -> Self {
        self.recovery_hints = recovery_hints;
        self
    }

    /// Attach an underlying source error.
    #[must_use]
    pub(crate) fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
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

    /// Return the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the captured backtrace.
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error_display(f, &self.message, self.source(), &self.backtrace)
    }
}

impl StdError for ConfigError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|error| error as &(dyn StdError + 'static))
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
    Validation(Box<ValidationError>),
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
        Self::Validation(Box::new(value))
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

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::path::PathBuf;

    use super::{
        ComposeError, ConfigError, IncludeError, RecoveryHint, RecoveryHintKind, RenderError,
        ResolveError, ValidationError,
    };
    use crate::Diagnostic;
    use crate::diagnostics::{DiagnosticCode, DiagnosticSeverity};
    use crate::types::VariableName;

    #[test]
    fn resolve_error_constructor_roundtrip_and_display() {
        let error = ResolveError::new(
            DiagnosticCode::ErrResolveNotFound,
            "template not found",
            vec![std::path::PathBuf::from("a.md.j2")],
        )
        .with_source(std::io::Error::other("missing"));

        assert_eq!(error.code(), Some(DiagnosticCode::ErrResolveNotFound));
        assert_eq!(error.attempted_paths().len(), 1);
        assert!(error.to_string().contains("template not found"));
        assert!(error.to_string().contains("caused by:"));
        assert!(error.to_string().contains("missing"));
        assert!(error.to_string().contains("backtrace"));
        assert!(error.source().is_some());
    }

    #[test]
    fn include_error_constructor_roundtrip_and_display() {
        let error = IncludeError::new(
            DiagnosticCode::ErrIncludeEscape,
            "include escaped root",
            vec![std::path::PathBuf::from("parent.md.j2")],
        )
        .with_source(std::io::Error::other("escape"));

        assert_eq!(error.code(), Some(DiagnosticCode::ErrIncludeEscape));
        assert_eq!(error.include_chain().len(), 1);
        assert!(error.to_string().contains("include escaped root"));
        assert!(error.to_string().contains("caused by:"));
        assert!(error.to_string().contains("escape"));
        assert!(error.to_string().contains("backtrace"));
        assert!(error.source().is_some());
    }

    #[test]
    fn validation_error_constructor_roundtrip_and_display() {
        let variable = VariableName::new("name").unwrap();
        let error = ValidationError::duplicate_variable(&variable)
            .with_recovery_hints(vec![RecoveryHint::new(RecoveryHintKind::ProvideVariable {
                variable: variable.clone(),
            })])
            .with_source(std::io::Error::other("duplicate"));

        assert_eq!(error.code(), Some(DiagnosticCode::ErrValDuplicate));
        assert_eq!(error.recovery_hints().len(), 1);
        assert!(error.to_string().contains("duplicate frontmatter variable"));
        assert!(error.to_string().contains("caused by:"));
        assert!(error.to_string().contains("duplicate"));
        assert!(error.to_string().contains("backtrace"));
        assert!(error.source().is_some());
    }

    #[test]
    fn render_error_constructor_roundtrip_and_display() {
        let error = RenderError::render(std::io::Error::other("render failed"));
        assert_eq!(error.code(), None);
        assert!(error.to_string().contains("template rendering failed"));
        assert!(error.source().is_some());
    }

    #[test]
    fn render_error_code_can_be_set_or_left_unset() {
        let without_code = RenderError::render(std::io::Error::other("render failed"));
        let with_code = RenderError::render(std::io::Error::other("write failed"))
            .with_code(DiagnosticCode::ErrRenderWrite);

        assert_eq!(without_code.code(), None);
        assert_eq!(with_code.code(), Some(DiagnosticCode::ErrRenderWrite));
    }

    #[test]
    fn config_error_constructor_roundtrip_and_display() {
        let error = ConfigError::new(DiagnosticCode::ErrConfigParse, "config parse failed")
            .with_recovery_hints(vec![RecoveryHint::new(
                RecoveryHintKind::ReviewConfiguration {
                    key: "frontmatter".to_owned(),
                },
            )])
            .with_source(std::io::Error::other("parse"));

        assert_eq!(error.code(), Some(DiagnosticCode::ErrConfigParse));
        assert_eq!(error.recovery_hints().len(), 1);
        assert!(error.to_string().contains("config parse failed"));
        assert!(error.to_string().contains("caused by:"));
        assert!(error.to_string().contains("parse"));
        assert!(error.to_string().contains("backtrace"));
        assert!(error.source().is_some());
    }

    #[test]
    fn validation_error_from_diagnostics_preserves_all_diagnostics() {
        let diagnostics = vec![
            Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrValMissingRequired,
                "missing required variable: name",
            )
            .with_path("templates/root.md.j2")
            .with_location(12, 4),
            Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrValUndeclaredToken,
                "undeclared referenced token: role",
            )
            .with_include_chain(vec![PathBuf::from("partials/child.md.j2")]),
        ];

        let error = ValidationError::from_diagnostics(diagnostics.clone());

        assert_eq!(error.code(), Some(DiagnosticCode::ErrValMissingRequired));
        assert_eq!(error.diagnostics(), diagnostics.as_slice());
        assert!(error.to_string().contains("templates/root.md.j2:12:4"));
        assert!(
            error
                .to_string()
                .contains("include_chain=partials/child.md.j2")
        );
        assert!(error.to_string().contains("backtrace"));
    }

    #[test]
    fn compose_error_from_conversions_cover_all_variants() {
        let resolve = ComposeError::from(ResolveError::new(
            DiagnosticCode::ErrResolveNotFound,
            "resolve",
            Vec::new(),
        ));
        let include = ComposeError::from(IncludeError::new(
            DiagnosticCode::ErrIncludeEscape,
            "include",
            Vec::new(),
        ));
        let validation = ComposeError::from(ValidationError::new(
            DiagnosticCode::ErrValEmpty,
            "validation",
        ));
        let render = ComposeError::from(
            RenderError::render(std::io::Error::other("render"))
                .with_code(DiagnosticCode::ErrRenderWrite),
        );
        let config = ComposeError::from(ConfigError::new(DiagnosticCode::ErrConfigParse, "config"));

        assert!(matches!(resolve, ComposeError::Resolve(_)));
        assert!(matches!(include, ComposeError::Include(_)));
        assert!(matches!(validation, ComposeError::Validation(_)));
        assert!(matches!(render, ComposeError::Render(_)));
        assert!(matches!(config, ComposeError::Config(_)));
    }
}
