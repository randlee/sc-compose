use std::collections::BTreeMap;
use std::path::Path;

use crate::cli::{Mode, RenderArgs, ResolveArgs, ValidateArgs};
use crate::commands::template_lint::lint_request;
use crate::path_utils::to_forward_slash;
use crate::render_request::build_request;
use crate::{CommandError, print_json};
use anyhow::anyhow;
use sc_composer::{CompositionObserver, DiagnosticCode};

#[path = "compose_output.rs"]
mod compose_output;
#[path = "compose_render.rs"]
mod compose_render;
#[path = "compose_request.rs"]
mod compose_request;

use compose_output::{emit_single_pass_all_warning, format_diagnostic, single_pass_all_warning};
pub(crate) use compose_render::execute_render;
use compose_render::execute_render_with_expanded;
use compose_request::{
    build_multi_pass_render_request, build_render_request, build_validate_request,
    parse_pass_input_groups, preflight_template,
};

pub(crate) fn run_render(
    args: &RenderArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    if args.all {
        let pass_inputs = parse_pass_input_groups("render")?;
        let request = build_multi_pass_render_request(args, &pass_inputs)?;
        let (resolve_result, expanded, root_passes) = preflight_template(&request)?;
        if root_passes.len() <= 1 {
            emit_single_pass_all_warning(observer);
            return compose_render::execute_render_with_expanded(
                &request,
                &args.render,
                observer,
                resolve_result,
                expanded,
                vec![single_pass_all_warning()],
            );
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
        return execute_render_with_expanded(
            &request,
            &args.render,
            observer,
            resolve_result,
            expanded,
            Vec::new(),
        );
    }

    if args.brace_count.is_some() || args.variable_delimiters.is_some() {
        let request = build_render_request(args)?;
        return compose_render::execute_custom_delimiter_render(&request, args, observer);
    }

    let request = build_render_request(args)?;
    compose_render::execute_render(&request, &args.render, observer)
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
        let pass_inputs = parse_pass_input_groups("validate")?;
        let request = build_validate_request(args, &pass_inputs)?;
        let (_, _, root_passes) = preflight_template(&request)?;
        if root_passes.len() <= 1 {
            emit_single_pass_all_warning(observer);
        }
        (request, root_passes.len() <= 1)
    } else {
        (
            build_request(&args.common, (None, None), BTreeMap::default())?,
            false,
        )
    };
    let (request, single_pass_fallback) = request;
    let mut report =
        sc_composer::validate_with_observer(&request, observer).map_err(CommandError::compose)?;
    if single_pass_fallback {
        report.warnings.push(single_pass_all_warning());
    }
    let lint_diagnostics = if args.lint {
        lint_request(&request)?
    } else {
        Vec::new()
    };
    let lint_failed = lint_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == sc_composer::DiagnosticSeverity::Error);
    let mut diagnostics = report
        .warnings
        .iter()
        .chain(report.errors.iter())
        .cloned()
        .collect::<Vec<_>>();
    diagnostics.extend(lint_diagnostics);
    if args.check_render {
        return run_checked_validate(
            &request,
            &report.resolve_result.resolved_path,
            args,
            observer,
            diagnostics,
        );
    }
    let render_report = sc_composer::RenderCheckReport::StaticOnly {
        meta: render_check_meta(&request, &report.resolve_result.resolved_path),
        diagnostics: diagnostics.clone(),
    };
    if args.json {
        let mut payload = serde_json::to_value(render_report)
            .map_err(|error| CommandError::usage(anyhow!(error)))?;
        payload["valid"] = serde_json::Value::Bool(report.ok);
        print_json(payload, diagnostics).map_err(CommandError::usage)?;
    } else if diagnostics.is_empty() {
        println!("valid (static_only)");
    } else {
        for diagnostic in &diagnostics {
            println!("{}", format_diagnostic(diagnostic));
        }
    }
    Ok(if report.ok && !lint_failed {
        crate::exit_codes::SUCCESS
    } else {
        crate::exit_codes::VALIDATION_OR_RENDER_FAIL
    })
}

