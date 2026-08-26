//! Fixed-delimiter formula rendering.

use std::fs;
use std::path::Path;

use serde_json::Map;

use crate::error::BeadComposeError;

/// Render one Beads formula template with triple-brace composition delimiters.
///
/// Beads' ordinary `{{ runtime_var }}` expressions remain literal because
/// only `{{{ compose_value }}}` is interpreted by `sc-composer`.
///
/// # Errors
///
/// Returns [`BeadComposeError::RenderFailed`] when the input cannot be read,
/// rendered, or written.
pub fn render_formula(
    template: &Path,
    rendered_formula: &Path,
    compose_variables: &Map<String, serde_json::Value>,
) -> Result<(), BeadComposeError> {
    let template_text =
        fs::read_to_string(template).map_err(|error| BeadComposeError::RenderFailed {
            message: error.to_string(),
        })?;
    let rendered = sc_composer::Renderer::with_delimiters("{{{", "}}}")
        .and_then(|renderer| {
            renderer.render_named(
                &template.to_string_lossy(),
                &template_text,
                compose_variables,
            )
        })
        .map_err(|error| BeadComposeError::RenderFailed {
            message: error.to_string(),
        })?;
    fs::write(rendered_formula, rendered).map_err(|error| BeadComposeError::RenderFailed {
        message: error.to_string(),
    })
}
