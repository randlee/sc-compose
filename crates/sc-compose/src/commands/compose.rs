use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{ComposeMode, ComposeRequest, CompositionObserver, Diagnostic, DiagnosticCode};

use crate::cli::{Mode, RenderArgs, RenderBehaviorArgs, ResolveArgs, ValidateArgs};
use crate::path_utils::to_forward_slash;
use crate::render_request::{build_request, read_block_pair};
use crate::{CommandError, print_diagnostic_messages, print_json};

pub(crate) fn run_render(
    args: &RenderArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let request = build_request(
        &args.common,
        read_block_pair(&args.common.input, &args.render)?,
        std::collections::BTreeMap::default(),
    )?;
    execute_render(&request, &args.render, observer)
}

pub(crate) fn execute_render(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let result =
        sc_composer::compose_with_observer(request, observer).map_err(CommandError::compose)?;
    let output_path = args.output.clone();
    let derived_path = derived_output_path(request, output_path.as_deref());
    let would_change = render_would_change(&derived_path, &result.rendered_text);
    let bytes_written = if args.dry_run {
        None
    } else if let Some(output) = output_path.as_ref() {
        std::fs::write(output, &result.rendered_text).map_err(|error| {
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
        Some(result.rendered_text.len())
    };

    if args.json {
        let payload = if args.dry_run {
            serde_json::json!({
                "would_write": to_forward_slash(&derived_path),
                "would_change": would_change,
                "template": to_forward_slash(&result.resolve_result.resolved_path),
                "rendered_preview": result.rendered_text,
            })
        } else {
            serde_json::json!({
                "output_path": output_path
                    .as_ref()
                    .map_or_else(|| "stdout".to_owned(), |path| to_forward_slash(path)),
                "bytes_written": bytes_written.unwrap_or_default(),
                "template": to_forward_slash(&result.resolve_result.resolved_path),
            })
        };
        print_json(payload, result.warnings).map_err(CommandError::usage)?;
    } else if args.dry_run {
        println!(
            "template: {}",
            result.resolve_result.resolved_path.display()
        );
        println!("would_write: {}", derived_path.display());
        println!("would_change: {would_change}");
        if !result.warnings.is_empty() {
            println!();
            print_diagnostic_messages(&result.warnings);
        }
        println!();
        println!("{}", result.rendered_text);
    } else {
        println!("{}", result.rendered_text);
    }

    Ok(crate::exit_codes::SUCCESS)
}

pub(crate) fn run_resolve(
    args: &ResolveArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    if matches!(args.common.mode, Mode::File) {
        return Err(CommandError::usage_with_code(
            anyhow!("resolve is only supported in profile mode"),
            DiagnosticCode::ErrConfigMode,
        ));
    }
    let request = build_request(
        &args.common,
        (None, None),
        std::collections::BTreeMap::default(),
    )?;
    let result = sc_composer::resolve_profile_with_observer(&request, observer)
        .map_err(CommandError::compose)?;
    if args.json {
        let payload = serde_json::json!({
            "resolved_path": to_forward_slash(&result.resolved_path),
            "search_trace": result
                .attempted_paths
                .iter()
                .map(|path| to_forward_slash(path))
                .collect::<Vec<_>>(),
            "found": true,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("{}", result.resolved_path.display());
        for path in result.attempted_paths {
            println!("searched: {}", path.display());
        }
    }
    Ok(crate::exit_codes::SUCCESS)
}

pub(crate) fn run_validate(
    args: &ValidateArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let request = build_request(
        &args.common,
        (None, None),
        std::collections::BTreeMap::default(),
    )?;
    let report =
        sc_composer::validate_with_observer(&request, observer).map_err(CommandError::compose)?;
    let diagnostics = report
        .warnings
        .iter()
        .chain(report.errors.iter())
        .cloned()
        .collect::<Vec<_>>();
    if args.json {
        print_json(
            serde_json::json!({
                "valid": report.ok,
            }),
            diagnostics,
        )
        .map_err(CommandError::usage)?;
    } else if diagnostics.is_empty() {
        println!("valid");
    } else {
        for diagnostic in &diagnostics {
            println!("{}", format_diagnostic(diagnostic));
        }
    }
    Ok(if report.ok {
        crate::exit_codes::SUCCESS
    } else {
        crate::exit_codes::VALIDATION_OR_RENDER_FAIL
    })
}

fn derived_output_path(request: &ComposeRequest, explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    match &request.mode {
        ComposeMode::File { template_path } => strip_j2_suffix(template_path),
        ComposeMode::Profile { name, .. } => request.root.as_path().join(".prompts").join(format!(
            "{}-{}.md",
            name,
            ulid::Ulid::new()
        )),
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

fn format_diagnostic(diagnostic: &Diagnostic) -> String {
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
