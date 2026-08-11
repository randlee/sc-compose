//! Canonical crate-owned error families.

use crate::diagnostics::DiagnosticCode;

use std::fmt;

mod config;
mod display;
mod include;
mod recovery;
mod render;
mod resolve;
mod validation;

pub use config::ConfigError;
pub use include::IncludeError;
pub use recovery::{RecoveryHint, RecoveryHintKind};
pub use render::RenderError;
pub use resolve::ResolveError;
pub use validation::ValidationError;

use std::error::Error as StdError;

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
            Self::Resolve(error) => Some(error.code()),
            Self::Include(error) => Some(error.code()),
            Self::Validation(error) => Some(error.code()),
            Self::Render(error) => error.code(),
            Self::Config(error) => Some(error.code()),
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
    use crate::renderer::Renderer;
    use crate::types::VariableName;

    #[test]
    fn empty_custom_delimiter_error_does_not_duplicate_source_text() {
        let error = ComposeError::from(Renderer::with_delimiters("", "").unwrap_err());
        let mut formatted = error.to_string();
        if let Some(source) = error.source() {
            formatted.push_str(": ");
            formatted.push_str(&source.to_string());
        }

        assert_eq!(formatted.matches("invalid custom delimiters").count(), 1);
    }

    #[test]
    fn non_delimiter_render_error_keeps_source_detail() {
        let error = ComposeError::from(RenderError::render(std::io::Error::other("render failed")));
        let mut formatted = error.to_string();
        if let Some(source) = error.source() {
            formatted.push_str(": ");
            formatted.push_str(&source.to_string());
        }

        assert!(!formatted.is_empty());
        assert!(formatted.contains("render failed"));
    }

    #[test]
    fn resolve_error_constructor_roundtrip_and_display() {
        let error = ResolveError::new(
            DiagnosticCode::ErrResolveNotFound,
            "template not found",
            vec![std::path::PathBuf::from("a.md.j2")],
        )
        .with_source(std::io::Error::other("missing"));

        assert_eq!(error.code(), DiagnosticCode::ErrResolveNotFound);
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

        assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
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
        let error = ValidationError::duplicate_variable(&variable);

        assert_eq!(error.code(), DiagnosticCode::ErrValDuplicate);
        assert_eq!(error.recovery_hints().len(), 1);
        assert!(error.to_string().contains("duplicate frontmatter variable"));
        assert!(error.to_string().contains("backtrace"));
        assert!(error.source().is_none());
    }

    #[test]
    fn render_error_constructor_roundtrip_and_display() {
        let error = RenderError::render(std::io::Error::other("render failed"));
        assert_eq!(error.code(), None);
        assert!(error.to_string().contains("template rendering failed"));
        assert!(error.source().is_some());
    }

    #[test]
    fn render_error_code_defaults_to_none() {
        let error = RenderError::render(std::io::Error::other("render failed"));
        assert_eq!(error.code(), None);
    }

    #[test]
    fn config_error_constructor_roundtrip_and_display() {
        let error = ConfigError::new(DiagnosticCode::ErrConfigParse, "config parse failed")
            .with_source(std::io::Error::other("parse"));

        assert_eq!(error.code(), DiagnosticCode::ErrConfigParse);
        assert!(error.recovery_hints().is_empty());
        assert!(error.to_string().contains("config parse failed"));
        assert!(error.to_string().contains("caused by:"));
        assert!(error.to_string().contains("parse"));
        assert!(error.to_string().contains("backtrace"));
        assert!(error.source().is_some());
    }

    #[test]
    fn recovery_hints_keep_structured_payloads_across_families() {
        let config = ConfigError::new(DiagnosticCode::ErrConfigParse, "config").with_recovery_hint(
            RecoveryHint::new(RecoveryHintKind::RunCommand {
                command: "sc-compose validate".to_owned(),
            }),
        );
        assert_eq!(
            config.recovery_hints()[0].kind,
            RecoveryHintKind::RunCommand {
                command: "sc-compose validate".to_owned(),
            }
        );

        let validation = ValidationError::new(DiagnosticCode::ErrValEmpty, "validation")
            .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::UnsupportedConstruct {
                description: "dynamic key".to_owned(),
            }));
        assert_eq!(
            validation.recovery_hints()[0].kind,
            RecoveryHintKind::UnsupportedConstruct {
                description: "dynamic key".to_owned(),
            }
        );
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

        assert_eq!(error.code(), DiagnosticCode::ErrValMissingRequired);
        assert_eq!(error.diagnostics(), diagnostics.as_slice());
        assert!(error.to_string().contains("templates/root.md.j2:12:4"));
        assert!(
            error
                .to_string()
                .contains("include_chain=partials/child.md.j2")
        );
        assert_eq!(error.recovery_hints().len(), 1);
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
        let render = ComposeError::from(RenderError::render(std::io::Error::other("render")));
        let config = ComposeError::from(ConfigError::new(DiagnosticCode::ErrConfigParse, "config"));

        assert!(matches!(resolve, ComposeError::Resolve(_)));
        assert!(matches!(include, ComposeError::Include(_)));
        assert!(matches!(validation, ComposeError::Validation(_)));
        assert!(matches!(render, ComposeError::Render(_)));
        assert!(matches!(config, ComposeError::Config(_)));
    }
}
