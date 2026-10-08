use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{
    ComposeRequest, CompositionObserver, Diagnostic, DiagnosticCode, DiagnosticSeverity,
    RecoveryHint, RecoveryHintKind, ValidationOutcomeEvent,
};

use crate::cli::RenderBehaviorArgs;
use crate::path_utils::to_forward_slash;
use crate::{CommandError, print_diagnostic_messages, print_json};

pub(super) fn emit_render_output(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    resolved_path: &Path,
    checked_output: &sc_composer::CheckedOutput,
    warnings: Vec<Diagnostic>,
    render_check: Option<sc_composer::RenderCheckReport>,
) -> Result<(), CommandError> {
    let rendered_text = checked_output.body();
    let output_path: Option<&Path> = args.output.as_deref().or(args.append.as_deref());
    let derived_path = derived_output_path(request, output_path);
    let would_change = render_would_change(&derived_path, rendered_text);
    let bytes_written = if args.dry_run {
        None
    } else if let Some(output) = args.append.as_ref() {
        Some(append_json_record(output, resolved_path, rendered_text)?)
    } else if let Some(output) = output_path {
        let mut file = std::fs::File::create(output).map_err(|error| {
            CommandError::render_write(
                anyhow!(error).context(format!("failed to write {}", output.display())),
            )
        })?;
        checked_output.emit(&mut file).map_err(|error| {
            CommandError::render_write(
                anyhow!(error).context(format!("failed to write {}", output.display())),
            )
        })?;
        Some(
            usize::try_from(
                std::fs::metadata(output)
                    .map_err(|error| {
                        CommandError::render_write(
                            anyhow!(error).context(format!("failed to stat {}", output.display())),
                        )
                    })?
                    .len(),
            )
            .map_err(|error| {
                CommandError::render_write(
                    anyhow!(error)
                        .context(format!("output too large to report {}", output.display())),
                )
            })?,
        )
    } else {
        // Plain stdout uses println!, so the logical render target includes
        // its trailing newline even though the JSON body does not.
        Some(rendered_text.len() + 1)
    };

    if args.json {
        let payload = if args.dry_run {
            let mut payload = serde_json::json!({
                "would_write": to_forward_slash(&derived_path),
                "would_change": would_change,
                "template": to_forward_slash(resolved_path),
                "rendered_preview": rendered_text,
            });
            add_render_check(&mut payload, render_check);
            payload
        } else if output_path.is_none() {
            let mut payload = serde_json::json!({
                "output_path": "stdout",
                "bytes_written": bytes_written.unwrap_or_default(),
                "template": to_forward_slash(resolved_path),
                "body": rendered_text,
            });
            add_render_check(&mut payload, render_check);
            payload
        } else {
            let mut payload = serde_json::json!({
                "output_path": output_path
                    .map_or_else(|| "stdout".to_owned(), to_forward_slash),
                "bytes_written": bytes_written.unwrap_or_default(),
                "template": to_forward_slash(resolved_path),
            });
            if args.append.is_some() {
                payload["appended"] = serde_json::Value::Bool(true);
            }
            add_render_check(&mut payload, render_check);
            payload
        };
        print_json(payload, warnings).map_err(CommandError::usage)?;
    } else if args.dry_run {
        println!("template: {}", resolved_path.display());
        println!("would_write: {}", derived_path.display());
        println!("would_change: {would_change}");
        if !warnings.is_empty() {
            println!();
            print_diagnostic_messages(&warnings);
        }
        println!();
        println!("{rendered_text}");
    } else {
        let mut stdout = std::io::stdout().lock();
        checked_output
            .emit(&mut stdout)
            .and_then(|()| writeln!(stdout))
            .map_err(|error| CommandError::render_write(anyhow!(error)))?;
    }

    Ok(())
}

fn append_json_record(
    path: &Path,
    template_path: &Path,
    rendered: &str,
) -> Result<usize, CommandError> {
    let checked = sc_composer::check_rendered_output(
        sc_composer::OutputFormat::Json,
        template_path,
        rendered,
    )
    .map_err(CommandError::render_check)?;
    // The body already passed JSON syntax validation. Inspect only its root
    // token, avoiding a numeric conversion that would reject valid exponents.
    if !checked.body().trim_start().starts_with('{') {
        return Err(CommandError::render_append(
            anyhow!("--append requires the rendered output to be a JSON object"),
            DiagnosticCode::ErrRenderAppendNotObject,
            vec![RecoveryHint::new(RecoveryHintKind::InspectInput {
                description: "the rendered output; --append requires a JSON object".to_owned(),
            })],
        ));
    }
    let object: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
        serde_json::from_str(checked.body()).map_err(|error| {
            CommandError::render_append(
                anyhow!(error).context(format!(
                    "failed to parse checked JSON from template {}",
                    template_path.display()
                )),
                DiagnosticCode::ErrRenderJsonMalformed,
                Vec::new(),
            )
        })?;
    let mut line = serde_json::to_string(&object)
        .map_err(|error| CommandError::render_write(anyhow!(error)))?;
    compact_json_whitespace(&mut line);
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| {
            CommandError::render_write(
                anyhow!(error).context(format!("failed to open {}", path.display())),
            )
        })?;
    file.lock().map_err(|error| {
        CommandError::render_write(anyhow!(error).context("failed to lock append target"))
    })?;
    let original_len = file
        .metadata()
        .map_err(|error| {
            CommandError::render_write(anyhow!(error).context(format!(
                "failed to inspect append target {}",
                path.display()
            )))
        })?
        .len();
    if original_len > 0 {
        file.seek(SeekFrom::End(-1))
            .map_err(|error| CommandError::render_write(anyhow!(error)))?;
        let mut tail = [0];
        file.read_exact(&mut tail)
            .map_err(|error| CommandError::render_write(anyhow!(error)))?;
        if tail[0] != b'\n' {
            return Err(CommandError::render_append(
                anyhow!("append target must end with a newline"),
                DiagnosticCode::ErrRenderAppendNoFinalNewline,
                vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                    path: path.to_path_buf(),
                })],
            ));
        }
    }
    file.seek(SeekFrom::End(0))
        .map_err(|error| CommandError::render_write(anyhow!(error)))?;
    if let Err(error) = file.write_all(line.as_bytes()).and_then(|()| file.flush()) {
        let rollback = file.set_len(original_len);
        return Err(append_failure(path, error, rollback));
    }
    Ok(line.len())
}

