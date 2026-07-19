use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::commands::reports::{
    ReportCatalogArgs, ReportRenderManyArgs, ReportsArgs, ReportsSubcommand,
};

#[derive(Debug, Parser)]
#[command(name = "sc-compose")]
#[command(version)]
#[command(about = "Standalone template composition CLI")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(about = "Render a template or resolved profile")]
    Render(RenderArgs),
    #[command(about = "Resolve a profile name to a concrete template path")]
    Resolve(ResolveArgs),
    #[command(about = "Validate templates without rendering output")]
    Validate(ValidateArgs),
    #[command(name = "frontmatter-init")]
    #[command(about = "Insert minimal frontmatter for referenced variables")]
    FrontmatterInit(FrontmatterInitArgs),
    #[command(about = "Bootstrap a workspace for composed outputs")]
    Init(InitArgs),
    #[command(name = "observability-health")]
    #[command(about = "Report process-local logging health")]
    ObservabilityHealth(ObservabilityHealthArgs),
    #[command(about = "List or render bundled example templates")]
    Examples(ExamplesArgs),
    #[command(about = "List, add, or render user template packs")]
    Templates(TemplatesArgs),
    #[command(about = "Initialize and run shared report scaffolds")]
    Reports(ReportsArgs),
    #[command(hide = true, name = "report-render-many")]
    ReportRenderMany(ReportRenderManyArgs),
    #[command(hide = true, name = "report-catalog")]
    ReportCatalog(ReportCatalogArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct InputArgs {
    #[arg(
        long = "var",
        value_parser = parse_var,
        action = clap::ArgAction::Append,
        help = "Provide one explicit input variable as key=value"
    )]
    pub(crate) vars: Vec<(String, String)>,
    #[arg(
        long = "var-file",
        action = clap::ArgAction::Append,
        help = "Load input variables from a JSON or YAML object file"
    )]
    pub(crate) var_file: Vec<String>,
    #[arg(
        long,
        help = "Absorb environment variables that match the given prefix"
    )]
    pub(crate) env_prefix: Option<String>,
    #[arg(long, help = "Treat undeclared referenced variables as errors")]
    pub(crate) strict: bool,
    #[arg(
        long,
        value_enum,
        default_value = "ignore",
        help = "Control how extra caller-provided variables are reported"
    )]
    pub(crate) unknown_var_mode: UnknownVarMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassInputArgs {
    pub(crate) pass_number: u8,
    pub(crate) vars: Vec<(String, String)>,
    pub(crate) var_files: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CommonArgs {
    #[arg(
        long,
        value_enum,
        default_value = "file",
        help = "Choose file or profile resolution mode"
    )]
    pub(crate) mode: Mode,
    #[arg(
        long,
        value_enum,
        default_value = "agent",
        help = "Choose the profile kind in profile mode"
    )]
    pub(crate) kind: Kind,
    #[arg(long, help = "Profile name in profile mode")]
    pub(crate) agent: Option<String>,
    #[arg(long, alias = "agent-type", help = "Alias for --agent")]
    pub(crate) agent_type: Option<String>,
    #[arg(
        long,
        alias = "ai",
        value_enum,
        help = "Optional runtime selector in profile mode"
    )]
    pub(crate) runtime: Option<Ai>,
    #[command(flatten)]
    pub(crate) input: InputArgs,
    #[arg(
        long,
        default_value = ".",
        help = "Workspace root for resolution and confinement"
    )]
    pub(crate) root: PathBuf,
    #[arg(long, help = "Template path in file mode")]
    pub(crate) file: Option<PathBuf>,
}

