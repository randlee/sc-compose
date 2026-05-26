mod commands;
mod exit_codes;
mod json_output;
mod observability;
mod observer_impl;
mod render_request;
mod reporting;
mod template_store;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use mimalloc::MiMalloc;
use sc_composer::{
    ComposeError, ComposeMode, ComposeRequest, CompositionObserver, Diagnostic, DiagnosticCode,
    DiagnosticSeverity, FrontmatterInitResult, RecoveryHint, RecoveryHintKind,
};

use crate::commands::examples::{run_examples_list, run_examples_render};
use crate::commands::templates::{run_templates_add, run_templates_list, run_templates_render};
use crate::observer_impl::{CommandEndEvent, CommandLifecycleObserver, CommandStartEvent};
use crate::render_request::{build_request, read_block_pair};
use crate::reporting::catalog::ReportCatalog;
use crate::reporting::index::{build_report_index, verify_required_reports};
use crate::reporting::init::{init_report_scaffold, run_smoke_report};
use crate::reporting::output::{FinalizeReportRequest, finalize_report_outputs};
use crate::reporting::publish_manifest::write_publish_manifest;
use crate::reporting::render_many::{RenderManyRequest, SourceSetDefinition, render_many};
use crate::reporting::spec::run_render_spec_report;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Debug, Parser)]
#[command(name = "sc-compose")]
#[command(version)]
#[command(about = "Standalone template composition CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
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
struct InputArgs {
    #[arg(
        long = "var",
        value_parser = parse_var,
        action = clap::ArgAction::Append,
        help = "Provide one explicit input variable as key=value"
    )]
    vars: Vec<(String, String)>,
    #[arg(
        long = "var-file",
        help = "Load input variables from a JSON or YAML object file"
    )]
    var_file: Option<String>,
    #[arg(
        long,
        help = "Absorb environment variables that match the given prefix"
    )]
    env_prefix: Option<String>,
    #[arg(long, help = "Treat undeclared referenced variables as errors")]
    strict: bool,
    #[arg(
        long,
        value_enum,
        default_value = "ignore",
        help = "Control how extra caller-provided variables are reported"
    )]
    unknown_var_mode: UnknownVarMode,
}

#[derive(Debug, Clone, Args)]
struct CommonArgs {
    #[arg(
        long,
        value_enum,
        default_value = "file",
        help = "Choose file or profile resolution mode"
    )]
    mode: Mode,
    #[arg(
        long,
        value_enum,
        default_value = "agent",
        help = "Choose the profile kind in profile mode"
    )]
    kind: Kind,
    #[arg(long, help = "Profile name in profile mode")]
    agent: Option<String>,
    #[arg(long, alias = "agent-type", help = "Alias for --agent")]
    agent_type: Option<String>,
    #[arg(
        long,
        alias = "ai",
        value_enum,
        help = "Optional runtime selector in profile mode"
    )]
    runtime: Option<Ai>,
    #[command(flatten)]
    input: InputArgs,
    #[arg(
        long,
        default_value = ".",
        help = "Workspace root for resolution and confinement"
    )]
    root: PathBuf,
    #[arg(long, help = "Template path in file mode")]
    file: Option<PathBuf>,
}

#[derive(Debug, Clone, Args, Default)]
struct RenderBehaviorArgs {
    #[arg(
        long,
        help = "Write rendered output to the given path instead of stdout"
    )]
    output: Option<PathBuf>,
    #[arg(long, help = "Append a guidance block after the rendered body")]
    guidance: Option<String>,
    #[arg(long, help = "Read the guidance block from a file or stdin")]
    guidance_file: Option<String>,
    #[arg(
        long,
        help = "Append a user prompt block after the rendered body and guidance"
    )]
    prompt: Option<String>,
    #[arg(long, help = "Read the user prompt block from a file or stdin")]
    prompt_file: Option<String>,
    #[arg(long, help = "Emit machine-readable JSON output")]
    json: bool,
    #[arg(long, help = "Report the derived output target without writing files")]
    dry_run: bool,
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
    #[command(flatten)]
    render: RenderBehaviorArgs,
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

