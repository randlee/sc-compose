use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{
    ComposeRequest, CompositionObserver, Diagnostic, DiagnosticCode, DiagnosticSeverity,
    ValidationOutcomeEvent,
};

use crate::cli::RenderBehaviorArgs;
use crate::path_utils::to_forward_slash;
use crate::{CommandError, print_diagnostic_messages, print_json};

pub(super) fn emit_render_output(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    resolved_path: &Path,
    rendered_text: &str,
    warnings: Vec<Diagnostic>,
) -> Result<(), CommandError> {
    let output_path = args.output.clone();
    let derived_path = derived_output_path(request, output_path.as_deref());
    let would_change = render_would_change(&derived_path, rendered_text);
    let bytes_written = if args.dry_run {
        None
    } else if let Some(output) = output_path.as_ref() {
        std::fs::write(output, rendered_text).map_err(|error| {
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
            serde_json::json!({
                "would_write": to_forward_slash(&derived_path),
                "would_change": would_change,
                "template": to_forward_slash(resolved_path),
                "rendered_preview": rendered_text,
            })
        } else if output_path.is_none() {
            serde_json::json!({
                "output_path": "stdout",
                "bytes_written": bytes_written.unwrap_or_default(),
                "template": to_forward_slash(resolved_path),
                "body": rendered_text,
            })
        } else {
            serde_json::json!({
                "output_path": output_path
                    .as_ref()
                    .map_or_else(|| "stdout".to_owned(), |path| to_forward_slash(path)),
                "bytes_written": bytes_written.unwrap_or_default(),
                "template": to_forward_slash(resolved_path),
            })
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
        println!("{rendered_text}");
    }

    Ok(())
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
