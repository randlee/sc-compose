//! Template renderer wrapper.

use minijinja::Environment;

use crate::RenderError;

/// Pure template-engine wrapper used by composition entry points.
#[derive(Debug)]
pub struct Renderer {
    env: Environment<'static>,
}

impl Renderer {
    /// Create a renderer with the default environment options.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(|_| {})
    }

    /// Create a renderer with additional environment configuration.
    #[must_use]
    pub fn with_options(configure: impl FnOnce(&mut Environment<'static>)) -> Self {
        let mut env = Environment::new();
        env.set_trim_blocks(true);
        env.set_lstrip_blocks(true);
        configure(&mut env);
        Self { env }
    }

    /// Render a template string with the provided serializable context.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when template parsing or rendering fails.
    pub fn render<T: serde::Serialize>(
        &self,
        template: &str,
        context: T,
    ) -> Result<String, RenderError> {
        let template = self
            .env
            .template_from_named_str("inline", template)
            .map_err(RenderError::render)?;
        template.render(context).map_err(RenderError::render)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a template string with the provided serializable context.
///
/// This is the stable one-shot convenience API over [`Renderer`].
///
/// # Errors
///
/// Returns [`RenderError`] when template parsing or rendering fails.
pub fn render_template<T: serde::Serialize>(
    template: &str,
    context: T,
) -> Result<String, RenderError> {
    Renderer::new().render(template, context)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Renderer;

    #[test]
    fn renderer_can_render_multiple_templates_with_one_environment() {
        let renderer = Renderer::new();

        let first = renderer.render("hello {{ name }}", json!({ "name": "world" }));
        let second = renderer.render("bye {{ name }}", json!({ "name": "world" }));

        assert_eq!(first.unwrap(), "hello world");
        assert_eq!(second.unwrap(), "bye world");
    }
}
