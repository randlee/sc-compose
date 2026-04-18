use std::collections::BTreeMap;
use std::io::Read;

use anyhow::{Context, anyhow};
use sc_composer::{
    ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, InputValue, ProfileKind,
    RuntimeKind, UnknownVariablePolicy, VariableName, input_value_from_yaml, validate_input_value,
};

use crate::template_store::TemplatePack;
use crate::{
    Ai, CommandError, CommonArgs, DiagnosticCode, InputArgs, Kind, Mode, RenderBehaviorArgs,
    UnknownVarMode,
};

pub(crate) fn build_request(
    args: &CommonArgs,
    blocks: (Option<String>, Option<String>),
    vars_defaults: BTreeMap<VariableName, InputValue>,
) -> Result<ComposeRequest, CommandError> {
    let root = ConfiningRoot::new(&args.root)
        .with_context(|| format!("failed to canonicalize root {}", args.root.display()))
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))?;
    let mode = match args.mode {
        Mode::File => ComposeMode::File {
            template_path: args
                .file
                .clone()
                .ok_or_else(|| CommandError::usage(anyhow!("--file is required in file mode")))?,
        },
        Mode::Profile => ComposeMode::Profile {
            kind: match args.kind {
                Kind::Agent => ProfileKind::Agent,
                Kind::Command => ProfileKind::Command,
                Kind::Skill => ProfileKind::Skill,
            },
            name: args
                .agent
                .clone()
                .or_else(|| args.agent_type.clone())
                .ok_or_else(|| {
                    CommandError::usage(anyhow!("--agent/--agent-type is required in profile mode"))
                })
                .and_then(|name| {
                    sc_composer::ProfileName::new(name).map_err(|error| {
                        CommandError::usage(anyhow!("invalid profile name: {error}"))
                    })
                })?,
        },
    };

    Ok(ComposeRequest {
        runtime: args.runtime.map(runtime_kind),
        mode,
        root,
        vars_input: load_vars(&args.input)?,
        vars_env: load_env(&args.input)?,
        vars_defaults,
        guidance_block: blocks.0,
        user_prompt: blocks.1,
        policy: compose_policy(&args.input),
    })
}

pub(crate) fn build_named_request(
    pack: &TemplatePack,
    input: &InputArgs,
    blocks: (Option<String>, Option<String>),
) -> Result<ComposeRequest, CommandError> {
    let root = ConfiningRoot::new(&pack.root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize root {}",
                pack.root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;

    Ok(ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: pack.template_path.clone(),
        },
        root,
        vars_input: load_vars(input)?,
        vars_env: load_env(input)?,
        vars_defaults: pack.input_defaults.clone(),
        guidance_block: blocks.0,
        user_prompt: blocks.1,
        policy: compose_policy(input),
    })
}

pub(crate) fn read_block_pair(
    input: &InputArgs,
    render: &RenderBehaviorArgs,
) -> Result<(Option<String>, Option<String>), CommandError> {
    if render.guidance.is_some() && render.guidance_file.is_some() {
        return Err(CommandError::usage(anyhow!(
            "--guidance and --guidance-file are mutually exclusive"
        )));
    }
    if render.prompt.is_some() && render.prompt_file.is_some() {
        return Err(CommandError::usage(anyhow!(
            "--prompt and --prompt-file are mutually exclusive"
        )));
    }
    let stdin_reads = usize::from(input.var_file.as_deref() == Some("-"))
        + usize::from(render.guidance_file.as_deref() == Some("-"))
        + usize::from(render.prompt_file.as_deref() == Some("-"));
    if stdin_reads > 1 {
        return Err(CommandError::stdin_double_read());
    }

    let guidance = read_block(render.guidance.clone(), render.guidance_file.as_deref())?;
    let prompt = read_block(render.prompt.clone(), render.prompt_file.as_deref())?;
    Ok((guidance, prompt))
}

