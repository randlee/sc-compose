use std::collections::BTreeMap;
use std::io::Read;

use anyhow::{Context, anyhow};
use sc_composer::{
    BUILTIN_VARIABLE_NAMES, ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot,
    DiagnosticCode, InputValue, PassConfig, ProfileKind, RuntimeKind, UnknownVariablePolicy,
    VariableName,
};

use crate::CommandError;
use crate::cli::{
    Ai, CommonArgs, InputArgs, Kind, Mode, PassInputArgs, RenderBehaviorArgs, UnknownVarMode,
};
use crate::template_store::TemplatePack;
use crate::var_file::{load_var_file, parse_var_file_contents};

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

pub(crate) fn build_multi_pass_request(
    args: &CommonArgs,
    blocks: (Option<String>, Option<String>),
    vars_defaults: BTreeMap<VariableName, InputValue>,
    pass_inputs: &[PassInputArgs],
) -> Result<ComposeRequest, CommandError> {
    let root = ConfiningRoot::new(&args.root)
        .with_context(|| format!("failed to canonicalize root {}", args.root.display()))
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))?;
    let mode = compose_mode(args)?;

    let vars_input = collect_pass_union(pass_inputs)?;
    let vars_env = load_env(&args.input)?;
    let policy = ComposePolicy {
        passes: pass_inputs
            .iter()
            .map(|pass| {
                Ok(PassConfig {
                    pass_number: pass.pass_number,
                    defaults: load_pass_vars(pass)?,
                    ..PassConfig::default()
                })
            })
            .collect::<Result<Vec<_>, CommandError>>()?,
        ..compose_policy(&args.input)
    };

    Ok(ComposeRequest {
        runtime: args.runtime.map(runtime_kind),
        mode,
        root,
        vars_input,
        vars_env,
        vars_defaults,
        guidance_block: blocks.0,
        user_prompt: blocks.1,
        policy,
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
    read_block_pair_with_extra_stdin_reads(input, render, 0)
}

pub(crate) fn read_block_pair_with_extra_stdin_reads(
    input: &InputArgs,
    render: &RenderBehaviorArgs,
    extra_stdin_reads: usize,
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
    let stdin_reads = input
        .var_file
        .iter()
        .filter(|path| path.as_str() == "-")
        .count()
        + usize::from(render.guidance_file.as_deref() == Some("-"))
        + usize::from(render.prompt_file.as_deref() == Some("-"));
    if stdin_reads + extra_stdin_reads > 1 {
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

fn compose_mode(args: &CommonArgs) -> Result<ComposeMode, CommandError> {
    match args.mode {
        Mode::File => Ok(ComposeMode::File {
            template_path: args
                .file
                .clone()
                .ok_or_else(|| CommandError::usage(anyhow!("--file is required in file mode")))?,
        }),
        Mode::Profile => Ok(ComposeMode::Profile {
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
        }),
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
    for path in &args.var_file {
        let object = if path == "-" {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| {
                    CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
                })?;
            parse_var_file_contents(&input)?
        } else {
            load_var_file(std::path::Path::new(path))?
        };
        vars.extend(object);
    }
    Ok(vars)
}

fn load_env(args: &InputArgs) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::default();
    if let Some(prefix) = &args.env_prefix {
        for (key, value) in std::env::vars() {
            if let Some(trimmed) = key.strip_prefix(prefix) {
                let name = if BUILTIN_VARIABLE_NAMES.contains(&trimmed) {
                    trimmed.to_owned()
                } else {
                    trimmed.to_ascii_lowercase()
                };
                vars.insert(
                    VariableName::new(name).map_err(|error| {
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

fn load_pass_vars(
    pass: &PassInputArgs,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::new();
    for (key, value) in &pass.vars {
        vars.insert(
            VariableName::new(key.clone()).map_err(|error| {
                CommandError::usage(anyhow!("invalid `--var` name `{key}`: {error}"))
            })?,
            serde_json::Value::String(value.clone()),
        );
    }
    for path in &pass.var_files {
        if path == "-" {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| {
                    CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
                })?;
            vars.extend(parse_var_file_contents(&input)?);
        } else {
            vars.extend(load_var_file(std::path::Path::new(path))?);
        }
    }
    Ok(vars)
}

fn collect_pass_union(
    pass_inputs: &[PassInputArgs],
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::new();
    for pass in pass_inputs {
        vars.extend(load_pass_vars(pass)?);
    }
    Ok(vars)
}
