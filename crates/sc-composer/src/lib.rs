//! Core rendering and composition primitives for the `sc-compose` workspace.

use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;

use minijinja::Environment;

/// Canonical render error for template compilation and rendering failures.
///
/// This type is only constructed by the library; callers receive it as an
/// opaque error value by design.
#[derive(Debug)]
pub struct RenderError {
    source: Box<dyn StdError + Send + Sync + 'static>,
    backtrace: Backtrace,
}

impl RenderError {
    pub(crate) fn render(source: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
            backtrace: Backtrace::capture(),
        }
    }

    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
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

/// Render a template string with the provided serializable context.
///
/// This is the stable one-shot convenience API. Callers that render repeatedly
/// should use the future long-lived renderer/session API described in the
/// architecture document.
///
/// # Errors
///
/// Returns [`RenderError`] when the template cannot be parsed, loaded, or
/// rendered by the underlying template engine.
pub fn render_template<T: serde::Serialize>(
    template: &str,
    context: T,
) -> Result<String, RenderError> {
    let mut env = Environment::new();
    env.add_template("inline", template)
        .map_err(RenderError::render)?;
    let template = env.get_template("inline").map_err(RenderError::render)?;
    template.render(context).map_err(RenderError::render)
}

#[cfg(test)]
mod tests {
    use super::render_template;

    #[test]
    fn renders_inline_template() {
        let rendered =
            render_template("hello {{ name }}", serde_json::json!({ "name": "world" })).unwrap();
        assert_eq!(rendered, "hello world");
    }
}
