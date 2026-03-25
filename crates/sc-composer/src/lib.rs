//! Core rendering and composition primitives for the `sc-compose` workspace.

use minijinja::Environment;
use thiserror::Error;

/// Canonical render error for template compilation and rendering failures.
#[derive(Debug, Error)]
pub enum RenderError {
    /// The template engine failed while parsing, loading, or rendering.
    #[error("template render failed: {0}")]
    Render(#[from] minijinja::Error),
}

/// Render a template string with the provided serializable context.
pub fn render_template<T: serde::Serialize>(
    template: &str,
    context: T,
) -> Result<String, RenderError> {
    let mut env = Environment::new();
    env.add_template("inline", template)?;
    let template = env.get_template("inline")?;
    Ok(template.render(context)?)
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
