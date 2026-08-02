use std::ffi::OsString;
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
    #[command(about = "Verify deployed output matches a rendered template")]
    Verify(VerifyArgs),
    #[command(about = "Extract variables from a known XML template and rendered output")]
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
        value_parser = parse_var,
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
    #[arg(value_name = "RENDERED", help = "Rendered XML output file (required)")]
    pub(crate) rendered: PathBuf,
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

pub(crate) fn parse_var(input: &str) -> Result<(String, String), String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| "expected key=value".to_owned())?;
    Ok((key.to_owned(), value.to_owned()))
}

pub(crate) fn parse_pass_inputs<I>(
    args: I,
    command_name: &str,
) -> Result<Vec<PassInputArgs>, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
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
                let value = arg.strip_prefix("--pass=").unwrap_or(arg.as_ref());
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
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var must appear after --pass".to_owned())?;
                current.vars.push(parse_var(&value)?);
            }
            _ if arg.starts_with("--var=") => {
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var must appear after --pass".to_owned())?;
                current.vars.push(parse_var(
                    arg.strip_prefix("--var=").unwrap_or(arg.as_ref()),
                )?);
            }
            "--var-file" => {
                let Some(value) = args.next() else {
                    return Err("--var-file requires a path".to_owned());
                };
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var-file must appear after --pass".to_owned())?;
                current.var_files.push(value.to_string_lossy().into_owned());
            }
            _ if arg.starts_with("--var-file=") => {
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var-file must appear after --pass".to_owned())?;
                current.var_files.push(
                    arg.strip_prefix("--var-file=")
                        .unwrap_or(arg.as_ref())
                        .to_owned(),
                );
            }
            _ => {}
        }
    }

    if let Some(group) = current {
        parsed.push(group);
    }

    Ok(parsed)
}

pub(crate) fn parse_cli() -> Cli {
    Cli::parse_from(filtered_args_for_clap(std::env::args_os()))
}

pub(crate) fn filtered_args_for_clap<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut filtered = Vec::new();
    let mut args = args.into_iter();
    let Some(program) = args.next() else {
        return filtered;
    };
    filtered.push(program);

    let mut command_name: Option<String> = None;
    let mut in_pass_group = false;

    while let Some(arg) = args.next() {
        let arg_text = arg.to_string_lossy();
        if command_name.is_none() {
            if matches!(
                arg_text.as_ref(),
                "render" | "validate" | "verify" | "template-init"
            ) {
                command_name = Some(arg_text.into_owned());
                in_pass_group = false;
            }
            filtered.push(arg);
            continue;
        }

        if matches!(arg_text.as_ref(), "--pass") {
            in_pass_group = true;
            let _ = args.next();
            continue;
        }
        if arg_text.starts_with("--pass=") {
            in_pass_group = true;
            continue;
        }
        if in_pass_group && matches!(arg_text.as_ref(), "--var" | "--var-file") {
            let _ = args.next();
            continue;
        }
        if in_pass_group && (arg_text.starts_with("--var=") || arg_text.starts_with("--var-file="))
        {
            continue;
        }

        in_pass_group = false;
        filtered.push(arg);
    }

    filtered
}

