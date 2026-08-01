use anyhow::anyhow;
use sc_composer::{DiagnosticCode, RecoveryHint, RecoveryHintKind};

use crate::cli::{CommonArgs, VerifyArgs, parse_pass_inputs};
use crate::observer_impl::CliObserver;
use crate::path_utils::to_forward_slash;
use crate::render_request::{build_multi_pass_request, build_request};
use crate::{CommandError, print_json};

pub(crate) fn run_verify(
    args: &VerifyArgs,
    observer: &mut CliObserver,
) -> Result<i32, CommandError> {
    let common = effective_common_args(args)?;
    let result = if args.all {
        let pass_inputs =
            parse_pass_inputs(std::env::args_os(), "verify").map_err(pass_inputs_parse_error)?;
        let mut request = build_multi_pass_request(
            &common,
            (None, None),
            std::collections::BTreeMap::default(),
            &pass_inputs,
        )?;
        apply_builtin_overrides(&mut request, &args.builtin_vars)?;
        sc_composer::verify_with_observer(&request, &args.deployed, observer)
            .map_err(CommandError::compose)?
    } else {
        let mut request =
            build_request(&common, (None, None), std::collections::BTreeMap::default())?;
        apply_builtin_overrides(&mut request, &args.builtin_vars)?;
        sc_composer::verify_with_observer(&request, &args.deployed, observer)
            .map_err(CommandError::compose)?
    };

    if args.json {
        print_json(
            serde_json::json!({
                "clean": result.clean,
                "deployed_path": to_forward_slash(&result.deployed_path),
                "resolved_template_path": to_forward_slash(&result.resolved_template_path),
                "diff": result.diff,
            }),
            result.warnings,
        )
        .map_err(CommandError::usage)?;
    } else if result.clean {
        println!(
            "OK  {} -> {}",
            result.resolved_template_path.display(),
            result.deployed_path.display()
        );
    } else {
        eprintln!(
            "DRIFT detected: {} != {}",
            result.resolved_template_path.display(),
            result.deployed_path.display()
        );
        if let Some(diff) = result.diff.as_deref().filter(|_| !args.quiet) {
            eprintln!("{diff}");
        }
    }

    Ok(if result.clean {
        crate::exit_codes::SUCCESS
    } else {
        1
    })
}

fn effective_common_args(args: &VerifyArgs) -> Result<CommonArgs, CommandError> {
    if args.common.file.is_some() && args.against.is_some() {
        return Err(CommandError::usage_with_code(
            anyhow!("pass only one of --file or --against for verify"),
            DiagnosticCode::ErrConfigParse,
        ));
    }
    if matches!(args.common.mode, crate::cli::Mode::File)
        && args.common.file.is_none()
        && args.against.is_none()
    {
        return Err(CommandError::usage_with_code_and_hints(
            anyhow!("--against is required in file mode"),
            DiagnosticCode::ErrConfigMode,
            vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "pass --against <template> when verify runs in file mode".to_owned(),
            })],
        ));
    }

    let mut common = args.common.clone();
    if common.file.is_none() {
        common.file.clone_from(&args.against);
    }
    Ok(common)
}

fn pass_inputs_parse_error(error: String) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!(error),
        DiagnosticCode::ErrConfigParse,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "when using --all, declare each --pass N group before its --var/--var-file arguments"
                .to_owned(),
        })],
    )
}

fn apply_builtin_overrides(
    request: &mut sc_composer::ComposeRequest,
    overrides: &[(String, String)],
) -> Result<(), CommandError> {
    for (name, value) in overrides {
        if !sc_composer::BUILTIN_VARIABLE_NAMES.contains(&name.as_str()) {
            return Err(CommandError::usage_with_code_and_hints(
                anyhow!("invalid builtin override `{name}`"),
                DiagnosticCode::ErrConfigParse,
                vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                    key: format!(
                        "use one of {}",
                        sc_composer::BUILTIN_VARIABLE_NAMES.join(", ")
                    ),
                })],
            ));
        }
        let variable = sc_composer::VariableName::new(name.clone()).map_err(|error| {
            CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
        })?;
        request
            .vars_input
            .insert(variable, serde_json::Value::String(value.clone()));
    }
    Ok(())
}
