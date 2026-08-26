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
    let rendered = sc_composer::Renderer::with_delimiters_and_json_escape_mode(
        "{{{",
        "}}}",
        json_escape_mode(template),
    )
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

fn json_escape_mode(template: &Path) -> sc_composer::JsonEscapeMode {
    if template.to_string_lossy().ends_with(".formula.json.j2") {
        // Formula templates retain their literal JSON quotes.  This mirrors
        // the documented sc-compose legacy JSON shape while the deliberately
        // distinct triple braces preserve Beads runtime placeholders.
        sc_composer::JsonEscapeMode::Legacy
    } else {
        sc_composer::JsonEscapeMode::Auto
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::{Map, json};

    use super::render_formula;

    #[test]
    fn json_formula_templates_escape_literal_quoted_values_once() {
        let root = temporary_directory();
        let template = root.join("example.formula.json.j2");
        let output = root.join("example.formula.json");
        fs::write(
            &template,
            r#"{ "title": "{{{ title }}}", "runtime": "{{ bead_var }}" }"#,
        )
        .expect("write template");
        render_formula(
            &template,
            &output,
            &Map::from_iter([(String::from("title"), json!("café\nnext"))]),
        )
        .expect("render JSON formula");

        let rendered = fs::read_to_string(&output).expect("read output");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&rendered).expect("valid JSON"),
            json!({ "title": "café\nnext", "runtime": "{{ bead_var }}" })
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    fn temporary_directory() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sc-composer-beads-render-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create test directory");
        root
    }
}
