use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;
use std::path::Path;

use super::display::BoxedError;
use crate::diagnostics::DiagnosticCode;
use crate::include::{ExpandedTemplate, SourceLocation};
use crate::types::{ComposePolicy, ConfiningRoot};

/// Canonical render error for template compilation and rendering failures.
///
/// This type is only constructed by the library; callers receive it as an
/// opaque error value by design.
#[derive(Debug)]
pub struct RenderError {
    code: Option<DiagnosticCode>,
    message: String,
    source: BoxedError,
    backtrace: Backtrace,
    // Boxed so the common error path stays small.
    location: Option<Box<ErrorLocation>>,
}

/// Where the engine reported the failure and, once located, its source.
#[derive(Debug)]
struct ErrorLocation {
    engine: EngineLocation,
    source: Option<SourceLocation>,
}

/// Where the template engine reported the failure, in the text it was given.
#[derive(Debug)]
struct EngineLocation {
    template_name: String,
    line: usize,
    column: Option<usize>,
    line_text: String,
}

impl RenderError {
    /// Construct a canonical render error from an underlying render cause.
    ///
    /// This constructor exists so the library can erase engine-specific error
    /// types at the public API boundary.
    #[must_use]
    pub(crate) fn render(source: impl StdError + Send + Sync + 'static) -> Self {
        let message = source.to_string();
        Self {
            code: None,
            message,
            source: Box::new(source),
            backtrace: Backtrace::capture(),
            location: None,
        }
    }

    /// Construct a render error with a caller-chosen code and message.
    #[must_use]
    pub(crate) fn with_code(
        source: impl StdError + Send + Sync + 'static,
        code: DiagnosticCode,
        message: String,
    ) -> Self {
        Self {
            code: Some(code),
            message,
            source: Box::new(source),
            backtrace: Backtrace::capture(),
            location: None,
        }
    }

    /// Record the line (and column) the template engine reported in
    /// `template`, the text it rendered under `template_name`.
    #[must_use]
    pub(crate) fn with_engine_location(
        mut self,
        template_name: &str,
        template: &str,
        line: usize,
        byte_offset: Option<usize>,
    ) -> Self {
        let Some(line_text) = line
            .checked_sub(1)
            .and_then(|index| template.split_inclusive('\n').nth(index))
        else {
            return self;
        };
        let line_start = template
            .split_inclusive('\n')
            .take(line - 1)
            .map(str::len)
            .sum::<usize>();
        let column = byte_offset
            .and_then(|offset| offset.checked_sub(line_start))
            .and_then(|within| line_text.get(..within))
            .map(|prefix| prefix.chars().count() + 1);
        self.location = Some(Box::new(ErrorLocation {
            engine: EngineLocation {
                template_name: template_name.to_owned(),
                line,
                column,
                line_text: line_text.to_owned(),
            },
            source: None,
        }));
        self
    }

    /// Point the error at the source file and line it came from.
    ///
    /// The template engine counts lines in the expanded text it rendered,
    /// where included files are pasted in and the frontmatter is removed.
    /// This translates that line through `expanded` (ADR-0026 D10) and
    /// rewrites the location in the message to name the source file,
    /// relative to `root` when it is inside it. `template_path`, `root` and
    /// `policy` must be the values `expanded` was produced with. When the line
    /// cannot be located the error is returned unchanged.
    #[must_use]
    pub fn located_in(
        mut self,
        expanded: &ExpandedTemplate,
        template_path: &Path,
        root: &ConfiningRoot,
        policy: &ComposePolicy,
    ) -> Self {
        let Some(located) = self.location.as_deref_mut() else {
            return self;
        };
        let engine = &mut located.engine;
        let Some(location) = expanded.source_location(template_path, root, policy, engine.line)
        else {
            return self;
        };
        // The column only carries over when the engine saw the line unchanged.
        let expanded_line = expanded.text.split_inclusive('\n').nth(engine.line - 1);
        if expanded_line != Some(engine.line_text.as_str()) {
            engine.column = None;
        }
        let shown = location
            .path
            .strip_prefix(root.as_path())
            .unwrap_or(&location.path)
            .display()
            .to_string();
        let replacement = format!("{shown}:{}", location.line);
        located.source = Some(location);
        self.relocate_message(&replacement);
        self
    }

    /// Rewrite the engine's `name:line` location in the message and in the
    /// displayed cause, which repeats the engine's own text.
    fn relocate_message(&mut self, replacement: &str) {
        let Some(located) = &self.location else {
            return;
        };
        let (name, line) = (located.engine.template_name.as_str(), located.engine.line);
        self.message = replace_engine_location(&self.message, name, line, replacement);
        let cause = replace_engine_location(&self.source.to_string(), name, line, replacement);
        let engine_error = std::mem::replace(&mut self.source, Box::new(fmt::Error));
        self.source = Box::new(RelocatedCause {
            message: cause,
            engine_error,
        });
    }

    /// Say that the reported line is in the input of a later render pass.
    ///
    /// Pass 2 and later render the output of the previous pass, so their line
    /// numbers do not correspond to any source file.
    #[must_use]
    pub(crate) fn in_pass_input(mut self, pass: u8) -> Self {
        if let Some(located) = &self.location {
            let replacement = format!(
                "{}:{} of the pass {pass} input",
                located.engine.template_name, located.engine.line
            );
            self.relocate_message(&replacement);
        }
        self
    }

    /// Return the source file location, when the error was located with
    /// [`Self::located_in`].
    #[must_use]
    pub fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()?.source.as_ref()
    }

    /// Return the one-based column in the source line, when known.
    #[must_use]
    pub fn column(&self) -> Option<usize> {
        let located = self.location.as_ref()?;
        located.source.as_ref()?;
        located.engine.column
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
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "template rendering failed")
    }
}

impl StdError for RenderError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Replace the last `(name:line)` or `(in name:line)` location the engine or
/// renderer wrote into a message.
fn replace_engine_location(message: &str, name: &str, line: usize, replacement: &str) -> String {
    let needle = format!("{name}:{line})");
    let found = message
        .rmatch_indices(&needle)
        .map(|(start, _)| start)
        .find(|&start| {
            let before = &message[..start];
            before.ends_with('(') || before.ends_with("(in ")
        });
    match found {
        Some(start) => format!(
            "{}{replacement}){}",
            &message[..start],
            &message[start + needle.len()..]
        ),
        None => message.to_owned(),
    }
}

/// The engine's error with its location rewritten. It displays the rewritten
/// text and keeps the engine error's own causes.
#[derive(Debug)]
struct RelocatedCause {
    message: String,
    engine_error: BoxedError,
}

impl fmt::Display for RelocatedCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for RelocatedCause {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.engine_error.source()
    }
}
