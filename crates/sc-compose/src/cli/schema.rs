use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

pub(crate) use crate::commands::reports::{
    ReportCatalogArgs, ReportRenderManyArgs, ReportsArgs, ReportsSubcommand,
};

#[derive(Debug, clap::Parser)]
#[command(name = "sc-compose")]
#[command(version)]
#[command(about = "Standalone template composition CLI")]
#[command(disable_help_subcommand = true)]
#[command(
    after_help = "Detailed feature manuals ship with this CLI — run `sc-compose help` (or `sc-compose help <topic>`) to read them, starting from the exit-code contract."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(about = "Run an allowlisted sc-lint target and write its report")]
    Lint(ScLintArgs),
    #[command(about = "Show a feature manual, or list available manual topics")]
    Help(HelpArgs),
    #[command(about = "Render a template or resolved profile")]
    Render(RenderArgs),
    #[command(about = "Resolve a profile name to a concrete template path")]
    Resolve(ResolveArgs),
    #[command(about = "Validate templates without rendering output")]
    Validate(ValidateArgs),
    #[command(about = "Verify deployed output matches a rendered template")]
    Verify(VerifyArgs),
    #[command(about = "Extract variables from a known template and rendered output")]
    Extract(ExtractArgs),
    #[command(name = "template-init")]
    #[command(about = "Convert a concrete file into a template using pass-scoped replacements")]
    TemplateInit(TemplateInitArgs),
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
pub(crate) struct HelpArgs {
    #[arg(
        value_name = "TOPIC",
        conflicts_with = "list",
        help = "Manual topic to display"
    )]
    pub(crate) topic: Option<String>,
    #[arg(long, conflicts_with = "topic", help = "List available manual topics")]
    pub(crate) list: bool,
    #[arg(long, help = "Emit the manual response as a diagnostic JSON envelope")]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ScLintArgs {
    #[arg(long, default_value = "full", help = "Allowlisted sc-lint target")]
    pub(crate) target: String,
    #[arg(long, default_value = ".", help = "Repository root passed to sc-lint")]
    pub(crate) root: PathBuf,
    #[arg(long, help = "Emit the sc-lint envelope as machine-readable JSON")]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct InputArgs {
    #[arg(
        long = "var",
        value_parser = super::parse_var,
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
        help = "Control how extra caller-provided and referenced-but-unbound variables are reported"
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
    #[arg(
        long,
        value_enum,
        help = "Select JSON interpolation mode; default is auto unless frontmatter declares one"
    )]
    pub(crate) json_escape_mode: Option<JsonEscapeModeArg>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum JsonEscapeModeArg {
    /// Render complete JSON values, including string quotes.
    Auto,
    /// Preserve manually quoted JSON string placeholders safely.
    Legacy,
}

impl From<JsonEscapeModeArg> for sc_composer::JsonEscapeMode {
    fn from(value: JsonEscapeModeArg) -> Self {
        match value {
            JsonEscapeModeArg::Auto => Self::Auto,
            JsonEscapeModeArg::Legacy => Self::Legacy,
        }
    }
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
        long,
        help = "Report redundant filter chains and other lint findings with source locations"
    )]
    pub(crate) lint: bool,
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
pub(crate) struct VerifyArgs {
    #[command(flatten)]
    pub(crate) common: CommonArgs,
    #[arg(long, help = "Template path in file mode")]
    pub(crate) against: Option<PathBuf>,
    #[arg(long, help = "Verify all stacked template passes")]
    pub(crate) all: bool,
    #[arg(long, help = "Suppress diff output when drift is detected")]
    pub(crate) quiet: bool,
    #[arg(
        long = "builtin-var",
        value_parser = super::parse_var,
        action = clap::ArgAction::Append,
        help = "Override one builtin variable as key=value for deterministic verification"
    )]
    pub(crate) builtin_vars: Vec<(String, String)>,
    #[arg(long, help = "Emit machine-readable JSON output")]
    pub(crate) json: bool,
    #[arg(help = "Concrete deployed file to compare against")]
    pub(crate) deployed: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ExtractArgs {
    #[arg(value_name = "TEMPLATE", help = "Known template file (required)")]
    pub(crate) template: PathBuf,
    #[arg(value_name = "RENDERED", help = "Rendered output file (required)")]
    pub(crate) rendered: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value = "xml",
        help = "Rendered format: xml, json, yaml, toml, or raw known-template text"
    )]
    pub(crate) format: ExtractFormatArg,
    #[arg(
        long,
        value_name = "NAME",
        action = clap::ArgAction::Append,
        help = "Only recover this variable; repeat for multiple names"
    )]
    pub(crate) include: Vec<String>,
    #[arg(
        long,
        value_name = "NAME",
        action = clap::ArgAction::Append,
        help = "Exclude this variable; repeat for multiple names"
    )]
    pub(crate) exclude: Vec<String>,
    #[arg(
        long,
        help = "Emit machine-readable JSON output (format is XML by default)"
    )]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ExtractFormatArg {
    Xml,
    Json,
    Yaml,
    Toml,
    Raw,
}

impl ExtractFormatArg {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Xml => "xml",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Raw => "raw",
        }
    }
}

impl From<ExtractFormatArg> for sc_composer::ExtractFormat {
    fn from(format: ExtractFormatArg) -> Self {
        match format {
            ExtractFormatArg::Xml => Self::Xml,
            ExtractFormatArg::Json => Self::Json,
            ExtractFormatArg::Yaml => Self::Yaml,
            ExtractFormatArg::Toml => Self::Toml,
            ExtractFormatArg::Raw => Self::Raw,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct TemplateInitArgs {
    #[arg(help = "Concrete file to convert into a template")]
    pub(crate) file: PathBuf,
    #[arg(long)]
    pub(crate) force: bool,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) dry_run: bool,
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
