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
        help = "Load input variables from a JSON or YAML object file"
    )]
    pub(crate) var_file: Option<String>,
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
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct RenderArgs {
    #[command(flatten)]
    pub(crate) common: CommonArgs,
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
