use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{
    ComposeMode, ComposeRequest, CompositionObserver, Diagnostic, DiagnosticCode, ExpandedTemplate,
    Frontmatter, ParsedTemplate, Renderer, ResolveResult,
};

use crate::cli::{
    Mode, RenderArgs, RenderBehaviorArgs, ResolveArgs, ValidateArgs, parse_pass_inputs,
};
use crate::path_utils::to_forward_slash;
use crate::render_request::{
    build_multi_pass_request, build_request, read_block_pair,
    read_block_pair_with_extra_stdin_reads,
};
use crate::{CommandError, print_diagnostic_messages, print_json};

pub(crate) fn run_render(
    args: &RenderArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    if args.all {
        let pass_inputs =
            parse_pass_inputs("render").map_err(|error| CommandError::usage(anyhow!(error)))?;
        let stdin_reads = pass_inputs
            .iter()
            .flat_map(|pass| pass.var_files.iter())
            .filter(|path| path.as_str() == "-")
            .count();
        let request = build_multi_pass_request(
            &args.common,
            read_block_pair_with_extra_stdin_reads(&args.common.input, &args.render, stdin_reads)?,
            BTreeMap::default(),
            &pass_inputs,
        )?;
        let (_, _, root_passes) = preflight_template(&request)?;
        if root_passes.len() <= 1 {
            emit_single_pass_all_warning();
        } else if pass_inputs.len() != root_passes.len() {
            return Err(CommandError::usage_with_code(
                anyhow!(
                    "--all requires exactly {} --pass N groups for this template, got {}",
                    root_passes.len(),
                    pass_inputs.len()
                ),
                DiagnosticCode::ErrConfigParse,
            ));
        }
        return execute_render(&request, &args.render, observer);
    }

    if args.brace_count.is_some() || args.variable_delimiters.is_some() {
        let request = build_request(
            &args.common,
            read_block_pair(&args.common.input, &args.render)?,
            BTreeMap::default(),
        )?;
        return execute_custom_delimiter_render(&request, args, observer);
    }

    let request = build_request(
        &args.common,
        read_block_pair(&args.common.input, &args.render)?,
        BTreeMap::default(),
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
    emit_render_output(
        request,
        args,
        &result.resolve_result.resolved_path,
        &result.rendered_text,
        result.warnings,
    )?;

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
    let request = if args.all {
        let pass_inputs =
            parse_pass_inputs("validate").map_err(|error| CommandError::usage(anyhow!(error)))?;
        let stdin_reads = pass_inputs
            .iter()
            .flat_map(|pass| pass.var_files.iter())
            .filter(|path| path.as_str() == "-")
            .count();
        let request = build_multi_pass_request(
            &args.common,
            read_block_pair_with_extra_stdin_reads(
                &args.common.input,
                &RenderBehaviorArgs::default(),
                stdin_reads,
            )?,
            BTreeMap::default(),
            &pass_inputs,
        )?;
        let (_, _, root_passes) = preflight_template(&request)?;
        if root_passes.len() <= 1 {
            emit_single_pass_all_warning();
        }
        request
    } else {
        build_request(&args.common, (None, None), BTreeMap::default())?
    };
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

fn execute_custom_delimiter_render(
    request: &ComposeRequest,
    args: &RenderArgs,
    _observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let (resolve_result, expanded, root_passes) = preflight_template(request)?;
    let (open, close) = if let Some(brace_count) = args.brace_count {
        let brace_count = usize::from(brace_count);
        ("{".repeat(brace_count), "}".repeat(brace_count))
    } else {
        let delimiters = args.variable_delimiters.clone().ok_or_else(|| {
            CommandError::usage(anyhow!(
                "--brace-count or --variable-delimiters is required for custom delimiter rendering"
            ))
        })?;
        (delimiters[0].clone(), delimiters[1].clone())
    };
    let rendered_text = Renderer::with_delimiters(&open, &close)
        .render(
            ParsedTemplate::from_parts(root_passes.clone(), expanded.text.clone()).body(),
            build_custom_render_context(request, &resolve_result.resolved_path, &root_passes),
        )
        .map_err(|error| {
            CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
        })?;
    let rendered_text = assemble_output(
        &rendered_text,
        request.guidance_block.as_deref(),
        request.user_prompt.as_deref(),
    );
    emit_render_output(
        request,
        &args.render,
        &resolve_result.resolved_path,
        &rendered_text,
        Vec::new(),
    )?;
    Ok(crate::exit_codes::SUCCESS)
}

fn emit_render_output(
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
        Some(rendered_text.len())
    };

    if args.json {
        let payload = if args.dry_run {
            serde_json::json!({
                "would_write": to_forward_slash(&derived_path),
                "would_change": would_change,
                "template": to_forward_slash(resolved_path),
                "rendered_preview": rendered_text,
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

fn preflight_template(
    request: &ComposeRequest,
) -> Result<(ResolveResult, ExpandedTemplate, Vec<Frontmatter>), CommandError> {
    let resolve_result =
        sc_composer::resolve_template_path(request).map_err(CommandError::compose)?;
    let expanded = sc_composer::expand_includes(
        &resolve_result.resolved_path,
        &request.root,
        &request.policy,
    )
    .map_err(CommandError::compose)?;
    let root_passes = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| (path == &resolve_result.resolved_path).then(|| passes.clone()))
        .unwrap_or_default();
    Ok((resolve_result, expanded, root_passes))
}

fn emit_single_pass_all_warning() {
    eprintln!(
        "warning: --all requested for a template without stacked headers; proceeding in single-pass mode"
    );
}

fn build_custom_render_context(
    request: &ComposeRequest,
    resolved_path: &Path,
    root_passes: &[Frontmatter],
) -> BTreeMap<String, serde_json::Value> {
    let mut context = BTreeMap::new();
    if let Some(frontmatter) = root_passes.first() {
        for (name, value) in frontmatter.defaults() {
            context.insert(name.to_string(), value.clone());
        }
    }
    for (name, value) in &request.vars_defaults {
        context.insert(name.to_string(), value.clone());
    }
    context.insert(
        "TEMPLATE_NAME".to_owned(),
        serde_json::Value::String(to_forward_slash(resolved_path)),
    );
    context.insert(
        "HOSTNAME".to_owned(),
        serde_json::Value::String(
            std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("COMPUTERNAME"))
                .unwrap_or_else(|_| "unknown".to_owned()),
        ),
    );
    context.insert(
        "USERNAME".to_owned(),
        serde_json::Value::String(
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_owned()),
        ),
    );
    let now = time::OffsetDateTime::now_utc();
    context.insert(
        "RENDER_DATE".to_owned(),
        serde_json::Value::String(
            now.format(time::macros::format_description!("[year]-[month]-[day]"))
                .unwrap_or_else(|_| {
                    format!(
                        "{:04}-{:02}-{:02}",
                        now.year(),
                        u8::from(now.month()),
                        now.day()
                    )
                }),
        ),
    );
    context.insert(
        "RENDER_TIMESTAMP".to_owned(),
        serde_json::Value::String(
            now.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| {
                    format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                        now.year(),
                        u8::from(now.month()),
                        now.day(),
                        now.hour(),
                        now.minute(),
                        now.second()
                    )
                }),
        ),
    );
    for (name, value) in &request.vars_env {
        context.insert(name.to_string(), value.clone());
    }
    for (name, value) in &request.vars_input {
        context.insert(name.to_string(), value.clone());
    }
    context
}

fn assemble_output(
    profile_body: &str,
    guidance_block: Option<&str>,
    user_prompt: Option<&str>,
) -> String {
    let mut blocks = vec![profile_body.trim_end().to_owned()];
    if let Some(guidance) = guidance_block.filter(|value| !value.is_empty()) {
        blocks.push(guidance.to_owned());
    }
    if let Some(prompt) = user_prompt.filter(|value| !value.is_empty()) {
        blocks.push(prompt.to_owned());
    }
    blocks.join("\n\n")
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
