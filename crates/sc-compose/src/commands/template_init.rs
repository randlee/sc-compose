use anyhow::anyhow;
use sc_composer::{DiagnosticCode, InitPass, RecoveryHint, RecoveryHintKind, VariableName};

use crate::cli::{FrontmatterInitArgs, TemplateInitArgs, parse_pass_inputs};
use crate::path_utils::to_forward_slash;
use crate::{CommandError, print_json};

pub(crate) fn run_frontmatter_init(args: &FrontmatterInitArgs) -> Result<i32, CommandError> {
    let result = sc_composer::frontmatter_init(&args.file, args.force, args.dry_run)
        .map_err(CommandError::compose)?;
    if args.json && args.dry_run {
        print_json(
            serde_json::json!({
                "action": "frontmatter-init",
                "would_affect": [to_forward_slash(&result.target_path)],
                "changed": result.changed,
                "would_change": result.would_change,
                "skipped": !result.would_change,
                "vars": result.discovered_variables,
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else if args.json {
        print_json_frontmatter_init(&result).map_err(CommandError::usage)?;
    } else if args.dry_run {
        println!("{}", result.frontmatter_text);
    }
    Ok(crate::exit_codes::SUCCESS)
}

pub(crate) fn run_template_init(args: &TemplateInitArgs) -> Result<i32, CommandError> {
    let pass_inputs = parse_pass_inputs("template-init").map_err(template_init_parse_error)?;
    if pass_inputs.is_empty() {
        return Err(CommandError::usage_with_code_and_hints(
            anyhow!("template-init requires at least one --pass N group"),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "pass one or more --pass N groups followed by --var key=value replacements"
                    .to_owned(),
            })],
        ));
    }

    let init_passes = pass_inputs
        .iter()
        .map(|pass| {
            let variables = pass
                .vars
                .iter()
                .map(|(name, value)| {
                    VariableName::new(name.clone())
                        .map(|validated| (validated, value.clone()))
                        .map_err(|error| {
                            CommandError::usage_with_code_and_hints(
                                anyhow!("invalid `--var` name `{name}`: {error}"),
                                DiagnosticCode::ErrConfigParse,
                                vec![RecoveryHint::new(
                                    RecoveryHintKind::ReviewConfiguration {
                                        key: "use variable names containing only ASCII letters, digits, ., _, or -"
                                            .to_owned(),
                                    },
                                )],
                            )
                        })
                })
                .collect::<Result<Vec<_>, CommandError>>()?;
            Ok(InitPass {
                pass_number: pass.pass_number,
                variables,
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;

    let result = sc_composer::template_init(&args.file, &init_passes, args.force, args.dry_run)
        .map_err(CommandError::compose)?;
    if args.json && args.dry_run {
        print_json(
            serde_json::json!({
                "action": "template-init",
                "would_affect": [to_forward_slash(&result.target_path)],
                "changed": result.changed,
                "would_change": result.would_change,
                "skipped": !result.would_change,
                "vars": result.discovered_variables,
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else if args.json {
        print_json(
            serde_json::json!({
                "template_path": to_forward_slash(&result.target_path),
                "template_added": result.changed,
                "would_change": result.would_change,
                "vars": result.discovered_variables,
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else if args.dry_run {
        println!("{}", result.template_text);
    }

    Ok(crate::exit_codes::SUCCESS)
}

fn print_json_frontmatter_init(result: &sc_composer::FrontmatterInitResult) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "template_path": to_forward_slash(&result.target_path),
        "frontmatter_added": result.changed,
        "would_change": result.would_change,
        "vars": result.discovered_variables,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&crate::json_output::envelope(payload, Vec::new()))?
    );
    Ok(())
}

fn template_init_parse_error(error: String) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!(error),
        DiagnosticCode::ErrConfigParse,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "declare each --pass N group before its --var replacements".to_owned(),
        })],
    )
}
