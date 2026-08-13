use std::collections::BTreeMap;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::{
    ComposeRequest, CompositionObserver, Diagnostic, ExpandedTemplate, Frontmatter, ParsedTemplate,
    RenderOutcomeEvent, Renderer, ResolveResult,
};

use super::compose_output::emit_render_output;
use crate::cli::{RenderArgs, RenderBehaviorArgs};
use crate::path_utils::to_forward_slash;
use crate::{CommandError, exit_codes};

pub(crate) fn execute_render(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    execute_render_with_extra_warnings(request, args, observer, Vec::new())
}

pub(super) fn execute_render_with_extra_warnings(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    observer: &mut dyn CompositionObserver,
    mut extra_warnings: Vec<Diagnostic>,
) -> Result<i32, CommandError> {
    let result =
        sc_composer::compose_with_observer(request, observer).map_err(CommandError::compose)?;
    extra_warnings.extend(result.warnings);
    emit_render_output(
        request,
        args,
        &result.resolve_result.resolved_path,
        &result.rendered_text,
        extra_warnings,
    )?;

    Ok(exit_codes::SUCCESS)
}

pub(super) fn execute_render_with_expanded(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    observer: &mut dyn CompositionObserver,
    resolve_result: ResolveResult,
    expanded: ExpandedTemplate,
    mut extra_warnings: Vec<Diagnostic>,
) -> Result<i32, CommandError> {
    let result = sc_composer::compose_with_observer_and_expanded(
        request,
        observer,
        resolve_result,
        expanded,
    )
    .map_err(CommandError::compose)?;
    extra_warnings.extend(result.warnings);
    emit_render_output(
        request,
        args,
        &result.resolve_result.resolved_path,
        &result.rendered_text,
        extra_warnings,
    )?;

    Ok(exit_codes::SUCCESS)
}

pub(super) fn execute_custom_delimiter_render(
    request: &ComposeRequest,
    args: &RenderArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let (open, close) = custom_variable_delimiters(args)?;
    let (report, expanded) = sc_composer::validate_with_observer_and_delimiters_with_expansion(
        request,
        observer,
        Some((&open, &close)),
    )
    .map_err(CommandError::compose)?;
    if !report.ok {
        return Err(validation_report_error(report.errors));
    }
    let resolve_result = report.resolve_result;
    let root_passes = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| (path == &resolve_result.resolved_path).then(|| passes.clone()))
        .unwrap_or_default();
    let parsed = ParsedTemplate::from_parts_validated(root_passes.clone(), expanded.text.clone())
        .map_err(CommandError::compose)?;
    let template_name = resolve_result
        .resolved_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("inline")
        .to_owned();
    let json_escape_mode = request
        .policy
        .json_escape_mode
        .or_else(|| {
            root_passes
                .first()
                .and_then(sc_composer::Frontmatter::json_escape_mode)
        })
        .unwrap_or_default();
    let rendered_text =
        Renderer::with_delimiters_and_json_escape_mode(&open, &close, json_escape_mode)
            .map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!(error),
                    sc_composer::DiagnosticCode::ErrConfigParse,
                )
            })?
            .render_named(
                &template_name,
                parsed.body(),
                build_custom_render_context(request, &resolve_result.resolved_path, &root_passes),
            )
            .inspect_err(|error| {
                observer.on_render_outcome(&RenderOutcomeEvent {
                    rendered_bytes: None,
                    code: error.code(),
                });
            })
            .map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!(error),
                    sc_composer::DiagnosticCode::ErrConfigParse,
                )
            })?;
    observer.on_render_outcome(&RenderOutcomeEvent {
        rendered_bytes: Some(rendered_text.len()),
        code: None,
    });
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
        report.warnings,
    )?;
    Ok(exit_codes::SUCCESS)
}

fn custom_variable_delimiters(args: &RenderArgs) -> Result<(String, String), CommandError> {
    if let Some(brace_count) = args.brace_count {
        let brace_count = usize::from(brace_count);
        return Ok(("{".repeat(brace_count), "}".repeat(brace_count)));
    }

    let delimiters = args.variable_delimiters.clone().ok_or_else(|| {
        CommandError::usage_with_code(
            anyhow!(
                "--brace-count or --variable-delimiters is required for custom delimiter rendering"
            ),
            sc_composer::DiagnosticCode::ErrConfigParse,
        )
    })?;
    Ok((delimiters[0].clone(), delimiters[1].clone()))
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
        serde_json::Value::String(current_username()),
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

fn current_username() -> String {
    environment_value("USER")
        .or_else(|| environment_value("USERNAME"))
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn environment_value(name: &str) -> Option<std::ffi::OsString> {
    std::env::vars_os().find_map(|(key, value)| (key == name).then_some(value))
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

fn validation_report_error(errors: Vec<sc_composer::Diagnostic>) -> CommandError {
    let diagnostic_code = errors.first().map(|diagnostic| diagnostic.code);
    let message = errors.first().map_or_else(
        || "render request failed validation".to_owned(),
        |diagnostic| diagnostic.message.clone(),
    );
    CommandError {
        exit_code: exit_codes::VALIDATION_OR_RENDER_FAIL,
        diagnostic_code,
        diagnostics: errors,
        recovery_hints: Vec::new(),
        error: anyhow!(message),
    }
}
