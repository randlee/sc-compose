use std::collections::BTreeMap;
use std::io::Read;

use anyhow::{Context, anyhow};
use sc_composer::{
    BUILTIN_VARIABLE_NAMES, ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot,
    DiagnosticCode, InputValue, PassConfig, ProfileKind, RecoveryHint, RecoveryHintKind,
    RuntimeKind, UnknownVariablePolicy, VariableName,
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
    let root = build_root(&args.root)?;
    let mode = build_mode(args)?;

    Ok(ComposeRequest {
        runtime: args.runtime.map(runtime_kind),
        mode,
        root,
        vars_input: load_cli_and_file_vars(&args.input)?,
        vars_env: load_prefixed_env_vars(&args.input)?,
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
    let root = build_root(&args.root)?;
    let mode = build_mode(args)?;

    let vars_input = collect_pass_union(pass_inputs)?;
    let vars_env = load_prefixed_env_vars(&args.input)?;
    let policy = ComposePolicy {
        passes: pass_inputs
            .iter()
            .map(|pass| {
                Ok(PassConfig {
                    pass_number: if pass.pass_number == 0 {
                        sc_composer::types::default_pass_number()
                    } else {
                        pass.pass_number
                    },
                    defaults: load_pass_input_vars(pass)?,
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
        vars_input: load_cli_and_file_vars(input)?,
        vars_env: load_prefixed_env_vars(input)?,
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
    validate_block_sources(input, render, extra_stdin_reads)?;
    let guidance = read_optional_block(render.guidance.clone(), render.guidance_file.as_deref())?;
    let prompt = read_optional_block(render.prompt.clone(), render.prompt_file.as_deref())?;
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

fn build_mode(args: &CommonArgs) -> Result<ComposeMode, CommandError> {
    match args.mode {
        Mode::File => Ok(ComposeMode::File {
            template_path: required_file_path(args)?,
        }),
        Mode::Profile => Ok(ComposeMode::Profile {
            kind: profile_kind(args.kind),
            name: required_profile_name(args)?,
        }),
    }
}

fn build_root(root: &std::path::Path) -> Result<ConfiningRoot, CommandError> {
    ConfiningRoot::new(root)
        .with_context(|| format!("failed to canonicalize root {}", root.display()))
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))
}

fn required_file_path(args: &CommonArgs) -> Result<std::path::PathBuf, CommandError> {
    args.file.clone().ok_or_else(|| {
        CommandError::usage_with_code_and_hints(
            anyhow!("--file is required in file mode"),
            DiagnosticCode::ErrConfigMode,
            vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "pass --file when --mode file is selected".to_owned(),
            })],
        )
    })
}

fn required_profile_name(args: &CommonArgs) -> Result<sc_composer::ProfileName, CommandError> {
    let name = args
        .agent
        .clone()
        .or_else(|| args.agent_type.clone())
        .ok_or_else(|| {
            CommandError::usage_with_code_and_hints(
                anyhow!("--agent/--agent-type is required in profile mode"),
                DiagnosticCode::ErrConfigMode,
                vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                    key: "pass --agent or --agent-type when --mode profile is selected".to_owned(),
                })],
            )
        })?;
    sc_composer::ProfileName::new(name).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!("invalid profile name: {error}"),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "use an alphanumeric profile name with . _ or - only".to_owned(),
            })],
        )
    })
}

const fn profile_kind(kind: Kind) -> ProfileKind {
    match kind {
        Kind::Agent => ProfileKind::Agent,
        Kind::Command => ProfileKind::Command,
        Kind::Skill => ProfileKind::Skill,
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

fn validate_block_sources(
    input: &InputArgs,
    render: &RenderBehaviorArgs,
    extra_stdin_reads: usize,
) -> Result<(), CommandError> {
    if render.guidance.is_some() && render.guidance_file.is_some() {
        return Err(CommandError::usage_with_code(
            anyhow!("--guidance and --guidance-file are mutually exclusive"),
            DiagnosticCode::ErrConfigParse,
        ));
    }
    if render.prompt.is_some() && render.prompt_file.is_some() {
        return Err(CommandError::usage_with_code(
            anyhow!("--prompt and --prompt-file are mutually exclusive"),
            DiagnosticCode::ErrConfigParse,
        ));
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
    Ok(())
}

fn read_optional_block(
    inline: Option<String>,
    file: Option<&str>,
) -> Result<Option<String>, CommandError> {
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

fn load_cli_and_file_vars(
    args: &InputArgs,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::default();
    load_cli_vars(&mut vars, &args.vars)?;
    load_var_file_vars(&mut vars, &args.var_file)?;
    Ok(vars)
}

fn load_prefixed_env_vars(
    args: &InputArgs,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
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
                        CommandError::usage_with_code(
                            anyhow!("invalid environment-derived variable `{trimmed}`: {error}"),
                            DiagnosticCode::ErrConfigParse,
                        )
                    })?,
                    serde_json::Value::String(value),
                );
            }
        }
    }
    Ok(vars)
}

