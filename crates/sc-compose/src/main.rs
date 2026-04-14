mod exit_codes;
mod json_output;
mod observer_impl;

use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use sc_composer::{
    ComposeError, ComposeMode, ComposePolicy, ComposeRequest, CompositionObserver, ConfiningRoot,
    Diagnostic, DiagnosticCode, DiagnosticSeverity, FrontmatterInitResult, ProfileKind,
    RuntimeKind, ScalarValue, UnknownVariablePolicy,
};

use crate::observer_impl::{CommandEndEvent, CommandLifecycleObserver, CommandStartEvent};

#[derive(Debug, Parser)]
#[command(name = "sc-compose")]
#[command(about = "Standalone template composition CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Render(RenderArgs),
    Resolve(ResolveArgs),
    Validate(ValidateArgs),
    #[command(name = "frontmatter-init")]
    FrontmatterInit(FrontmatterInitArgs),
    Init(InitArgs),
    #[command(name = "observability-health")]
    // TODO(Sprint 2 / RB-03)
    ObservabilityHealth(ObservabilityHealthArgs),
}

#[derive(Debug, Clone, Args)]
struct CommonArgs {
    #[arg(long, value_enum, default_value = "file")]
    mode: Mode,
    #[arg(long, value_enum, default_value = "agent")]
    kind: Kind,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long, alias = "agent-type")]
    agent_type: Option<String>,
    #[arg(long, value_enum)]
    runtime: Option<Ai>,
    #[arg(long, alias = "ai", value_enum)]
    ai: Option<Ai>,
    #[arg(long = "var", value_parser = parse_var, action = clap::ArgAction::Append)]
    vars: Vec<(String, String)>,
    #[arg(long = "var-file")]
    var_file: Option<String>,
    #[arg(long)]
    env_prefix: Option<String>,
    #[arg(long)]
    strict: bool,
    #[arg(long, value_enum, default_value = "ignore")]
    unknown_var_mode: UnknownVarMode,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    file: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
struct ResolveArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ValidateArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct RenderArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    guidance: Option<String>,
    #[arg(long)]
    guidance_file: Option<String>,
    #[arg(long)]
    prompt: Option<String>,
    #[arg(long)]
    prompt_file: Option<String>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Clone, Args)]
struct FrontmatterInitArgs {
    #[arg(long)]
    file: PathBuf,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Clone, Args)]
struct InitArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Clone, Args)]
struct ObservabilityHealthArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    Profile,
    File,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Kind {
    Agent,
    Command,
    Skill,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Ai {
    Claude,
    Codex,
    Gemini,
    Opencode,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum UnknownVarMode {
    Error,
    Warn,
    Ignore,
}

fn main() {
    let cli = Cli::parse();
    let wants_json = command_wants_json(&cli.command);
    let code = match run(cli) {
        Ok(code) => code,
        Err(error) => {
            if wants_json {
                if let Err(print_error) =
                    print_json(serde_json::json!({}), error.diagnostics.clone())
                {
                    eprintln!("{error}");
                    eprintln!("{print_error:#}");
                }
            } else {
                eprintln!("{error}");
            }
            error.exit_code
        }
    };
    // TODO(Sprint 2 / RB-03): call logger.shutdown() here for graceful flush before exit.
    std::process::exit(code);
}

fn run(cli: Cli) -> Result<i32, CommandError> {
    let mut observer = observer_impl::CliObserver::from_env();
    match cli.command {
        Command::Render(args) => observe_command(&mut observer, "render", |observer| {
            run_render(&args, observer)
        }),
        Command::Resolve(args) => observe_command(&mut observer, "resolve", |observer| {
            run_resolve(&args, observer)
        }),
        Command::Validate(args) => observe_command(&mut observer, "validate", |observer| {
            run_validate(&args, observer)
        }),
        Command::FrontmatterInit(args) => {
            observe_command(&mut observer, "frontmatter-init", |_observer| {
                run_frontmatter_init(&args)
            })
        }
        Command::Init(args) => observe_command(&mut observer, "init", |_observer| run_init(&args)),
        Command::ObservabilityHealth(args) => {
            observe_command(&mut observer, "observability-health", |_observer| {
                run_observability_health(&args)
            })
        }
    }
}

fn run_render(
    args: &RenderArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let request = build_request(&args.common, read_block_pair(args)?)?;
    let result =
        sc_composer::compose_with_observer(&request, observer).map_err(CommandError::compose)?;
    let output_path = args.output.clone();
    let derived_path = derived_output_path(&request, output_path.as_deref());
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
                "would_write": derived_path.display().to_string(),
                "template": result.resolve_result.resolved_path.display().to_string(),
                "rendered_preview": result.rendered_text,
            })
        } else {
            serde_json::json!({
                "output_path": output_path.as_ref().map_or_else(|| "stdout".to_owned(), |path| path.display().to_string()),
                "bytes_written": bytes_written.unwrap_or_default(),
                "template": result.resolve_result.resolved_path.display().to_string(),
            })
        };
        print_json(payload, result.warnings).map_err(CommandError::usage)?;
    } else if args.dry_run {
        println!(
            "template: {}",
            result.resolve_result.resolved_path.display()
        );
        println!("would_write: {}", derived_path.display());
        println!();
        println!("{}", result.rendered_text);
    } else {
        println!("{}", result.rendered_text);
    }

    Ok(exit_codes::SUCCESS)
}

