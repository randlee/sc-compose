//! Fixed-delimiter formula rendering.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Map;

use crate::error::BeadComposeError;
use crate::execute::public_path_buf;

static TEMPORARY_OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Render one Beads formula template with triple-brace composition delimiters.
///
/// Beads' ordinary `{{ runtime_var }}` expressions remain literal because
/// only `{{{ compose_value }}}` is interpreted by `sc-composer`.
///
/// Includes are confined to the template's parent directory. Beads requests use
/// their working directory as the confinement root instead.
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
    let root = template
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    render_formula_in_root(template, rendered_formula, compose_variables, root)
}

/// Render with includes confined to the request's working directory.
pub(crate) fn render_formula_in_root(
    template: &Path,
    rendered_formula: &Path,
    compose_variables: &Map<String, serde_json::Value>,
    working_directory: &Path,
) -> Result<(), BeadComposeError> {
    let root =
        sc_composer::ConfiningRoot::new(working_directory).map_err(|error| render_error(&error))?;
    let expanded =
        sc_composer::expand_includes(template, &root, &sc_composer::ComposePolicy::default())
            .map_err(|error| BeadComposeError::RenderFailed {
                message: error.to_string(),
            })?;
    let rendered =
        sc_composer::Renderer::with_delimiters_and_escape_mode("{{{", "}}}", escape_mode(template))
            .and_then(|renderer| {
                renderer.render_named(
                    &template.to_string_lossy(),
                    &expanded.text,
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
            Err(BeadComposeError::OutputPathSymlink {
                path: public_path_buf(path),
            })
        }
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(BeadComposeError::OutputPathInvalid {
            path: public_path_buf(path),
            rule: "destination must be a regular file or a nonexistent path".into(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(BeadComposeError::OutputPathInvalid {
            path: public_path_buf(path),
            rule: format!("cannot inspect output destination: {error}"),
        }),
    }
}

pub(crate) fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), BeadComposeError> {
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
        .ok_or_else(|| BeadComposeError::TemplatePathInvalid {
            path: public_path_buf(path),
        })?;
    let name = path
        .file_name()
        .ok_or_else(|| BeadComposeError::TemplatePathInvalid {
            path: public_path_buf(path),
        })?;
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
pub(crate) fn replace_output(temporary: &Path, path: &Path) -> Result<(), BeadComposeError> {
    fs::rename(temporary, path).map_err(|error| render_error(&error))
}

#[cfg(windows)]
pub(crate) fn replace_output(temporary: &Path, path: &Path) -> Result<(), BeadComposeError> {
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

fn escape_mode(template: &Path) -> sc_composer::TemplateEscapeMode {
    if template.to_string_lossy().ends_with(".formula.json.j2") {
        // Formula templates retain their literal JSON quotes.  This mirrors
        // the documented sc-compose legacy JSON shape while the deliberately
        // distinct triple braces preserve Beads runtime placeholders.
        sc_composer::TemplateEscapeMode::Json(sc_composer::JsonEscapeMode::Legacy)
    } else if template.to_string_lossy().ends_with(".formula.toml.j2") {
        sc_composer::TemplateEscapeMode::Toml
    } else {
        sc_composer::TemplateEscapeMode::Json(sc_composer::JsonEscapeMode::Auto)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::{Map, json};

    use super::{render_formula, render_formula_in_root};

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
    // template-probe): TOML formula interpolation uses TOML basic-string
    // escaping, so quotes and backslashes remain data in both single-line and
    // triple-quoted multiline rendered values.
    #[test]
    fn toml_formula_templates_embed_unescaped_quotes_and_backslashes() {
        let root = temporary_directory();
        let template = root.join("example.formula.toml.j2");
        let output = root.join("example.formula.toml");
        fs::write(
            &template,
            "single = \"{{{ x }}}\"\nmultiline = \"\"\"{{{ x }}}\"\"\"",
        )
        .expect("write template");
        let hostile = "has \"quotes\" and \\backslash\nwith a second line";
        render_formula(
            &template,
            &output,
            &Map::from_iter([(String::from("x"), json!(hostile))]),
        )
        .expect("render TOML formula");

        let rendered = fs::read_to_string(&output).expect("read output");
        let parsed: toml::Value = toml::from_str(&rendered).expect("valid TOML");
        assert_eq!(
            parsed.get("single").and_then(toml::Value::as_str),
            Some(hostile)
        );
        assert_eq!(
            parsed.get("multiline").and_then(toml::Value::as_str),
            Some(hostile)
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

    #[test]
    fn includes_use_working_root_and_recursive_relative_lookup_before_rendering() {
        let root = temporary_directory();
        fs::create_dir_all(root.join("templates/parts")).expect("parts directory");
        fs::create_dir_all(root.join("shared")).expect("shared directory");
        let template = root.join("templates/main.formula.toml.j2");
        fs::write(&template, "@<parts/one.toml.j2>\n").expect("main template");
        fs::write(
            root.join("templates/parts/one.toml.j2"),
            "@<two.toml.j2>\n@<shared/root.toml.j2>\n",
        )
        .expect("first include");
        fs::write(
            root.join("templates/parts/two.toml.j2"),
            "title = \"{{{ title }}}\"\nruntime = \"{{ bead_var }}\"\n",
        )
        .expect("relative include");
        fs::write(root.join("shared/root.toml.j2"), "root = true\n")
            .expect("root fallback include");
        let output = root.join("out.formula.toml");
        render_formula_in_root(
            &template,
            &output,
            &Map::from_iter([(String::from("title"), json!("Included"))]),
            &root,
        )
        .expect("expand and render");
        assert_eq!(
            fs::read_to_string(output).expect("output"),
            "title = \"Included\"\nruntime = \"{{ bead_var }}\"\nroot = true"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn failed_include_expansion_preserves_existing_output() {
        let root = temporary_directory();
        let template = root.join("main.formula.toml.j2");
        let output = root.join("out.formula.toml");
        fs::write(&output, "previous formula").expect("existing output");
        for (text, expected) in [
            ("@<main.formula.toml.j2>\n", "include cycle"),
            (
                "@<../outside-missing.toml.j2>\n",
                "escapes confinement root",
            ),
            ("@<missing.toml.j2>\n", "include file not found"),
        ] {
            fs::write(&template, text).expect("template");
            let error = render_formula_in_root(&template, &output, &Map::new(), &root)
                .expect_err("invalid include");
            assert!(error.to_string().contains(expected), "{error}");
            assert_eq!(
                fs::read_to_string(&output).expect("output"),
                "previous formula"
            );
        }
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