fn load_pass_input_vars(
    pass: &PassInputArgs,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::new();
    load_cli_vars(&mut vars, &pass.vars)?;
    load_var_file_vars(&mut vars, &pass.var_files)?;
    Ok(vars)
}

fn collect_pass_union(
    pass_inputs: &[PassInputArgs],
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::new();
    for pass in pass_inputs {
        vars.extend(load_pass_input_vars(pass)?);
    }
    Ok(vars)
}

fn load_var_source(path: &str) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    if path == "-" {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| {
                CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
            })?;
        parse_var_file_contents(&input)
    } else {
        load_var_file(std::path::Path::new(path))
    }
}

fn invalid_var_name_error(
    key: &str,
    error: &sc_composer::InvalidVariableNameError,
) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!("invalid `--var` name `{key}`: {error}"),
        DiagnosticCode::ErrConfigParse,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "use variable names containing only ASCII letters, digits, ., _, or -".to_owned(),
        })],
    )
}

fn load_cli_vars(
    vars: &mut BTreeMap<VariableName, InputValue>,
    entries: &[(String, String)],
) -> Result<(), CommandError> {
    for (key, value) in entries {
        vars.insert(
            VariableName::new(key.clone()).map_err(|error| invalid_var_name_error(key, &error))?,
            serde_json::Value::String(value.clone()),
        );
    }
    Ok(())
}