#[derive(Debug, Clone, Args, Default)]
pub(crate) struct RenderBehaviorArgs {
    #[arg(
        long,
        help = "Write rendered output to the given path instead of stdout"
    )]
    pub(crate) output: Option<PathBuf>,
    #[arg(long, help = "Append a guidance block after the rendered body")]
    pub(crate) guidance: Option<String>,
    #[arg(long, help = "Read the guidance block from a file or stdin")]
    pub(crate) guidance_file: Option<String>,
    #[arg(
        long,
        help = "Append a user prompt block after the rendered body and guidance"
    )]
    pub(crate) prompt: Option<String>,
    #[arg(long, help = "Read the user prompt block from a file or stdin")]
    pub(crate) prompt_file: Option<String>,
    #[arg(long, help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
    #[arg(long, help = "Report the derived output target without writing files")]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ResolveArgs {
    #[command(flatten)]
    pub(crate) common: CommonArgs,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ValidateArgs {
    #[command(flatten)]
    pub(crate) common: CommonArgs,
    #[arg(long, help = "Validate all stacked template passes")]
    pub(crate) all: bool,
    #[arg(
        long = "pass",
        action = clap::ArgAction::Append,
        value_name = "N",
        help = "Declare the next per-pass variable group"
    )]
    pub(crate) pass_groups: Vec<u8>,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct RenderArgs {
    #[command(flatten)]
    pub(crate) common: CommonArgs,
    #[arg(long, help = "Render all stacked template passes")]
    pub(crate) all: bool,
    #[arg(
        long = "pass",
        action = clap::ArgAction::Append,
        value_name = "N",
        help = "Declare the next per-pass variable group"
    )]
    pub(crate) pass_groups: Vec<u8>,
    #[arg(
        long = "brace-count",
        value_parser = clap::value_parser!(u8).range(2..),
        conflicts_with_all = ["all", "variable_delimiters"],
        help = "Render with custom brace-count delimiters (for example 3 => {{{ }}})"
    )]
    pub(crate) brace_count: Option<u8>,
    #[arg(
        long = "variable-delimiters",
        num_args = 2,
        value_names = ["OPEN", "CLOSE"],
        conflicts_with_all = ["all", "brace_count"],
        help = "Render with explicit variable delimiters"
    )]
    pub(crate) variable_delimiters: Option<Vec<String>>,
    #[command(flatten)]
    pub(crate) render: RenderBehaviorArgs,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct FrontmatterInitArgs {
    #[arg(long)]
    pub(crate) file: PathBuf,
    #[arg(long)]
    pub(crate) force: bool,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct InitArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ObservabilityHealthArgs {
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct ExamplesArgs {
    #[command(subcommand)]
    pub(crate) command: Option<ExamplesSubcommand>,
    #[arg(index = 1, help = "Bundled example pack name to render")]
    pub(crate) name: Option<String>,
    #[command(flatten)]
    pub(crate) input: InputArgs,
    #[command(flatten)]
    pub(crate) render: RenderBehaviorArgs,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum ExamplesSubcommand {
    #[command(about = "List bundled example packs")]
    List(ListArgs),
}

#[derive(Debug, Clone, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct TemplatesArgs {
    #[command(subcommand)]
    pub(crate) command: Option<TemplatesSubcommand>,
    #[arg(index = 1, help = "User template pack name to render")]
    pub(crate) name: Option<String>,
    #[command(flatten)]
    pub(crate) input: InputArgs,
    #[command(flatten)]
    pub(crate) render: RenderBehaviorArgs,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum TemplatesSubcommand {
    #[command(about = "List user template packs")]
    List(ListArgs),
    #[command(about = "Import a file or directory as one user template pack")]
    Add(TemplatesAddArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ListArgs {
    #[arg(long, help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct TemplatesAddArgs {
    /// Source file or directory to import as one template pack.
    pub(crate) src: PathBuf,
    /// Optional pack name override.
    pub(crate) name: Option<String>,
    #[arg(long, help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Mode {
    Profile,
    File,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Kind {
    Agent,
    Command,
    Skill,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Ai {
    Claude,
    Codex,
    Gemini,
    Opencode,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum UnknownVarMode {
    Error,
    Warn,
    Ignore,
}

pub(crate) fn parse_var(input: &str) -> Result<(String, String), String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| "expected key=value".to_owned())?;
    Ok((key.to_owned(), value.to_owned()))
}

pub(crate) fn parse_pass_inputs(command_name: &str) -> Result<Vec<PassInputArgs>, String> {
    let mut args = std::env::args_os();
    let mut found_command = false;
    let mut current: Option<PassInputArgs> = None;
    let mut parsed = Vec::new();

    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if !found_command {
            if arg == command_name {
                found_command = true;
            }
            continue;
        }

        match arg.as_ref() {
            "--pass" => {
                if let Some(group) = current.take() {
                    parsed.push(group);
                }
                let Some(value) = args.next() else {
                    return Err("--pass requires a numeric pass number".to_owned());
                };
                let value = value.to_string_lossy();
                let pass_number = value
                    .parse::<u8>()
                    .map_err(|error| format!("invalid pass number `{value}`: {error}"))?;
                current = Some(PassInputArgs {
                    pass_number,
                    vars: Vec::new(),
                    var_files: Vec::new(),
                });
            }
            _ if arg.starts_with("--pass=") => {
                if let Some(group) = current.take() {
                    parsed.push(group);
                }
                let value = arg.trim_start_matches("--pass=");
                let pass_number = value
                    .parse::<u8>()
                    .map_err(|error| format!("invalid pass number `{value}`: {error}"))?;
                current = Some(PassInputArgs {
                    pass_number,
                    vars: Vec::new(),
                    var_files: Vec::new(),
                });
            }
            "--var" => {
                let Some(value) = args.next() else {
                    return Err("--var requires key=value".to_owned());
                };
                let value = value.to_string_lossy();
                let current = current.as_mut().ok_or_else(|| {
                    "--var must appear after --pass when --all is enabled".to_owned()
                })?;
                current.vars.push(parse_var(&value)?);
            }
            _ if arg.starts_with("--var=") => {
                let current = current.as_mut().ok_or_else(|| {
                    "--var must appear after --pass when --all is enabled".to_owned()
                })?;
                current
                    .vars
                    .push(parse_var(arg.trim_start_matches("--var="))?);
            }
            "--var-file" => {
                let Some(value) = args.next() else {
                    return Err("--var-file requires a path".to_owned());
                };
                let current = current.as_mut().ok_or_else(|| {
                    "--var-file must appear after --pass when --all is enabled".to_owned()
                })?;
                current.var_files.push(value.to_string_lossy().into_owned());
            }
            _ if arg.starts_with("--var-file=") => {
                let current = current.as_mut().ok_or_else(|| {
                    "--var-file must appear after --pass when --all is enabled".to_owned()
                })?;
                current
                    .var_files
                    .push(arg.trim_start_matches("--var-file=").to_owned());
            }
            _ => {}
        }
    }

    if let Some(group) = current {
        parsed.push(group);
    }

    Ok(parsed)
}

pub(crate) fn command_wants_json(command: &Command) -> bool {
    match command {
        Command::Render(args) => args.render.json,
        Command::Resolve(args) => args.json,
        Command::Validate(args) => args.json,
        Command::FrontmatterInit(args) => args.json,
        Command::Init(args) => args.json,
        Command::ObservabilityHealth(args) => args.json,
        Command::Examples(args) => match &args.command {
            Some(ExamplesSubcommand::List(list_args)) => list_args.json,
            None => args.render.json,
        },
        Command::Templates(args) => match &args.command {
            Some(TemplatesSubcommand::List(list_args)) => list_args.json,
            Some(TemplatesSubcommand::Add(add_args)) => add_args.json,
            None => args.render.json,
        },
        Command::Reports(args) => match &args.command {
            ReportsSubcommand::Init(init_args) => init_args.json,
            ReportsSubcommand::Smoke(smoke_args) => smoke_args.json,
            ReportsSubcommand::Finalize(finalize_args) => finalize_args.json,
            ReportsSubcommand::RenderSpec(render_args) => render_args.json,
            ReportsSubcommand::Index(index_args) => index_args.json,
            ReportsSubcommand::Verify(verify_args) => verify_args.json,
            ReportsSubcommand::PublishManifest(publish_args) => publish_args.json,
        },
        Command::ReportRenderMany(args) => args.json,
        Command::ReportCatalog(args) => args.json,
    }
}