#[derive(Debug, Clone, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ExamplesArgs {
    #[command(subcommand)]
    command: Option<ExamplesSubcommand>,
    #[arg(index = 1, help = "Bundled example pack name to render")]
    name: Option<String>,
    #[command(flatten)]
    input: InputArgs,
    #[command(flatten)]
    render: RenderBehaviorArgs,
}

#[derive(Debug, Clone, Subcommand)]
enum ExamplesSubcommand {
    #[command(about = "List bundled example packs")]
    List(ListArgs),
}

#[derive(Debug, Clone, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct TemplatesArgs {
    #[command(subcommand)]
    command: Option<TemplatesSubcommand>,
    #[arg(index = 1, help = "User template pack name to render")]
    name: Option<String>,
    #[command(flatten)]
    input: InputArgs,
    #[command(flatten)]
    render: RenderBehaviorArgs,
}

#[derive(Debug, Clone, Subcommand)]
enum TemplatesSubcommand {
    #[command(about = "List user template packs")]
    List(ListArgs),
    #[command(about = "Import a file or directory as one user template pack")]
    Add(TemplatesAddArgs),
}

#[derive(Debug, Clone, Args)]
struct ListArgs {
    #[arg(long, help = "Emit machine-readable JSON output")]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct TemplatesAddArgs {
    /// Source file or directory to import as one template pack.
    src: PathBuf,
    /// Optional pack name override.
    name: Option<String>,
    #[arg(long, help = "Emit machine-readable JSON output")]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportsArgs {
    #[command(subcommand)]
    command: ReportsSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
enum ReportsSubcommand {
    #[command(about = "Initialize the shared reports scaffold")]
    Init(ReportsInitArgs),
    #[command(about = "Run the shared smoke-report fixture harness")]
    Smoke(ReportsSmokeArgs),
    #[command(about = "Write shared report metadata and archive outputs for one producer result")]
    Finalize(ReportsFinalizeArgs),
    #[command(about = "Render one semantic diagram spec to Mermaid output")]
    RenderSpec(ReportsRenderSpecArgs),
    #[command(about = "Summarize latest report entrypoints and sidecars")]
    Index(ReportsIndexArgs),
    #[command(about = "Verify required report evidence from the catalog")]
    Verify(ReportsVerifyArgs),
    #[command(about = "Write one machine-readable publish manifest for current report outputs")]
    PublishManifest(ReportsPublishManifestArgs),
}

#[derive(Debug, Clone, Args)]
struct ReportsInitArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportsSmokeArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    fixture: PathBuf,
    #[arg(long)]
    vars: PathBuf,
    #[arg(long)]
    archive: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportsFinalizeArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long = "report-id")]
    report_id: String,
    #[arg(long)]
    kind: String,
    #[arg(long, default_value = "pass")]
    status: String,
    #[arg(long)]
    entrypoint: PathBuf,
    #[arg(long = "artifact", action = clap::ArgAction::Append)]
    artifacts: Vec<PathBuf>,
    #[arg(long)]
    archive: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportsIndexArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportsVerifyArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportsRenderSpecArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long = "spec")]
    spec_path: PathBuf,
    #[arg(long)]
    archive: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportsPublishManifestArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportCatalogArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ReportRenderManyArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    id: String,
    #[arg(long)]
    glob: String,
    #[arg(long, conflicts_with = "template_family")]
    template: Option<String>,
    #[arg(long = "template-family", conflicts_with = "template")]
    template_family: Option<String>,
    #[arg(long = "output-dir")]
    output_dir: PathBuf,
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
    let mut observer =
        match observability::build_logger(wants_json).map(observer_impl::CliObserver::new) {
            Ok(observer) => observer,
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
                std::process::exit(error.exit_code);
            }
        };
    let code = match run(cli, &mut observer) {
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
    if let Err(error) = observer.shutdown() {
        eprintln!("{error}");
    }
    std::process::exit(code);
}

