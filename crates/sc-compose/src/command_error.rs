use std::fmt;

use anyhow::{Error, anyhow};
use sc_composer::{
    ComposeError, Diagnostic, DiagnosticCode, DiagnosticSeverity, RecoveryHint, RecoveryHintKind,
};

#[derive(Debug)]
pub(crate) struct CommandError {
    pub(crate) exit_code: i32,
    pub(crate) diagnostic_code: Option<DiagnosticCode>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) recovery_hints: Vec<RecoveryHint>,
    pub(crate) error: Error,
}

impl CommandError {
    pub(crate) fn usage(error: Error) -> Self {
        Self {
            exit_code: crate::exit_codes::USAGE_FAIL,
            diagnostic_code: None,
            diagnostics: Vec::new(),
            recovery_hints: Vec::new(),
            error,
        }
    }

    pub(crate) fn usage_with_code(error: Error, diagnostic_code: DiagnosticCode) -> Self {
        Self::usage_with_code_and_hints(error, diagnostic_code, Vec::new())
    }

    pub(crate) fn usage_with_code_and_hints(
        error: Error,
        diagnostic_code: DiagnosticCode,
        recovery_hints: Vec<RecoveryHint>,
    ) -> Self {
        Self {
            exit_code: crate::exit_codes::USAGE_FAIL,
            diagnostic_code: Some(diagnostic_code),
            diagnostics: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                diagnostic_code,
                format!("{error:#}"),
            )],
            recovery_hints,
            error,
        }
    }

    pub(crate) fn compose(error: ComposeError) -> Self {
        let exit_code = match &error {
            ComposeError::Validation(_) | ComposeError::Render(_) | ComposeError::Include(_) => {
                crate::exit_codes::VALIDATION_OR_RENDER_FAIL
            }
            ComposeError::Resolve(_) | ComposeError::Config(_) => crate::exit_codes::USAGE_FAIL,
        };
        Self {
            exit_code,
            diagnostic_code: error.code(),
            diagnostics: compose_error_diagnostics(&error),
            recovery_hints: compose_error_recovery_hints(&error),
            error: anyhow!(error),
        }
    }

    pub(crate) fn render_write(error: Error) -> Self {
        Self {
            exit_code: crate::exit_codes::VALIDATION_OR_RENDER_FAIL,
            diagnostic_code: Some(DiagnosticCode::ErrRenderWrite),
            diagnostics: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrRenderWrite,
                format!("{error:#}"),
            )],
            recovery_hints: Vec::new(),
            error,
        }
    }

    pub(crate) fn stdin_double_read() -> Self {
        Self {
            exit_code: crate::exit_codes::VALIDATION_OR_RENDER_FAIL,
            diagnostic_code: Some(DiagnosticCode::ErrRenderStdinDoubleRead),
            diagnostics: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrRenderStdinDoubleRead,
                "guidance and prompt cannot both read from stdin",
            )],
            recovery_hints: Vec::new(),
            error: anyhow!("guidance and prompt cannot both read from stdin"),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = self.diagnostic_code {
            write!(f, "{}: {:#}", code.as_str(), self.error)?;
        } else {
            write!(f, "{:#}", self.error)?;
        }
        for hint in &self.recovery_hints {
            write!(f, "\nrecovery: {}", format_recovery_hint(hint))?;
        }
        Ok(())
    }
}

impl std::error::Error for CommandError {}

fn format_recovery_hint(hint: &RecoveryHint) -> String {
    match &hint.kind {
        RecoveryHintKind::RunCommand { command } => format!("run `{command}`"),
        RecoveryHintKind::InspectPath { path } => format!("inspect {}", path.display()),
        RecoveryHintKind::ProvideVariable { variable } => {
            format!("provide variable `{}`", variable.as_str())
        }
        RecoveryHintKind::ReviewConfiguration { key } => {
            format!("review configuration: {key}")
        }
        RecoveryHintKind::InspectInput { description } => {
            format!("inspect input: {description}")
        }
        RecoveryHintKind::DisambiguateOccurrences { description } => {
            format!("disambiguate occurrences: {description}")
        }
        RecoveryHintKind::UnsupportedConstruct { description } => {
            format!("replace unsupported construct: {description}")
        }
    }
}

fn compose_error_diagnostics(error: &ComposeError) -> Vec<Diagnostic> {
    match error {
        ComposeError::Validation(validation) if !validation.diagnostics().is_empty() => {
            validation.diagnostics().to_vec()
        }
        ComposeError::Resolve(resolve) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            resolve.code(),
            resolve.message(),
        )],
        ComposeError::Include(include) => vec![
            Diagnostic::new(DiagnosticSeverity::Error, include.code(), include.message())
                .with_include_chain(include.include_chain().to_vec()),
        ],
        ComposeError::Validation(validation) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            validation.code(),
            validation.message(),
        )],
        ComposeError::Render(render) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            render.code().unwrap_or(DiagnosticCode::ErrRenderWrite),
            render.message(),
        )],
        ComposeError::Config(config) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            config.code(),
            config.message(),
        )],
    }
}

fn compose_error_recovery_hints(error: &ComposeError) -> Vec<RecoveryHint> {
    match error {
        ComposeError::Validation(error) => error.recovery_hints().to_vec(),
        ComposeError::Config(error) => error.recovery_hints().to_vec(),
        ComposeError::Resolve(_) | ComposeError::Include(_) | ComposeError::Render(_) => Vec::new(),
    }
}
