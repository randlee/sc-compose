use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::schema::{DiagnosticCode, DiagnosticSeverity};

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Diagnostic, DiagnosticCode, DiagnosticSeverity};

    #[test]
    fn constructors_preserve_defaults_and_chain_order() {
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticCode::WarnExtractNotObserved,
            "not observed",
        );
        assert_eq!(diagnostic.path, None);
        assert_eq!(diagnostic.line, None);
        assert_eq!(diagnostic.column, None);
        assert!(diagnostic.include_chain.is_empty());

        let diagnostic = diagnostic
            .with_path("root.md")
            .with_location(4, 9)
            .with_include_chain(vec![PathBuf::from("root.md"), PathBuf::from("child.md")]);
        assert_eq!(diagnostic.path, Some(PathBuf::from("root.md")));
        assert_eq!(diagnostic.line, Some(4));
        assert_eq!(diagnostic.column, Some(9));
        assert_eq!(
            diagnostic.include_chain,
            [PathBuf::from("root.md"), PathBuf::from("child.md")]
        );
    }
}