pub(crate) fn command_wants_json(command: &Command) -> bool {
    match command {
        Command::Render(args) => args.render.json,
        Command::Resolve(args) => args.json,
        Command::Validate(args) => args.json,
        Command::Verify(args) => args.json,
        Command::Extract(args) => args.json,
        Command::TemplateInit(args) => args.json,
        Command::FrontmatterInit(args) => args.json,
        Command::Init(args) => args.json,
        Command::ObservabilityHealth(args) => args.json,
        Command::Examples(args) => args
            .command
            .as_ref()
            .map_or(args.render.json, |subcommand| match subcommand {
                ExamplesSubcommand::List(args) => args.json,
            }),
        Command::Templates(args) => args
            .command
            .as_ref()
            .map_or(args.render.json, |subcommand| match subcommand {
                TemplatesSubcommand::List(args) => args.json,
                TemplatesSubcommand::Add(args) => args.json,
            }),
        Command::Reports(args) => match &args.command {
            ReportsSubcommand::Init(args) => args.json,
            ReportsSubcommand::Smoke(args) => args.json,
            ReportsSubcommand::Finalize(args) => args.json,
            ReportsSubcommand::RenderSpec(args) => args.json,
            ReportsSubcommand::Index(args) => args.json,
            ReportsSubcommand::Verify(args) => args.json,
            ReportsSubcommand::PublishManifest(args) => args.json,
        },
        Command::ReportRenderMany(args) => args.json,
        Command::ReportCatalog(args) => args.json,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn json_capability(args: &[&str]) -> bool {
        let cli = Cli::try_parse_from(args).expect("valid CLI arguments");
        command_wants_json(&cli.command)
    }

    #[test]
    fn parse_pass_inputs_accepts_mixed_syntax_and_preserves_order() {
        let parsed = parse_pass_inputs(
            os_args(&[
                "sc-compose",
                "render",
                "--pass",
                "1",
                "--var",
                "first=one",
                "--var-file=one.json",
                "--pass=2",
                "--var=second=two",
                "--var-file",
                "two.yaml",
            ]),
            "render",
        )
        .expect("pass groups parse");

        assert_eq!(
            parsed,
            vec![
                PassInputArgs {
                    pass_number: 1,
                    vars: vec![("first".to_owned(), "one".to_owned())],
                    var_files: vec!["one.json".to_owned()],
                },
                PassInputArgs {
                    pass_number: 2,
                    vars: vec![("second".to_owned(), "two".to_owned())],
                    var_files: vec!["two.yaml".to_owned()],
                },
            ]
        );
    }

    #[test]
    fn parse_pass_inputs_rejects_malformed_and_misplaced_arguments() {
        assert_eq!(
            parse_pass_inputs(os_args(&["sc-compose", "render", "--pass"]), "render"),
            Err("--pass requires a numeric pass number".to_owned())
        );
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--pass", "1", "--var"]),
                "render"
            ),
            Err("--var requires key=value".to_owned())
        );
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--var=orphan=value"]),
                "render"
            ),
            Err("--var must appear after --pass".to_owned())
        );
        assert!(
            parse_pass_inputs(os_args(&["sc-compose", "render", "--pass=bad"]), "render")
                .unwrap_err()
                .starts_with("invalid pass number `bad`:")
        );
    }

    #[test]
    fn parse_pass_inputs_rejects_misplaced_var_file_arguments() {
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--var-file", "orphan.json"]),
                "render"
            ),
            Err("--var-file must appear after --pass".to_owned())
        );
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--var-file=orphan.json"]),
                "render"
            ),
            Err("--var-file must appear after --pass".to_owned())
        );
    }

    #[test]
    fn filtered_args_for_clap_removes_only_pass_scoped_arguments() {
        let filtered = filtered_args_for_clap(os_args(&[
            "sc-compose",
            "render",
            "--pass=1",
            "--var=first=one",
            "--file",
            "template.j2",
            "--pass",
            "2",
            "--var",
            "second=two",
            "--json",
        ]));

        assert_eq!(
            filtered,
            os_args(&["sc-compose", "render", "--file", "template.j2", "--json",])
        );
    }

    #[test]
    fn json_capability_covers_commands_and_nested_subcommands() {
        let json_commands: &[&[&str]] = &[
            &["sc-compose", "render", "--json"],
            &["sc-compose", "resolve", "--json"],
            &["sc-compose", "validate", "--json"],
            &["sc-compose", "verify", "deployed.txt", "--json"],
            &[
                "sc-compose",
                "extract",
                "template.xml.j2",
                "rendered.xml",
                "--json",
            ],
            &["sc-compose", "template-init", "template.txt", "--json"],
            &[
                "sc-compose",
                "frontmatter-init",
                "--file",
                "template.txt",
                "--json",
            ],
            &["sc-compose", "init", "--json"],
            &["sc-compose", "observability-health", "--json"],
            &["sc-compose", "examples", "--json"],
            &["sc-compose", "examples", "list", "--json"],
            &["sc-compose", "templates", "--json"],
            &["sc-compose", "templates", "list", "--json"],
            &["sc-compose", "templates", "add", "pack", "--json"],
            &["sc-compose", "reports", "init", "--json"],
            &[
                "sc-compose",
                "reports",
                "smoke",
                "--fixture",
                "fixture",
                "--vars",
                "vars",
                "--json",
            ],
            &[
                "sc-compose",
                "reports",
                "finalize",
                "--report-id",
                "id",
                "--kind",
                "kind",
                "--entrypoint",
                "report.html",
                "--json",
            ],
            &[
                "sc-compose",
                "reports",
                "render-spec",
                "--spec",
                "spec.toml",
                "--json",
            ],
            &["sc-compose", "reports", "index", "--json"],
            &["sc-compose", "reports", "verify", "--json"],
            &["sc-compose", "reports", "publish-manifest", "--json"],
            &[
                "sc-compose",
                "report-render-many",
                "--id",
                "id",
                "--glob",
                "*.txt",
                "--output-dir",
                "out",
                "--json",
            ],
            &["sc-compose", "report-catalog", "--json"],
        ];

        for args in json_commands {
            assert!(
                json_capability(args),
                "expected JSON capability for {args:?}"
            );
        }

        assert!(!json_capability(&["sc-compose", "render"]));
        assert!(!json_capability(&["sc-compose", "examples"]));
        assert!(!json_capability(&["sc-compose", "templates", "list"]));
    }
}