fn run(cli: Cli, observer: &mut observer_impl::CliObserver) -> Result<i32, CommandError> {
    match cli.command {
        Command::Render(args) => {
            observe_command(observer, "render", args.render.json, |observer| {
                run_render(&args, observer)
            })
        }
        Command::Resolve(args) => observe_command(observer, "resolve", args.json, |observer| {
            run_resolve(&args, observer)
        }),
        Command::Validate(args) => observe_command(observer, "validate", args.json, |observer| {
            run_validate(&args, observer)
        }),
        Command::FrontmatterInit(args) => {
            observe_command(observer, "frontmatter-init", args.json, |_observer| {
                run_frontmatter_init(&args)
            })
        }
        Command::Init(args) => {
            observe_command(observer, "init", args.json, |_observer| run_init(&args))
        }
        Command::ObservabilityHealth(args) => {
            observe_command(observer, "observability-health", args.json, |observer| {
                run_observability_health(&args, observer)
            })
        }
        Command::Examples(args) => match &args.command {
            Some(ExamplesSubcommand::List(list_args)) => {
                observe_command(observer, "examples", list_args.json, |_observer| {
                    run_examples_list(list_args)
                })
            }
            None => observe_command(observer, "examples", args.render.json, |observer| {
                run_examples_render(&args, observer)
            }),
        },
        Command::Templates(args) => match &args.command {
            Some(TemplatesSubcommand::List(list_args)) => {
                observe_command(observer, "templates", list_args.json, |_observer| {
                    run_templates_list(list_args)
                })
            }
            Some(TemplatesSubcommand::Add(add_args)) => {
                observe_command(observer, "templates", add_args.json, |_observer| {
                    run_templates_add(add_args)
                })
            }
            None => observe_command(observer, "templates", args.render.json, |observer| {
                run_templates_render(&args, observer)
            }),
        },
        Command::Reports(args) => run_reports_command(&args, observer),
        Command::ReportRenderMany(args) => {
            observe_command(observer, "report-render-many", args.json, |_observer| {
                run_report_render_many(args)
            })
        }
        Command::ReportCatalog(args) => {
            observe_command(observer, "report-catalog", args.json, |_observer| {
                run_report_catalog(&args)
            })
        }
    }
}