/// Map a failed append write, and the result of rolling the file back, to the
/// command error. A failed rollback may leave a partial last line, so it names
/// the append target and carries an inspect-path recovery hint.
fn append_failure(
    path: &Path,
    error: std::io::Error,
    rollback: std::io::Result<()>,
) -> CommandError {
    match rollback {
        Ok(()) => CommandError::render_write(
            anyhow!(error).context("failed to append JSON record; restored append target"),
        ),
        Err(rollback_error) => CommandError::render_append(
            anyhow!(error).context(format!(
                "failed to append JSON record; rollback also failed ({rollback_error}); a partial last line may remain; inspect and restore the append target {}",
                path.display()
            )),
            DiagnosticCode::ErrRenderWrite,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.to_path_buf(),
            })],
        ),
    }
}

/// Compact already validated JSON without parsing numeric or string lexemes again.
fn compact_json_whitespace(json: &mut String) {
    let mut in_string = false;
    let mut escaped = false;
    json.retain(|character| {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            true
        } else if character == '"' {
            in_string = true;
            true
        } else {
            !matches!(character, ' ' | '\t' | '\r' | '\n')
        }
    });
}

fn add_render_check(
    payload: &mut serde_json::Value,
    report: Option<sc_composer::RenderCheckReport>,
) {
    if let Some(report) = report {
        payload["render_check"] = serde_json::to_value(report).unwrap_or_else(|_| {
            serde_json::json!({
                "state": "contract_invalid",
                "diagnostics": []
            })
        });
    }
}

pub(super) fn emit_single_pass_all_warning(observer: &mut dyn CompositionObserver) {
    observer.on_validation_outcome(&ValidationOutcomeEvent {
        warnings: vec![single_pass_all_warning()],
        errors: Vec::new(),
    });
}

pub(super) fn single_pass_all_warning() -> Diagnostic {
    Diagnostic::new(
        DiagnosticSeverity::Warning,
        DiagnosticCode::WarnConfigSinglePassAllFallback,
        "--all requested for a template without stacked headers; proceeding in single-pass mode",
    )
}

pub(super) fn format_diagnostic(diagnostic: &Diagnostic) -> String {
    let severity = diagnostic.severity.to_string();
    let location =
        diagnostic
            .path
            .as_ref()
            .map(|path| match (diagnostic.line, diagnostic.column) {
                (Some(line), Some(column)) => format!("{}:{line}:{column}", path.display()),
                _ => path.display().to_string(),
            });
    match location {
        Some(location) => format!(
            "[{severity}] {}: {} ({location})",
            diagnostic.code.as_str(),
            diagnostic.message
        ),
        None => format!(
            "[{severity}] {}: {}",
            diagnostic.code.as_str(),
            diagnostic.message
        ),
    }
}

fn derived_output_path(request: &ComposeRequest, explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    match &request.mode {
        sc_composer::ComposeMode::File { template_path } => strip_j2_suffix(template_path),
        sc_composer::ComposeMode::Profile { name, .. } => request
            .root
            .as_path()
            .join(".prompts")
            .join(format!("{}-{}.md", name, ulid::Ulid::new())),
    }
}

fn render_would_change(output_path: &Path, rendered_text: &str) -> bool {
    match std::fs::read(output_path) {
        Ok(existing) => existing != rendered_text.as_bytes(),
        Err(_) => true,
    }
}

fn strip_j2_suffix(path: &Path) -> PathBuf {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path.to_path_buf();
    };
    let Some(stripped) = file_name.strip_suffix(".j2") else {
        return path.to_path_buf();
    };

    let mut rebuilt = path.to_path_buf();
    rebuilt.set_file_name(stripped);
    rebuilt
}

#[cfg(test)]
mod append_failure_tests {
    use super::append_failure;
    use sc_composer::{DiagnosticCode, RecoveryHint, RecoveryHintKind};
    use std::io::{Error, ErrorKind};
    use std::path::Path;

    #[test]
    fn failed_rollback_names_the_target_and_hints_inspect_path() {
        let path = Path::new("out/records.jsonl");
        let error = append_failure(
            path,
            Error::new(ErrorKind::StorageFull, "disk full"),
            Err(Error::other("truncate failed")),
        );
        assert_eq!(error.diagnostic_code, Some(DiagnosticCode::ErrRenderWrite));
        assert_eq!(
            error.recovery_hints,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.to_path_buf()
            })]
        );
        let message = format!("{:#}", error.error);
        assert!(message.contains("out/records.jsonl"), "{message}");
        assert!(message.contains("rollback also failed"), "{message}");
        assert!(error.diagnostics[0].message.contains("out/records.jsonl"));
    }

    #[test]
    fn restored_target_needs_no_recovery_hint() {
        let error = append_failure(
            Path::new("out/records.jsonl"),
            Error::new(ErrorKind::StorageFull, "disk full"),
            Ok(()),
        );
        assert!(error.recovery_hints.is_empty());
        assert!(format!("{:#}", error.error).contains("restored append target"));
    }
}