fn run_resolve(
    args: &ResolveArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    if matches!(args.common.mode, Mode::File) {
        return Err(CommandError::usage_with_code(
            anyhow!("resolve is only supported in profile mode"),
            DiagnosticCode::ErrConfigMode,
        ));
    }
    let request = build_request(&args.common, (None, None))?;
    let result = sc_composer::resolve_profile_with_observer(&request, observer)
        .map_err(CommandError::compose)?;
    if args.json {
        let payload = serde_json::json!({
            "resolved_path": result.resolved_path.display().to_string(),
            "search_trace": result.attempted_paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "found": true,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("{}", result.resolved_path.display());
        for path in result.attempted_paths {
            println!("searched: {}", path.display());
        }
    }
    Ok(exit_codes::SUCCESS)
}

fn run_validate(
    args: &ValidateArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let request = build_request(&args.common, (None, None))?;
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
        exit_codes::SUCCESS
    } else {
        exit_codes::VALIDATION_OR_RENDER_FAIL
    })
}

fn run_frontmatter_init(args: &FrontmatterInitArgs) -> Result<i32, CommandError> {
    let result = sc_composer::frontmatter_init(&args.file, args.force, args.dry_run)
        .map_err(CommandError::compose)?;
    if args.json && args.dry_run {
        print_json(
            serde_json::json!({
                "action": "frontmatter-init",
                "would_affect": [result.target_path.display().to_string()],
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
    Ok(exit_codes::SUCCESS)
}

fn run_init(args: &InitArgs) -> Result<i32, CommandError> {
    let canonical_root = std::fs::canonicalize(&args.root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize workspace root {}",
                args.root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let planned_changes = planned_init_changes(&canonical_root);
    let result =
        sc_composer::init_workspace(&args.root, args.dry_run).map_err(CommandError::compose)?;
    if args.json {
        let payload = if args.dry_run {
            serde_json::json!({
                "action": "init",
                "would_affect": planned_changes.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                "changed": false,
                "would_change": !planned_changes.is_empty(),
                "skipped": planned_changes.is_empty(),
            })
        } else {
            serde_json::json!({
                "workspace_root": canonical_root.display().to_string(),
                "created_files": planned_changes.iter().map(|path| {
                    path.strip_prefix(&canonical_root)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                }).collect::<Vec<_>>(),
            })
        };
        print_json(payload, result.recommendations).map_err(CommandError::usage)?;
    } else if args.dry_run {
        for path in &planned_changes {
            println!("would_affect: {}", path.display());
        }
    } else {
        println!("workspace_root: {}", canonical_root.display());
    }
    Ok(if result.validation_passed {
        exit_codes::SUCCESS
    } else {
        exit_codes::VALIDATION_OR_RENDER_FAIL
    })
}

fn run_observability_health(_args: &ObservabilityHealthArgs) -> Result<i32, CommandError> {
    Err(CommandError::usage_with_code(
        anyhow!("observability-health is planned for Sprint 2 / RB-03 and is not implemented yet"),
        DiagnosticCode::ErrConfigMode,
    ))
}

fn build_request(
    args: &CommonArgs,
    blocks: (Option<String>, Option<String>),
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
        runtime: args.runtime.or(args.ai).map(|runtime| match runtime {
            Ai::Claude => RuntimeKind::Claude,
            Ai::Codex => RuntimeKind::Codex,
            Ai::Gemini => RuntimeKind::Gemini,
            Ai::Opencode => RuntimeKind::Opencode,
        }),
        mode,
        root,
        vars_input: load_vars(args)?,
        vars_env: load_env(args)?,
        guidance_block: blocks.0,
        user_prompt: blocks.1,
        policy: ComposePolicy {
            strict_undeclared_variables: args.strict,
            unknown_variable_policy: match args.unknown_var_mode {
                UnknownVarMode::Error => UnknownVariablePolicy::Error,
                UnknownVarMode::Warn => UnknownVariablePolicy::Warn,
                UnknownVarMode::Ignore => UnknownVariablePolicy::Ignore,
            },
            ..ComposePolicy::default()
        },
    })
}

fn read_block_pair(args: &RenderArgs) -> Result<(Option<String>, Option<String>), CommandError> {
    if args.guidance.is_some() && args.guidance_file.is_some() {
        return Err(CommandError::usage(anyhow!(
            "--guidance and --guidance-file are mutually exclusive"
        )));
    }
    if args.prompt.is_some() && args.prompt_file.is_some() {
        return Err(CommandError::usage(anyhow!(
            "--prompt and --prompt-file are mutually exclusive"
        )));
    }
    if args.guidance_file.as_deref() == Some("-") && args.prompt_file.as_deref() == Some("-") {
        return Err(CommandError::stdin_double_read());
    }

    let guidance = read_block(args.guidance.clone(), args.guidance_file.as_deref())?;
    let prompt = read_block(args.prompt.clone(), args.prompt_file.as_deref())?;
    Ok((guidance, prompt))
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

fn load_vars(
    args: &CommonArgs,
) -> Result<BTreeMap<sc_composer::VariableName, ScalarValue>, CommandError> {
    let mut vars = BTreeMap::default();
    for (key, value) in &args.vars {
        vars.insert(
            sc_composer::VariableName::new(key.clone()).map_err(|error| {
                CommandError::usage(anyhow!("invalid `--var` name `{key}`: {error}"))
            })?,
            ScalarValue::String(value.clone()),
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

fn load_env(
    args: &CommonArgs,
) -> Result<BTreeMap<sc_composer::VariableName, ScalarValue>, CommandError> {
    let mut vars = BTreeMap::default();
    if let Some(prefix) = &args.env_prefix {
        for (key, value) in std::env::vars() {
            if let Some(trimmed) = key.strip_prefix(prefix) {
                vars.insert(
                    sc_composer::VariableName::new(trimmed.to_ascii_lowercase()).map_err(
                        |error| {
                            CommandError::usage(anyhow!(
                                "invalid environment-derived variable `{trimmed}`: {error}"
                            ))
                        },
                    )?,
                    ScalarValue::String(value),
                );
            }
        }
    }
    Ok(vars)
}

fn parse_var_file(
    contents: &str,
) -> Result<BTreeMap<sc_composer::VariableName, ScalarValue>, CommandError> {
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
            sc_composer::VariableName::new(key.to_owned()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            ScalarValue::from_yaml(value).map_err(|error| {
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
) -> Result<BTreeMap<sc_composer::VariableName, ScalarValue>, CommandError> {
    let object = value.as_object().ok_or_else(|| {
        CommandError::usage_with_code(
            anyhow!("var-file must be a JSON object"),
            DiagnosticCode::ErrConfigVarfile,
        )
    })?;
    let mut vars = BTreeMap::default();
    for (key, value) in object {
        vars.insert(
            sc_composer::VariableName::new(key.clone()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            ScalarValue::try_from(value.clone()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file value for `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
        );
    }
    Ok(vars)
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

fn print_json_frontmatter_init(result: &FrontmatterInitResult) -> Result<()> {
    let payload = serde_json::json!({
        "template_path": result.target_path.display().to_string(),
        "frontmatter_added": result.changed,
        "would_change": result.would_change,
        "vars": result.discovered_variables,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json_output::envelope(payload, Vec::new()))?
    );
    Ok(())
}

fn parse_var(input: &str) -> Result<(String, String), String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| "expected key=value".to_owned())?;
    Ok((key.to_owned(), value.to_owned()))
}

fn print_json(payload: serde_json::Value, diagnostics: Vec<sc_composer::Diagnostic>) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&json_output::envelope(payload, diagnostics))?
    );
    Ok(())
}

fn command_wants_json(command: &Command) -> bool {
    match command {
        Command::Render(args) => args.json,
        Command::Resolve(args) => args.json,
        Command::Validate(args) => args.json,
        Command::FrontmatterInit(args) => args.json,
        Command::Init(args) => args.json,
        Command::ObservabilityHealth(args) => args.json,
    }
}

fn observe_command<O>(
    observer: &mut O,
    command_name: &str,
    action: impl FnOnce(&mut dyn CompositionObserver) -> Result<i32, CommandError>,
) -> Result<i32, CommandError>
where
    O: CompositionObserver + CommandLifecycleObserver,
{
    observer.on_command_start(&CommandStartEvent {
        command_name: command_name.to_owned(),
    });
    let result = action(observer);
    observer.on_command_end(&CommandEndEvent {
        command_name: command_name.to_owned(),
        success: matches!(result, Ok(exit_codes::SUCCESS)),
    });
    result
}

fn planned_init_changes(root: &Path) -> Vec<PathBuf> {
    let mut changes = Vec::new();
    let prompts_dir = root.join(".prompts");
    if !prompts_dir.exists() {
        changes.push(prompts_dir);
    }

    let gitignore = root.join(".gitignore");
    let current = std::fs::read_to_string(&gitignore).unwrap_or_default();
    if !current.lines().any(|line| line.trim() == ".prompts/") {
        changes.push(gitignore);
    }

    changes
}

fn format_diagnostic(diagnostic: &sc_composer::Diagnostic) -> String {
    let severity = format!("{:?}", diagnostic.severity).to_ascii_lowercase();
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

#[derive(Debug)]
struct CommandError {
    exit_code: i32,
    diagnostic_code: Option<DiagnosticCode>,
    diagnostics: Vec<Diagnostic>,
    error: anyhow::Error,
}

impl CommandError {
    fn usage(error: anyhow::Error) -> Self {
        Self {
            exit_code: exit_codes::USAGE_FAIL,
            diagnostic_code: None,
            diagnostics: Vec::new(),
            error,
        }
    }

    fn usage_with_code(error: anyhow::Error, diagnostic_code: DiagnosticCode) -> Self {
        Self {
            exit_code: exit_codes::USAGE_FAIL,
            diagnostic_code: Some(diagnostic_code),
            diagnostics: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                diagnostic_code,
                format!("{error:#}"),
            )],
            error,
        }
    }

    fn compose(error: ComposeError) -> Self {
        let exit_code = match &error {
            ComposeError::Validation(_) | ComposeError::Render(_) | ComposeError::Include(_) => {
                exit_codes::VALIDATION_OR_RENDER_FAIL
            }
            ComposeError::Resolve(_) | ComposeError::Config(_) => exit_codes::USAGE_FAIL,
        };
        Self {
            exit_code,
            diagnostic_code: error.code(),
            diagnostics: compose_error_diagnostics(&error),
            error: anyhow!(error),
        }
    }

    fn render_write(error: anyhow::Error) -> Self {
        Self {
            exit_code: exit_codes::VALIDATION_OR_RENDER_FAIL,
            diagnostic_code: Some(DiagnosticCode::ErrRenderWrite),
            diagnostics: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrRenderWrite,
                format!("{error:#}"),
            )],
            error,
        }
    }

    fn stdin_double_read() -> Self {
        Self {
            exit_code: exit_codes::VALIDATION_OR_RENDER_FAIL,
            diagnostic_code: Some(DiagnosticCode::ErrRenderStdinDoubleRead),
            diagnostics: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrRenderStdinDoubleRead,
                "guidance and prompt cannot both read from stdin",
            )],
            error: anyhow!("guidance and prompt cannot both read from stdin"),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = self.diagnostic_code {
            return write!(f, "{}: {:#}", code.as_str(), self.error);
        }
        write!(f, "{:#}", self.error)
    }
}

impl std::error::Error for CommandError {}

fn compose_error_diagnostics(error: &ComposeError) -> Vec<Diagnostic> {
    match error {
        ComposeError::Validation(validation) if !validation.diagnostics().is_empty() => {
            validation.diagnostics().to_vec()
        }
        ComposeError::Resolve(resolve) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            resolve.code().unwrap_or(DiagnosticCode::ErrResolveNotFound),
            resolve.message(),
        )],
        ComposeError::Include(include) => vec![
            Diagnostic::new(
                DiagnosticSeverity::Error,
                include.code().unwrap_or(DiagnosticCode::ErrIncludeNotFound),
                include.message(),
            )
            .with_include_chain(include.include_chain().to_vec()),
        ],
        ComposeError::Validation(validation) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            validation.code().unwrap_or(DiagnosticCode::ErrValEmpty),
            validation.message(),
        )],
        ComposeError::Render(render) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            render.code().unwrap_or(DiagnosticCode::ErrRenderWrite),
            render.message(),
        )],
        ComposeError::Config(config) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            config.code().unwrap_or(DiagnosticCode::ErrConfigParse),
            config.message(),
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandError, observe_command};
    use anyhow::anyhow;
    use sc_composer::{CompositionObserver, DiagnosticCode};

    use crate::observer_impl::{CommandEndEvent, CommandLifecycleObserver, CommandStartEvent};

    #[derive(Default)]
    struct CapturingObserver {
        started: Vec<CommandStartEvent>,
        ended: Vec<CommandEndEvent>,
    }

    impl CompositionObserver for CapturingObserver {}

    impl CommandLifecycleObserver for CapturingObserver {
        fn on_command_start(&mut self, event: &CommandStartEvent) {
            self.started.push(event.clone());
        }

        fn on_command_end(&mut self, event: &CommandEndEvent) {
            self.ended.push(event.clone());
        }
    }

    #[test]
    fn observe_command_emits_start_and_end_for_success() {
        let mut observer = CapturingObserver::default();

        let result = observe_command(&mut observer, "render", |_observer| Ok(0));

        assert_eq!(result.unwrap(), 0);
        assert_eq!(observer.started.len(), 1);
        assert_eq!(observer.ended.len(), 1);
        assert!(observer.ended[0].success);
    }

    #[test]
    fn observe_command_emits_start_and_end_for_failure() {
        let mut observer = CapturingObserver::default();

        let result = observe_command(&mut observer, "render", |_observer| {
            Err(CommandError::usage_with_code(
                anyhow!("boom"),
                DiagnosticCode::ErrRenderStdinDoubleRead,
            ))
        });

        let _ = result.unwrap_err();
        assert_eq!(observer.started.len(), 1);
        assert_eq!(observer.ended.len(), 1);
        assert!(!observer.ended[0].success);
    }
}