fn run_checked_validate(
    request: &sc_composer::ComposeRequest,
    resolved_path: &Path,
    args: &ValidateArgs,
    observer: &mut dyn CompositionObserver,
    mut diagnostics: Vec<sc_composer::Diagnostic>,
) -> Result<i32, CommandError> {
    let meta = render_check_meta(request, resolved_path);
    let report = match sc_composer::compose_with_observer(request, observer) {
        Ok(result) => {
            let checked =
                sc_composer::check_rendered_output_with_meta(meta.clone(), &result.rendered_text)
                    .map_err(|error| {
                        let annotated = error.clone().with_failing_pass(result.failing_pass);
                        diagnostics.extend(annotated.diagnostics.clone());
                        annotated
                    });
            match checked {
                Ok(_) => sc_composer::RenderCheckReport::RenderChecked {
                    meta,
                    checked_context: context_summary(request),
                    diagnostics: diagnostics.clone(),
                },
                Err(_) => sc_composer::RenderCheckReport::RenderInvalid {
                    meta,
                    diagnostics: diagnostics.clone(),
                },
            }
        }
        Err(error) => {
            let command_error = CommandError::compose(error);
            diagnostics.extend(command_error.diagnostics.clone());
            let context_required = command_error.diagnostics.iter().any(|diagnostic| {
                matches!(
                    diagnostic.code,
                    sc_composer::DiagnosticCode::ErrValMissingRequired
                        | sc_composer::DiagnosticCode::ErrValMissingNestedField
                        | sc_composer::DiagnosticCode::ErrValShapeMismatch
                        | sc_composer::DiagnosticCode::ErrValArrayShapeMismatch
                        | sc_composer::DiagnosticCode::ErrValUnboundVariable
                )
            });
            if context_required {
                sc_composer::RenderCheckReport::ContextRequired {
                    meta,
                    diagnostics: diagnostics.clone(),
                }
            } else {
                sc_composer::RenderCheckReport::ContractInvalid {
                    meta,
                    diagnostics: diagnostics.clone(),
                }
            }
        }
    };
    let success = report.permits_emission();
    if args.json {
        let mut payload =
            serde_json::to_value(&report).map_err(|error| CommandError::usage(anyhow!(error)))?;
        payload["valid"] = serde_json::Value::Bool(success);
        print_json(payload, diagnostics).map_err(CommandError::usage)?;
    } else {
        println!("state: {}", report_state(&report));
        for diagnostic in report.diagnostics() {
            println!("{}", format_diagnostic(diagnostic));
        }
    }
    Ok(if success {
        crate::exit_codes::SUCCESS
    } else {
        crate::exit_codes::VALIDATION_OR_RENDER_FAIL
    })
}

pub(crate) fn render_check_meta(
    request: &sc_composer::ComposeRequest,
    template: &Path,
) -> sc_composer::RenderCheckMeta {
    let declared_mode = std::fs::read_to_string(template)
        .ok()
        .and_then(|source| declared_json_escape_mode(&source));
    let output_format = sc_composer::OutputFormat::from_template_path(template);
    let mode = (output_format == sc_composer::OutputFormat::Json).then(|| {
        sc_composer::resolve_json_escape_mode(request.policy.json_escape_mode, declared_mode)
    });
    sc_composer::RenderCheckMeta::for_template(template).with_json_escape_mode(mode)
}

fn declared_json_escape_mode(source: &str) -> Option<sc_composer::JsonEscapeMode> {
    let parsed = sc_composer::parse_template_document(source).ok()?;
    parsed.frontmatter()?.json_escape_mode()
}

pub(crate) fn context_summary(request: &sc_composer::ComposeRequest) -> String {
    format!(
        "{} explicit, {} environment, and {} default variables",
        request.vars_input.len(),
        request.vars_env.len(),
        request.vars_defaults.len()
    )
}

fn report_state(report: &sc_composer::RenderCheckReport) -> &'static str {
    match report {
        sc_composer::RenderCheckReport::StaticOnly { .. } => "static_only",
        sc_composer::RenderCheckReport::ContractInvalid { .. } => "contract_invalid",
        sc_composer::RenderCheckReport::ContextRequired { .. } => "context_required",
        sc_composer::RenderCheckReport::RenderInvalid { .. } => "render_invalid",
        sc_composer::RenderCheckReport::RenderChecked { .. } => "render_checked",
    }
}