fn load_var_file_vars(
    vars: &mut BTreeMap<VariableName, InputValue>,
    paths: &[String],
) -> Result<(), CommandError> {
    for path in paths {
        vars.extend(load_var_source(path)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_mode, build_multi_pass_request, build_named_request, build_request, build_root,
        collect_pass_union, load_cli_and_file_vars, load_prefixed_env_vars,
        read_block_pair_with_extra_stdin_reads,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use sc_composer::{
        BUILTIN_VARIABLE_NAMES, ComposeMode, InputValue, PassConfig, ProfileKind, RuntimeKind,
        UnknownVariablePolicy, VariableName,
    };

    use crate::cli::{
        Ai, CommonArgs, InputArgs, Kind, Mode, PassInputArgs, RenderBehaviorArgs, UnknownVarMode,
    };
    use crate::template_store::TemplatePack;

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sc-compose-render-request-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn input_args() -> InputArgs {
        InputArgs {
            vars: Vec::new(),
            var_file: Vec::new(),
            env_prefix: None,
            strict: false,
            unknown_var_mode: UnknownVarMode::Ignore,
        }
    }

    fn common_args(root: &Path) -> CommonArgs {
        CommonArgs {
            mode: Mode::File,
            kind: Kind::Agent,
            agent: None,
            agent_type: None,
            runtime: None,
            input: input_args(),
            root: root.to_path_buf(),
            file: Some(PathBuf::from("template.md.j2")),
        }
    }

    #[test]
    fn compose_mode_maps_file_and_profile_requests() {
        let root = temp_root("compose-mode");

        let file_args = common_args(&root);
        assert_eq!(
            build_mode(&file_args).unwrap(),
            ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            }
        );

        let mut profile_args = common_args(&root);
        profile_args.mode = Mode::Profile;
        profile_args.kind = Kind::Skill;
        profile_args.agent = Some("skill.rx".to_owned());
        profile_args.file = None;

        assert_eq!(
            build_mode(&profile_args).unwrap(),
            ComposeMode::Profile {
                kind: ProfileKind::Skill,
                name: sc_composer::ProfileName::new("skill.rx").unwrap(),
            }
        );
    }

    #[test]
    fn confining_root_canonicalizes_root_path() {
        let root = temp_root("confining-root");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).unwrap();

        let confining = build_root(&nested).unwrap();

        assert_eq!(confining.as_path(), nested.canonicalize().unwrap());
    }

    #[test]
    fn read_block_pair_rejects_conflicting_sources_and_double_stdin() {
        let input = InputArgs {
            var_file: vec!["-".to_owned()],
            ..input_args()
        };
        let render = RenderBehaviorArgs {
            guidance_file: Some("-".to_owned()),
            ..RenderBehaviorArgs::default()
        };
        let error = read_block_pair_with_extra_stdin_reads(&input, &render, 0).unwrap_err();
        assert_eq!(
            error.diagnostic_code,
            Some(sc_composer::DiagnosticCode::ErrRenderStdinDoubleRead)
        );

        let render = RenderBehaviorArgs {
            guidance: Some("inline".to_owned()),
            guidance_file: Some("guidance.txt".to_owned()),
            ..RenderBehaviorArgs::default()
        };
        let error = read_block_pair_with_extra_stdin_reads(&input_args(), &render, 0).unwrap_err();
        assert_eq!(
            error.diagnostic_code,
            Some(sc_composer::DiagnosticCode::ErrConfigParse)
        );
        assert!(error.error.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn load_vars_merges_inline_and_var_file_inputs() {
        let root = temp_root("load-vars");
        let vars_path = root.join("vars.yaml");
        write_file(&vars_path, "name: yaml-world\ncount: 2\n");

        let args = InputArgs {
            vars: vec![("name".to_owned(), "inline-world".to_owned())],
            var_file: vec![vars_path.display().to_string()],
            ..input_args()
        };

        let vars = load_cli_and_file_vars(&args).unwrap();

        assert_eq!(
            vars.get(&VariableName::new("name".to_owned()).unwrap()),
            Some(&InputValue::String("yaml-world".to_owned()))
        );
        assert_eq!(
            vars.get(&VariableName::new("count".to_owned()).unwrap()),
            Some(&serde_json::json!(2))
        );
    }

    #[test]
    fn load_env_uses_prefix_and_preserves_builtin_casing() {
        let (source_key, source_value) = std::env::vars()
            .find(|(key, _)| key == "PATH")
            .or_else(|| std::env::vars().next())
            .expect("process environment must contain at least one variable");
        let prefix = source_key
            .chars()
            .next()
            .map(|ch| ch.to_string())
            .unwrap_or_default();
        let trimmed = source_key.strip_prefix(&prefix).unwrap_or(&source_key);
        let expected_name = if BUILTIN_VARIABLE_NAMES.contains(&trimmed) {
            trimmed.to_owned()
        } else {
            trimmed.to_ascii_lowercase()
        };

        let args = InputArgs {
            env_prefix: Some(prefix),
            ..input_args()
        };

        let vars = load_prefixed_env_vars(&args).unwrap();

        assert_eq!(
            vars.get(&VariableName::new(expected_name).unwrap()),
            Some(&InputValue::String(source_value))
        );
    }

    #[test]
    fn collect_pass_union_reads_inline_and_var_file_values() {
        let root = temp_root("collect-pass-union");
        let vars_path = root.join("pass.json");
        write_file(&vars_path, "{ \"branch\": \"develop\" }\n");

        let pass_inputs = vec![
            PassInputArgs {
                pass_number: 0,
                vars: vec![("name".to_owned(), "world".to_owned())],
                var_files: vec![vars_path.display().to_string()],
            },
            PassInputArgs {
                pass_number: 2,
                vars: vec![("owner".to_owned(), "team".to_owned())],
                var_files: Vec::new(),
            },
        ];

        let vars = collect_pass_union(&pass_inputs).unwrap();

        assert_eq!(
            vars.get(&VariableName::new("name".to_owned()).unwrap()),
            Some(&InputValue::String("world".to_owned()))
        );
        assert_eq!(
            vars.get(&VariableName::new("branch".to_owned()).unwrap()),
            Some(&InputValue::String("develop".to_owned()))
        );
        assert_eq!(
            vars.get(&VariableName::new("owner".to_owned()).unwrap()),
            Some(&InputValue::String("team".to_owned()))
        );
    }

    #[test]
    fn build_request_maps_runtime_mode_and_policy() {
        let root = temp_root("build-request");
        let vars_path = root.join("vars.json");
        write_file(&vars_path, "{ \"name\": \"json-world\" }\n");

        let mut args = common_args(&root);
        args.runtime = Some(Ai::Gemini);
        args.input.strict = true;
        args.input.unknown_var_mode = UnknownVarMode::Warn;
        args.input.var_file = vec![vars_path.display().to_string()];

        let request = build_request(
            &args,
            (Some("guidance".to_owned()), Some("prompt".to_owned())),
            BTreeMap::new(),
        )
        .unwrap();

        assert_eq!(request.runtime, Some(RuntimeKind::Gemini));
        assert_eq!(
            request.mode,
            ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            }
        );
        assert_eq!(request.guidance_block.as_deref(), Some("guidance"));
        assert_eq!(request.user_prompt.as_deref(), Some("prompt"));
        assert!(request.policy.strict_undeclared_variables);
        assert_eq!(
            request.policy.unknown_variable_policy,
            UnknownVariablePolicy::Warn
        );
        assert_eq!(
            request
                .vars_input
                .get(&VariableName::new("name".to_owned()).unwrap()),
            Some(&InputValue::String("json-world".to_owned()))
        );
    }

    #[test]
    fn build_multi_pass_request_normalizes_pass_zero_and_builds_union() {
        let root = temp_root("build-multi-pass-request");
        let vars_path = root.join("pass.yaml");
        write_file(&vars_path, "owner: qa\n");

        let pass_inputs = vec![
            PassInputArgs {
                pass_number: 0,
                vars: vec![("name".to_owned(), "world".to_owned())],
                var_files: vec![vars_path.display().to_string()],
            },
            PassInputArgs {
                pass_number: 3,
                vars: vec![("branch".to_owned(), "develop".to_owned())],
                var_files: Vec::new(),
            },
        ];

        let request = build_multi_pass_request(
            &common_args(&root),
            (None, None),
            BTreeMap::new(),
            &pass_inputs,
        )
        .unwrap();

        assert_eq!(
            request
                .vars_input
                .get(&VariableName::new("owner".to_owned()).unwrap()),
            Some(&InputValue::String("qa".to_owned()))
        );
        assert_eq!(request.policy.passes.len(), 2);
        assert_eq!(
            request.policy.passes[0],
            PassConfig {
                pass_number: sc_composer::types::default_pass_number(),
                defaults: BTreeMap::from([
                    (
                        VariableName::new("name".to_owned()).unwrap(),
                        InputValue::String("world".to_owned()),
                    ),
                    (
                        VariableName::new("owner".to_owned()).unwrap(),
                        InputValue::String("qa".to_owned()),
                    ),
                ]),
                ..PassConfig::default()
            }
        );
        assert_eq!(request.policy.passes[1].pass_number, 3);
    }

    #[test]
    fn build_named_request_uses_pack_root_template_and_defaults() {
        let root = temp_root("build-named-request");
        let pack = TemplatePack {
            root: root.clone(),
            template_path: root.join("template.md.j2"),
            input_defaults: BTreeMap::from([(
                VariableName::new("name".to_owned()).unwrap(),
                InputValue::String("pack-default".to_owned()),
            )]),
        };

        let request = build_named_request(
            &pack,
            &InputArgs {
                unknown_var_mode: UnknownVarMode::Error,
                ..input_args()
            },
            (Some("guidance".to_owned()), None),
        )
        .unwrap();

        assert_eq!(request.runtime, None);
        assert_eq!(
            request.mode,
            ComposeMode::File {
                template_path: pack.template_path.clone(),
            }
        );
        assert_eq!(request.root.as_path(), root.canonicalize().unwrap());
        assert_eq!(
            request
                .vars_defaults
                .get(&VariableName::new("name".to_owned()).unwrap()),
            Some(&InputValue::String("pack-default".to_owned()))
        );
        assert_eq!(
            request.policy.unknown_variable_policy,
            UnknownVariablePolicy::Error
        );
    }
}
