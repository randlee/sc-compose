use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;

use crate::Diagnostic;

pub(super) type BoxedError = Box<dyn StdError + Send + Sync + 'static>;

pub(super) fn write_error_display(
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

pub(super) fn format_diagnostic_message(diagnostic: &Diagnostic) -> String {
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
