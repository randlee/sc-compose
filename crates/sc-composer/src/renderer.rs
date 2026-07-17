//! Template renderer wrapper.

use std::collections::BTreeMap;

use minijinja::Environment;
use serde::Serialize;
use serde_json::Value;

use crate::RenderError;

/// Additional named templates that the main template may extend or include.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NamedTemplateAsset {
    /// Stable template identifier used for loader lookups.
    pub template_name: String,
    /// Template body associated with the identifier.
    pub template_text: String,
}

/// Request for rendering template text that the caller already loaded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoadedTemplateRequest {
    /// Stable template identifier used for diagnostics and template naming.
    pub template_name: String,
    /// Pre-loaded template text to render.
    pub template_text: String,
    /// Render context supplied by the caller.
    pub context: BTreeMap<String, Value>,
    /// Additional named templates the main template may extend or include.
    #[serde(default)]
    pub supporting_templates: Vec<NamedTemplateAsset>,
}

/// Artifact returned by the pre-loaded template render path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RenderedArtifact {
    /// Final rendered output text.
    pub rendered: String,
    /// Stable template identifier used during rendering.
    pub template_name: String,
}

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
    pub(crate) fn with_options(configure: impl FnOnce(&mut Environment<'static>)) -> Self {
        let mut env = Environment::new();
        env.set_trim_blocks(true);
        env.set_lstrip_blocks(true);
        configure(&mut env);
        Self { env }
    }

    /// Create a renderer with non-default variable delimiters.
    ///
    /// # Panics
    ///
    /// Panics if `open` or `close` are not valid delimiter tokens accepted by
    /// the underlying template engine.
    #[must_use]
    pub fn with_delimiters(open: &str, close: &str) -> Self {
        let open = open.to_owned();
        let close = close.to_owned();
        Self::with_options(|env| {
            env.set_syntax(
                minijinja::syntax::SyntaxConfig::builder()
                    .variable_delimiters(open, close)
                    .build()
                    .expect("valid delimiter configuration"),
            );
        })
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
        self.render_named("inline", template, context)
    }

    /// Render a template string with an explicit template name.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when template parsing or rendering fails.
    pub fn render_named<T: serde::Serialize>(
        &self,
        template_name: &str,
        template: &str,
        context: T,
    ) -> Result<String, RenderError> {
        let template = self
            .env
            .template_from_named_str(template_name, template)
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

/// Render pre-loaded template content without taking ownership of file
/// discovery or repository traversal.
///
/// # Errors
///
/// Returns [`RenderError`] when template parsing or rendering fails.
pub fn render_loaded_template(
    request: LoadedTemplateRequest,
) -> Result<RenderedArtifact, RenderError> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    for asset in request.supporting_templates {
        env.add_template_owned(asset.template_name, asset.template_text)
            .map_err(RenderError::render)?;
    }
    env.add_template_owned(request.template_name.clone(), request.template_text.clone())
        .map_err(RenderError::render)?;
    let rendered = env
        .get_template(&request.template_name)
        .map_err(RenderError::render)?
        .render(&request.context)
        .map_err(RenderError::render)?;
    Ok(RenderedArtifact {
        rendered,
        template_name: request.template_name,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{LoadedTemplateRequest, NamedTemplateAsset, Renderer, render_loaded_template};

    #[test]
    fn renderer_can_render_multiple_templates_with_one_environment() {
        let renderer = Renderer::new();

        let first = renderer.render("hello {{ name }}", json!({ "name": "world" }));
        let second = renderer.render("bye {{ name }}", json!({ "name": "world" }));

        assert_eq!(first.unwrap(), "hello world");
        assert_eq!(second.unwrap(), "bye world");
    }

    #[test]
    fn renderer_supports_jinja_plus_modifier_opt_out() {
        let renderer = Renderer::new();

        let output = renderer
            .render(
                "before\n    {%+ if true %}\nvalue\n    {% endif %}\nafter\n",
                json!({}),
            )
            .unwrap();

        assert_eq!(output, "before\n    value\nafter");
    }

    #[test]
    fn render_loaded_template_rejects_malformed_supporting_templates() {
        let error = render_loaded_template(LoadedTemplateRequest {
            template_name: "report.html.j2".to_owned(),
            template_text: "{{ value }}".to_owned(),
            context: json!({ "value": "ok" })
                .as_object()
                .unwrap()
                .clone()
                .into_iter()
                .collect(),
            supporting_templates: vec![NamedTemplateAsset {
                template_name: "shared/base.html.j2".to_owned(),
                template_text: "{% if broken %}".to_owned(),
            }],
        })
        .unwrap_err();

        assert!(
            error.to_string().contains("unexpected end of input"),
            "expected supporting-template parse failure, got: {error}"
        );
    }
}
