//! Template renderer wrapper.

use minijinja::Environment;

use crate::RenderError;

/// Pure template-engine wrapper used by composition entry points.
#[derive(Debug, Default)]
pub struct Renderer;

impl Renderer {
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
        let mut environment = Environment::new();
        environment.set_trim_blocks(true);
        environment.set_lstrip_blocks(true);
        environment
            .add_template("inline", template)
            .map_err(RenderError::render)?;
        let template = environment
            .get_template("inline")
            .map_err(RenderError::render)?;
        template.render(context).map_err(RenderError::render)
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
    Renderer.render(template, context)
}