fn run_reports_init(args: &ReportsInitArgs) -> Result<i32, CommandError> {
    let result = init_report_scaffold(&args.root)?;
    if args.json {
        let payload = serde_json::json!({
            "workspace_root": result.workspace_root.display().to_string(),
            "created_paths": result.created_paths,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("workspace_root: {}", result.workspace_root.display());
        for path in &result.created_paths {
            println!("created: {path}");
        }
    }
    Ok(exit_codes::SUCCESS)
}

fn run_reports_command(
    args: &ReportsArgs,
    observer: &mut observer_impl::CliObserver,
) -> Result<i32, CommandError> {
    match &args.command {
        ReportsSubcommand::Init(init_args) => {
            observe_command(observer, "reports-init", init_args.json, |_observer| {
                run_reports_init(init_args)
            })
        }
        ReportsSubcommand::Smoke(smoke_args) => {
            observe_command(observer, "reports-smoke", smoke_args.json, |observer| {
                run_reports_smoke(smoke_args, observer)
            })
        }
        ReportsSubcommand::Finalize(finalize_args) => observe_command(
            observer,
            "reports-finalize",
            finalize_args.json,
            |_observer| run_reports_finalize(finalize_args),
        ),
        ReportsSubcommand::RenderSpec(render_args) => observe_command(
            observer,
            "reports-render-spec",
            render_args.json,
            |observer| run_reports_render_spec(render_args, observer),
        ),
        ReportsSubcommand::Index(index_args) => {
            observe_command(observer, "reports-index", index_args.json, |_observer| {
                run_reports_index(index_args)
            })
        }
        ReportsSubcommand::Verify(verify_args) => {
            observe_command(observer, "reports-verify", verify_args.json, |_observer| {
                run_reports_verify(verify_args)
            })
        }
        ReportsSubcommand::PublishManifest(publish_args) => observe_command(
            observer,
            "reports-publish-manifest",
            publish_args.json,
            |_observer| run_reports_publish_manifest(publish_args),
        ),
    }
}

fn run_reports_smoke(
    args: &ReportsSmokeArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let result = run_smoke_report(
        &args.root,
        &args.fixture,
        &args.vars,
        args.archive,
        observer,
    )?;
    if args.json {
        let payload = serde_json::json!({
            "report_id": result.report_id,
            "kind": result.kind,
            "produced_at": result.produced_at,
            "status": result.status,
            "entrypoint": result.entrypoint.display().to_string(),
            "metadata": result.metadata.display().to_string(),
            "artifacts": result.artifacts.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "archived_artifacts": result.archived_artifacts.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        });
        print_json(payload, result.warnings).map_err(CommandError::usage)?;
    } else {
        println!("report_id: {}", result.report_id);
        println!("kind: {}", result.kind);
        println!("produced_at: {}", result.produced_at);
        println!("status: {}", result.status);
        println!("entrypoint: {}", result.entrypoint.display());
        println!("metadata: {}", result.metadata.display());
        for artifact in &result.artifacts {
            println!("artifact: {}", artifact.display());
        }
        for artifact in &result.archived_artifacts {
            println!("archived: {}", artifact.display());
        }
        if !result.warnings.is_empty() {
            print_diagnostic_messages(&result.warnings);
        }
    }
    Ok(exit_codes::SUCCESS)
}

fn run_reports_render_spec(
    args: &ReportsRenderSpecArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let result = run_render_spec_report(&args.root, &args.spec_path, args.archive, observer)?;
    if args.json {
        let payload = serde_json::json!({
            "report_id": result.report_id,
            "kind": result.kind,
            "produced_at": result.produced_at,
            "status": result.status,
            "entrypoint": result.entrypoint.display().to_string(),
            "metadata": result.metadata.display().to_string(),
            "artifacts": result.artifacts.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "archived_artifacts": result.archived_artifacts.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        });
        print_json(payload, result.warnings).map_err(CommandError::usage)?;
    } else {
        println!("report_id: {}", result.report_id);
        println!("kind: {}", result.kind);
        println!("produced_at: {}", result.produced_at);
        println!("status: {}", result.status);
        println!("entrypoint: {}", result.entrypoint.display());
        println!("metadata: {}", result.metadata.display());
        for artifact in &result.artifacts {
            println!("artifact: {}", artifact.display());
        }
        for artifact in &result.archived_artifacts {
            println!("archived: {}", artifact.display());
        }
    }
    Ok(exit_codes::SUCCESS)
}

fn run_reports_finalize(args: &ReportsFinalizeArgs) -> Result<i32, CommandError> {
    let result = finalize_report_outputs(
        &args.root,
        &FinalizeReportRequest {
            report_id: args.report_id.clone(),
            kind: args.kind.clone(),
            status: args.status.clone(),
            entrypoint: args.entrypoint.clone(),
            artifacts: args.artifacts.clone(),
            archive: args.archive,
        },
    )
    .map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    if args.json {
        let payload = serde_json::json!({
            "report_id": result.report_id,
            "kind": result.kind,
            "produced_at": result.produced_at,
            "status": result.status,
            "entrypoint": result.entrypoint.display().to_string(),
            "metadata": result.metadata.display().to_string(),
            "artifacts": result.latest_artifacts.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "archived_artifacts": result.archived_artifacts.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("report_id: {}", result.report_id);
        println!("kind: {}", result.kind);
        println!("produced_at: {}", result.produced_at);
        println!("status: {}", result.status);
        println!("entrypoint: {}", result.entrypoint.display());
        println!("metadata: {}", result.metadata.display());
        for artifact in &result.latest_artifacts {
            println!("artifact: {}", artifact.display());
        }
        for artifact in &result.archived_artifacts {
            println!("archived: {}", artifact.display());
        }
    }
    Ok(exit_codes::SUCCESS)
}

fn run_reports_index(args: &ReportsIndexArgs) -> Result<i32, CommandError> {
    let index = build_report_index(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;
    if args.json {
        let payload = serde_json::json!({
            "report_count": index.entries.len(),
            "entries": index.entries,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("reports: {}", index.entries.len());
        for entry in &index.entries {
            println!(
                "{} kind={} required={} status={} entrypoint={} metadata={}",
                entry.report_id,
                entry.kind,
                entry.required,
                entry.status.as_deref().unwrap_or("missing"),
                entry.entrypoint.display(),
                entry.metadata.display()
            );
            if !entry.missing_paths.is_empty() {
                for missing in &entry.missing_paths {
                    println!("missing: {}", missing.display());
                }
            }
        }
    }
    Ok(exit_codes::SUCCESS)
}

fn run_reports_verify(args: &ReportsVerifyArgs) -> Result<i32, CommandError> {
    let result = verify_required_reports(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;
    if args.json {
        let payload = serde_json::json!({
            "required_count": result.required_count,
            "verified_count": result.verified_count,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!(
            "verified required reports: {}/{}",
            result.verified_count, result.required_count
        );
    }
    Ok(exit_codes::SUCCESS)
}

fn run_reports_publish_manifest(args: &ReportsPublishManifestArgs) -> Result<i32, CommandError> {
    let result = write_publish_manifest(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;
    if args.json {
        let payload = serde_json::json!({
            "manifest_path": result.manifest_path.display().to_string(),
            "report_count": result.report_count,
            "manifest": result.manifest,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("manifest: {}", result.manifest_path.display());
        println!("reports: {}", result.report_count);
        for report in &result.manifest.reports {
            println!(
                "{} kind={} entrypoint={}",
                report.report_id,
                report.kind,
                report.entrypoint.display()
            );
            if let Some(archive_root) = &report.archive_root {
                println!("archive_root: {}", archive_root.display());
            }
            for file in &report.files {
                println!(
                    "file role={} path={} publish_to={}",
                    file.role,
                    file.path.display(),
                    file.publish_to.display()
                );
            }
        }
    }
    Ok(exit_codes::SUCCESS)
}

fn run_report_catalog(args: &ReportCatalogArgs) -> Result<i32, CommandError> {
    let catalog = ReportCatalog::load(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    if args.json {
        let payload = serde_json::json!({
            "catalog_path": catalog.catalog_path.display().to_string(),
            "report_count": catalog.reports.len(),
            "reports": catalog.reports,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("catalog: {}", catalog.catalog_path.display());
        println!("reports: {}", catalog.reports.len());
        for report in &catalog.reports {
            println!(
                "{} kind={} producer={} required={} entrypoint={} metadata={}",
                report.id,
                report.kind,
                report.producer,
                report.required,
                report.entrypoint.display(),
                report.metadata.display()
            );
        }
    }

    Ok(exit_codes::SUCCESS)
}

fn run_report_render_many(args: ReportRenderManyArgs) -> Result<i32, CommandError> {
    let template_selector = args.template.clone().unwrap_or_default();
    let request = RenderManyRequest {
        root: args.root,
        source_set: SourceSetDefinition {
            id: args.id,
            glob: args.glob,
            template_selector,
            template_family: args.template_family,
            output_dir: args.output_dir,
        },
    };
    let result = render_many(&request).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    if args.json {
        let payload = serde_json::json!({
            "manifest_path": result.manifest_path.display().to_string(),
            "output_count": result.generated_outputs.len(),
            "generated_outputs": result.generated_outputs.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "entries": result.entries,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("manifest: {}", result.manifest_path.display());
        println!("outputs: {}", result.generated_outputs.len());
        for output in &result.generated_outputs {
            println!("generated: {}", output.display());
        }
    }

    Ok(exit_codes::SUCCESS)
}

fn run_render(
    args: &RenderArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let request = build_request(
        &args.common,
        read_block_pair(&args.common.input, &args.render)?,
        std::collections::BTreeMap::default(),
    )?;
    execute_render(&request, &args.render, observer)
}

fn execute_render(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let result =
        sc_composer::compose_with_observer(request, observer).map_err(CommandError::compose)?;
    let output_path = args.output.clone();
    let derived_path = derived_output_path(request, output_path.as_deref());
    let would_change = render_would_change(&derived_path, &result.rendered_text);
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
                "would_change": would_change,
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
        println!("would_change: {would_change}");
        if !result.warnings.is_empty() {
            println!();
            print_diagnostic_messages(&result.warnings);
        }
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
    let request = build_request(
        &args.common,
        (None, None),
        std::collections::BTreeMap::default(),
    )?;
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
    let request = build_request(
        &args.common,
        (None, None),
        std::collections::BTreeMap::default(),
    )?;
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
    let prompts_dir_missing = !canonical_root.join(".prompts").exists();
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
                "created_files": actual_init_created_files(
                    prompts_dir_missing,
                    result.gitignore_updated,
                ),
            })
        };
        print_json(payload, result.recommendations).map_err(CommandError::usage)?;
    } else if args.dry_run {
        for path in &planned_changes {
            println!("would_affect: {}", path.display());
        }
        print_diagnostic_messages(&result.recommendations);
    } else {
        println!("workspace_root: {}", canonical_root.display());
        print_diagnostic_messages(&result.recommendations);
    }
    Ok(if result.validation_passed {
        exit_codes::SUCCESS
    } else {
        exit_codes::VALIDATION_OR_RENDER_FAIL
    })
}

fn run_observability_health(
    args: &ObservabilityHealthArgs,
    observer: &mut observer_impl::CliObserver,
) -> Result<i32, CommandError> {
    if std::env::var_os("SC_COMPOSE_TEST_FORCE_QUERY_UNAVAILABLE").is_some() {
        observer.shutdown().map_err(|error| {
            CommandError::usage(anyhow!(error).context("failed to shut down observability logger"))
        })?;
    }
    let health = observer.health();
    if args.json {
        print_json(
            serde_json::json!({ "logging": observability::health_json_value(&health) }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else {
        observability::print_observability_health(&health);
    }
    Ok(exit_codes::SUCCESS)
}

fn derived_output_path(request: &ComposeRequest, explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    match &request.mode {
        ComposeMode::File { template_path } => strip_j2_suffix(template_path),
        // Profile-mode dry-runs intentionally derive a fresh ULID so the reported
        // target matches the real non-dry-run naming policy and avoids collisions.
        ComposeMode::Profile { name, .. } => request.root.as_path().join(".prompts").join(format!(
            "{}-{}.md",
            name,
            ulid::Ulid::new()
        )),
    }
}

fn render_would_change(output_path: &Path, rendered_text: &str) -> bool {
    match std::fs::read(output_path) {
        Ok(existing) => existing != rendered_text.as_bytes(),
        Err(_) => true,
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

fn print_diagnostic_messages(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        println!("{}", diagnostic.message);
    }
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
        Command::Reports(args) => reports_command_wants_json(args),
        Command::ReportRenderMany(args) => args.json,
        Command::ReportCatalog(args) => args.json,
    }
}

fn reports_command_wants_json(args: &ReportsArgs) -> bool {
    match &args.command {
        ReportsSubcommand::Init(init_args) => init_args.json,
        ReportsSubcommand::Smoke(smoke_args) => smoke_args.json,
        ReportsSubcommand::Finalize(finalize_args) => finalize_args.json,
        ReportsSubcommand::RenderSpec(render_args) => render_args.json,
        ReportsSubcommand::Index(index_args) => index_args.json,
        ReportsSubcommand::Verify(verify_args) => verify_args.json,
        ReportsSubcommand::PublishManifest(publish_args) => publish_args.json,
    }
}

fn observe_command<O>(
    observer: &mut O,
    command_name: &str,
    json_output: bool,
    action: impl FnOnce(&mut O) -> Result<i32, CommandError>,
) -> Result<i32, CommandError>
where
    O: CompositionObserver + CommandLifecycleObserver,
{
    let started = Instant::now();
    observer.on_command_start(&CommandStartEvent {
        command_name: command_name.to_owned(),
        json_output,
    });
    let result = action(observer);
    let exit_code = match &result {
        Ok(code) => *code,
        Err(error) => error.exit_code,
    };
    observer.on_command_end(&CommandEndEvent {
        command_name: command_name.to_owned(),
        exit_code,
        success: result.is_ok(),
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        json_output,
        diagnostic_code: result
            .as_ref()
            .err()
            .and_then(|error| error.diagnostic_code.map(|code| code.as_str().to_owned())),
        diagnostic_message: result.as_ref().err().and_then(|error| {
            error
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
        }),
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
    let severity = diagnostic.severity.to_string();
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

fn format_recovery_hint(hint: &RecoveryHint) -> String {
    match &hint.kind {
        RecoveryHintKind::RunCommand { command } => format!("run `{command}`"),
        RecoveryHintKind::InspectPath { path } => format!("inspect {}", path.display()),
        RecoveryHintKind::ProvideVariable { variable } => {
            format!("provide variable `{}`", variable.as_str())
        }
        RecoveryHintKind::ReviewConfiguration { key } => {
            format!("review configuration: {key}")
        }
    }
}

fn actual_init_created_files(prompts_dir_missing: bool, gitignore_updated: bool) -> Vec<String> {
    let mut created = Vec::new();
    if prompts_dir_missing {
        created.push(".prompts/".to_owned());
    }
    if gitignore_updated {
        created.push(".gitignore".to_owned());
    }
    created
}

#[derive(Debug)]
struct CommandError {
    exit_code: i32,
    diagnostic_code: Option<DiagnosticCode>,
    diagnostics: Vec<Diagnostic>,
    recovery_hints: Vec<RecoveryHint>,
    error: anyhow::Error,
}

impl CommandError {
    fn usage(error: anyhow::Error) -> Self {
        Self {
            exit_code: exit_codes::USAGE_FAIL,
            diagnostic_code: None,
            diagnostics: Vec::new(),
            recovery_hints: Vec::new(),
            error,
        }
    }

    fn usage_with_code(error: anyhow::Error, diagnostic_code: DiagnosticCode) -> Self {
        Self::usage_with_code_and_hints(error, diagnostic_code, Vec::new())
    }

    fn usage_with_code_and_hints(
        error: anyhow::Error,
        diagnostic_code: DiagnosticCode,
        recovery_hints: Vec<RecoveryHint>,
    ) -> Self {
        Self {
            exit_code: exit_codes::USAGE_FAIL,
            diagnostic_code: Some(diagnostic_code),
            diagnostics: vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                diagnostic_code,
                format!("{error:#}"),
            )],
            recovery_hints,
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
            recovery_hints: compose_error_recovery_hints(&error),
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
            recovery_hints: Vec::new(),
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
            recovery_hints: Vec::new(),
            error: anyhow!("guidance and prompt cannot both read from stdin"),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = self.diagnostic_code {
            write!(f, "{}: {:#}", code.as_str(), self.error)?;
        } else {
            write!(f, "{:#}", self.error)?;
        }
        for hint in &self.recovery_hints {
            write!(f, "\nrecovery: {}", format_recovery_hint(hint))?;
        }
        Ok(())
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
            resolve.code(),
            resolve.message(),
        )],
        ComposeError::Include(include) => vec![
            Diagnostic::new(DiagnosticSeverity::Error, include.code(), include.message())
                .with_include_chain(include.include_chain().to_vec()),
        ],
        ComposeError::Validation(validation) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            validation.code(),
            validation.message(),
        )],
        ComposeError::Render(render) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            render.code().unwrap_or(DiagnosticCode::ErrRenderWrite),
            render.message(),
        )],
        ComposeError::Config(config) => vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            config.code(),
            config.message(),
        )],
    }
}

fn compose_error_recovery_hints(error: &ComposeError) -> Vec<RecoveryHint> {
    match error {
        ComposeError::Validation(error) => error.recovery_hints().to_vec(),
        ComposeError::Config(error) => error.recovery_hints().to_vec(),
        ComposeError::Resolve(_) | ComposeError::Include(_) | ComposeError::Render(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{CommandError, observe_command};
    use anyhow::anyhow;
    use sc_composer::{CompositionObserver, DiagnosticCode};
    use sc_observability_types::{MaintenanceWorkerState, QueryHealthState};

    use crate::exit_codes;
    use crate::observability::build_logger_for_root;
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

        let result = observe_command(&mut observer, "render", false, |_observer| Ok(0));

        assert_eq!(result.unwrap(), 0);
        assert_eq!(observer.started.len(), 1);
        assert_eq!(observer.ended.len(), 1);
        assert_eq!(observer.started[0].command_name, "render");
        assert!(!observer.started[0].json_output);
        assert_eq!(observer.ended[0].exit_code, 0);
        assert!(observer.ended[0].success);
    }

    #[test]
    fn observe_command_treats_successful_nonzero_exit_as_success() {
        let mut observer = CapturingObserver::default();

        let result = observe_command(&mut observer, "validate", true, |_observer| Ok(2));

        assert_eq!(result.unwrap(), 2);
        assert_eq!(observer.started.len(), 1);
        assert_eq!(observer.ended.len(), 1);
        assert_eq!(observer.ended[0].exit_code, 2);
        assert!(observer.ended[0].success);
    }

    #[test]
    fn observe_command_emits_start_and_end_for_failure() {
        let mut observer = CapturingObserver::default();

        let result = observe_command(&mut observer, "render", true, |_observer| {
            Err(CommandError::usage_with_code(
                anyhow!("boom"),
                DiagnosticCode::ErrRenderStdinDoubleRead,
            ))
        });

        let _ = result.unwrap_err();
        assert_eq!(observer.started.len(), 1);
        assert_eq!(observer.ended.len(), 1);
        assert!(observer.started[0].json_output);
        assert_eq!(observer.ended[0].exit_code, exit_codes::USAGE_FAIL);
        assert!(!observer.ended[0].success);
        assert_eq!(
            observer.ended[0].diagnostic_code.as_deref(),
            Some(DiagnosticCode::ErrRenderStdinDoubleRead.as_str())
        );
    }

    #[test]
    fn build_logger_disables_console_sink_for_json_output() {
        let logger = build_logger_for_root(temp_root("logger-json"), true).expect("logger");
        let health = logger.health();

        assert_eq!(health.sink_statuses.len(), 1);
        assert_eq!(health.sink_statuses[0].name.as_str(), "jsonl-file");
    }

    #[test]
    fn build_logger_enables_console_sink_for_text_output() {
        let logger = build_logger_for_root(temp_root("logger-text"), false).expect("logger");
        let health = logger.health();

        assert_eq!(health.sink_statuses.len(), 2);
        assert!(
            health
                .sink_statuses
                .iter()
                .any(|sink| sink.name.as_str() == "console")
        );
    }

    #[test]
    fn build_logger_enables_retained_log_maintenance_by_default() {
        let logger = build_logger_for_root(temp_root("logger-maintenance"), false).expect("logger");
        let health = logger.health();

        assert_eq!(
            health
                .maintenance
                .expect("maintenance health present")
                .state,
            MaintenanceWorkerState::Running
        );
    }

    #[test]
    fn shutdown_marks_query_health_unavailable() {
        let logger = build_logger_for_root(temp_root("logger-shutdown"), false).expect("logger");
        let mut observer = crate::observer_impl::CliObserver::new(logger);

        assert_eq!(
            observer.health().query.expect("query health present").state,
            QueryHealthState::Healthy
        );
        assert_eq!(
            observer
                .health()
                .maintenance
                .expect("maintenance health present")
                .state,
            MaintenanceWorkerState::Running
        );

        observer.shutdown().expect("shutdown");

        assert_eq!(
            observer.health().query.expect("query health present").state,
            QueryHealthState::Unavailable
        );
        assert_eq!(
            observer
                .health()
                .maintenance
                .expect("maintenance health present")
                .state,
            MaintenanceWorkerState::Stopped
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn build_logger_reports_usage_error_when_current_directory_is_unavailable() {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("lock current-dir guard");
        let original_dir = std::env::current_dir().expect("current dir");
        let missing_dir = temp_root("logger-missing-cwd").join("gone");
        fs::create_dir_all(&missing_dir).expect("create missing dir");
        std::env::set_current_dir(&missing_dir).expect("enter missing dir");
        fs::remove_dir_all(&missing_dir).expect("remove current dir");

        let result = crate::observability::build_logger(false);

        std::env::set_current_dir(&original_dir).expect("restore current dir");

        let Err(error) = result else {
            panic!("logger build should fail");
        };
        assert_eq!(error.exit_code, exit_codes::USAGE_FAIL);
        assert!(format!("{error}").contains("failed to determine current directory"));
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
