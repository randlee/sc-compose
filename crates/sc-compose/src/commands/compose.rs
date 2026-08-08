use std::collections::BTreeMap;

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
    let mut diagnostics = report
        .warnings
        .iter()
        .chain(report.errors.iter())
        .cloned()
        .collect::<Vec<_>>();
    diagnostics.extend(lint_diagnostics);
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
