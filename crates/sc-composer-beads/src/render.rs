//! Fixed-delimiter formula rendering.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Map;

use crate::error::BeadComposeError;

static TEMPORARY_OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Render one Beads formula template with triple-brace composition delimiters.
///
/// Beads' ordinary `{{ runtime_var }}` expressions remain literal because
/// only `{{{ compose_value }}}` is interpreted by `sc-composer`.
///
/// # Errors
///
/// Returns [`BeadComposeError::RenderFailed`] when the input cannot be read,
/// rendered, or written. Final-component symbolic links are rejected before
/// the rendered data is written.
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
        message: error.message().to_owned(),
    })?;
    atomic_write(rendered_formula, rendered.as_bytes())
}

/// Reject a final output component that would redirect a renderer write.
///
/// The caller must invoke this after normalizing the output parent directory.
/// The subsequent write uses a fresh sibling temporary file and rename, so it
/// never follows the final path component.
pub(crate) fn validate_output_destination(path: &Path) -> Result<(), BeadComposeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(BeadComposeError::OutputPathSymlink { path: path.into() })
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_error) => Err(BeadComposeError::TemplatePathInvalid { path: path.into() }),
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), BeadComposeError> {
    validate_output_destination(path)?;
    let temporary = temporary_output_path(path)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| render_error(&error))?;
        file.write_all(contents)
            .map_err(|error| render_error(&error))?;
        file.sync_all().map_err(|error| render_error(&error))?;
        drop(file);
        replace_output(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn temporary_output_path(path: &Path) -> Result<std::path::PathBuf, BeadComposeError> {
    let parent = path
        .parent()
        .ok_or_else(|| BeadComposeError::TemplatePathInvalid { path: path.into() })?;
    let name = path
        .file_name()
        .ok_or_else(|| BeadComposeError::TemplatePathInvalid { path: path.into() })?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| BeadComposeError::RenderFailed {
            message: error.to_string(),
        })?
        .as_nanos();
    let sequence = TEMPORARY_OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{}.sc-compose-{}-{timestamp}-{sequence}.tmp",
        name.to_string_lossy(),
        std::process::id(),
    )))
}

#[cfg(unix)]
fn replace_output(temporary: &Path, path: &Path) -> Result<(), BeadComposeError> {
    fs::rename(temporary, path).map_err(|error| render_error(&error))
}

#[cfg(windows)]
fn replace_output(temporary: &Path, path: &Path) -> Result<(), BeadComposeError> {
    // Windows cannot atomically replace an existing destination with
    // `std::fs::rename`. Rechecking and removing the final component still
    // prevents following a symbolic link; a racing replacement causes rename
    // to fail instead of redirecting the temporary file write.
    validate_output_destination(path)?;
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(render_error(&error)),
    }
    fs::rename(temporary, path).map_err(|error| render_error(&error))
}

fn render_error(error: &std::io::Error) -> BeadComposeError {
    BeadComposeError::RenderFailed {
        message: error.to_string(),
    }
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

    // FUZZ-TEMPLATE-001 (adversarial fuzz campaign 20260817-1,
    // template-probe): confirmed bug, not yet fixed. `.formula.toml.j2`
    // resolves to `JsonEscapeMode::Auto`, which only escapes html/xml/json
    // content (see `auto_escape_callback` in crates/sc-composer); TOML gets
    // no string-safety handling at all, unlike its `.formula.json.j2`
    // sibling above. A compose value containing a literal `"` or `\`
    // therefore corrupts the rendered TOML document instead of producing a
    // valid quoted string. This test pins the current corrupted output so
    // the gap is visible; update it once TOML formulas gain an escaping
    // path (or an explicit validation error) for such values.
    #[test]
    fn toml_formula_templates_embed_unescaped_quotes_and_backslashes() {
        let root = temporary_directory();
        let template = root.join("example.formula.toml.j2");
        let output = root.join("example.formula.toml");
        fs::write(&template, "a = \"{{{ x }}}\"").expect("write template");
        render_formula(
            &template,
            &output,
            &Map::from_iter([(String::from("x"), json!("has \"quotes\" and \\backslash"))]),
        )
        .expect("render TOML formula");

        let rendered = fs::read_to_string(&output).expect("read output");
        assert_eq!(rendered, "a = \"has \"quotes\" and \\backslash\"");
        assert!(
            rendered.matches('"').count() > 2,
            "a syntactically valid single quoted TOML string has exactly \
             2 quote characters; got: {rendered:?}"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    // FUZZ-4177-BOUNDARY-01 (adversarial fuzz campaign 20260817-1,
    // boundary-probe): regression coverage for preserving the distinct
    // cause-specific messages exposed by `RenderError::message()` instead of
    // its intentionally opaque `Display` implementation.
    #[test]
    fn render_failed_message_is_specific_for_distinct_failure_causes() {
        let root = temporary_directory();
        let vars: Map<String, serde_json::Value> =
            Map::from_iter([(String::from("x"), json!("v"))]);

        let unterminated = root.join("unterminated.formula.toml.j2");
        fs::write(&unterminated, "hello {{{ unterminated").expect("write template");
        let out1 = root.join("unterminated.formula.toml");
        let err1 = render_formula(&unterminated, &out1, &vars)
            .expect_err("unterminated expression must fail");

        let unknown_filter = root.join("unknown_filter.formula.toml.j2");
        fs::write(
            &unknown_filter,
            "{{{ x | this_filter_does_not_exist_anywhere }}}",
        )
        .expect("write template");
        let out2 = root.join("unknown_filter.formula.toml");
        let err2 =
            render_formula(&unknown_filter, &out2, &vars).expect_err("unknown filter must fail");

        assert_ne!(err1.to_string(), err2.to_string());
        assert_ne!(
            err1.to_string(),
            "formula rendering failed: template rendering failed"
        );
        assert_ne!(
            err2.to_string(),
            "formula rendering failed: template rendering failed"
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