fn compose_policy(input: &InputArgs) -> ComposePolicy {
    ComposePolicy {
        strict_undeclared_variables: input.strict,
        unknown_variable_policy: match input.unknown_var_mode {
            UnknownVarMode::Error => UnknownVariablePolicy::Error,
            UnknownVarMode::Warn => UnknownVariablePolicy::Warn,
            UnknownVarMode::Ignore => UnknownVariablePolicy::Ignore,
        },
        ..ComposePolicy::default()
    }
}

fn runtime_kind(runtime: Ai) -> RuntimeKind {
    match runtime {
        Ai::Claude => RuntimeKind::Claude,
        Ai::Codex => RuntimeKind::Codex,
        Ai::Gemini => RuntimeKind::Gemini,
        Ai::Opencode => RuntimeKind::Opencode,
    }
}

fn read_block(inline: Option<String>, file: Option<&str>) -> Result<Option<String>, CommandError> {
    if let Some(inline) = inline {
        return Ok(Some(inline));
    }
    match file {
        Some("-") => {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| {
                    CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
                })?;
            Ok(Some(input))
        }
        Some(path) => std::fs::read_to_string(path).map(Some).map_err(|error| {
            CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
        }),
        None => Ok(None),
    }
}

fn load_vars(args: &InputArgs) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::default();
    for (key, value) in &args.vars {
        vars.insert(
            VariableName::new(key.clone()).map_err(|error| {
                CommandError::usage(anyhow!("invalid `--var` name `{key}`: {error}"))
            })?,
            serde_json::Value::String(value.clone()),
        );
    }
    if let Some(path) = &args.var_file {
        let contents = if path == "-" {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| {
                    CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
                })?;
            input
        } else {
            std::fs::read_to_string(path).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!(error).context(format!("failed to read var-file {path}")),
                    DiagnosticCode::ErrConfigParse,
                )
            })?
        };
        let object = parse_var_file(&contents)?;
        vars.extend(object);
    }
    Ok(vars)
}

fn load_env(args: &InputArgs) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::default();
    if let Some(prefix) = &args.env_prefix {
        for (key, value) in std::env::vars() {
            if let Some(trimmed) = key.strip_prefix(prefix) {
                vars.insert(
                    VariableName::new(trimmed.to_ascii_lowercase()).map_err(|error| {
                        CommandError::usage(anyhow!(
                            "invalid environment-derived variable `{trimmed}`: {error}"
                        ))
                    })?,
                    serde_json::Value::String(value),
                );
            }
        }
    }
    Ok(vars)
}

fn parse_var_file(contents: &str) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) {
        return parse_object_value(&value);
    }
    let value = serde_yaml::from_str::<serde_yaml::Value>(contents).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("var-file must be valid JSON or YAML"),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let serde_yaml::Value::Mapping(object) = value else {
        return Err(CommandError::usage_with_code(
            anyhow!("var-file must be a JSON or YAML object"),
            DiagnosticCode::ErrConfigVarfile,
        ));
    };
    let mut vars = BTreeMap::default();
    for (key, value) in object {
        let key = key.as_str().ok_or_else(|| {
            CommandError::usage_with_code(
                anyhow!("var-file keys must be strings"),
                DiagnosticCode::ErrConfigVarfile,
            )
        })?;
        vars.insert(
            VariableName::new(key.to_owned()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            input_value_from_yaml(value).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file value for `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
        );
    }
    Ok(vars)
}

fn parse_object_value(
    value: &serde_json::Value,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let object = value.as_object().ok_or_else(|| {
        CommandError::usage_with_code(
            anyhow!("var-file must be a JSON object"),
            DiagnosticCode::ErrConfigVarfile,
        )
    })?;
    let mut vars = BTreeMap::default();
    for (key, value) in object {
        vars.insert(
            VariableName::new(key.clone()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            {
                validate_input_value(value).map_err(|error| {
                    CommandError::usage_with_code(
                        anyhow!("invalid var-file value for `{key}`: {error}"),
                        DiagnosticCode::ErrConfigVarfile,
                    )
                })?;
                value.clone()
            },
        );
    }
    Ok(vars)
}
