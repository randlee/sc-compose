use std::path::PathBuf;

use anyhow::anyhow;
use clap::{Args, Subcommand};
use sc_composer::{CompositionObserver, DiagnosticCode};

use crate::exit_codes;
use crate::path_utils::to_forward_slash;
use crate::reporting::catalog::{REPORT_CATALOG_RELATIVE_PATH, ReportCatalog};
use crate::reporting::init::{init_report_scaffold, run_smoke_report};
use crate::reporting::render_many::{RenderManyRequest, SourceSetDefinition, render_many};
use crate::{CommandError, print_diagnostic_messages, print_json};

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsArgs {
    #[command(subcommand)]
    pub(crate) command: ReportsSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum ReportsSubcommand {
    #[command(about = "Initialize the shared reports scaffold")]
    Init(ReportsInitArgs),
    #[command(about = "Run the shared smoke-report fixture harness")]
    Smoke(ReportsSmokeArgs),
    #[command(about = "Summarize latest report entrypoints from the catalog")]
    Index(ReportsIndexArgs),
    #[command(about = "Verify required report evidence declared in the catalog")]
    Verify(ReportsVerifyArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsInitArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsSmokeArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) fixture: PathBuf,
    #[arg(long)]
    pub(crate) vars: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsIndexArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long, default_value = REPORT_CATALOG_RELATIVE_PATH)]
    pub(crate) catalog: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsVerifyArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long, default_value = REPORT_CATALOG_RELATIVE_PATH)]
    pub(crate) catalog: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportCatalogArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportRenderManyArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) id: String,
    #[arg(long)]
    pub(crate) glob: String,
    #[arg(long, conflicts_with = "template_family")]
    pub(crate) template: Option<String>,
    #[arg(long = "template-family", conflicts_with = "template")]
    pub(crate) template_family: Option<String>,
    #[arg(long = "output-dir")]
    pub(crate) output_dir: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

pub(crate) fn run_reports_init(args: &ReportsInitArgs) -> Result<i32, CommandError> {
    let result = init_report_scaffold(&args.root)?;
    if args.json {
        let payload = serde_json::json!({
            "workspace_root": to_forward_slash(&result.workspace_root),
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

pub(crate) fn run_reports_smoke(
    args: &ReportsSmokeArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let result = run_smoke_report(&args.root, &args.fixture, &args.vars, observer)?;
    if args.json {
        let payload = serde_json::json!({
            "entrypoint": to_forward_slash(&result.entrypoint),
            "metadata": to_forward_slash(&result.metadata),
            "artifacts": result.artifacts.iter().map(|path| to_forward_slash(path)).collect::<Vec<_>>(),
        });
        print_json(payload, result.warnings).map_err(CommandError::usage)?;
    } else {
        println!("entrypoint: {}", result.entrypoint.display());
        println!("metadata: {}", result.metadata.display());
        for artifact in &result.artifacts {
            println!("artifact: {}", artifact.display());
        }
        if !result.warnings.is_empty() {
            print_diagnostic_messages(&result.warnings);
        }
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_reports_index(args: &ReportsIndexArgs) -> Result<i32, CommandError> {
    let payload = serde_json::json!({
        "status": "reserved",
        "subcommand": "reports index",
        "root": to_forward_slash(&args.root),
        "catalog": to_forward_slash(&args.catalog),
        "note": "full aggregation behavior lands in Sprint B5",
    });
    if args.json {
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("reports index reserved");
        println!("root: {}", args.root.display());
        println!("catalog: {}", args.catalog.display());
        println!("note: full aggregation behavior lands in Sprint B5");
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_reports_verify(args: &ReportsVerifyArgs) -> Result<i32, CommandError> {
    let payload = serde_json::json!({
        "status": "reserved",
        "subcommand": "reports verify",
        "root": to_forward_slash(&args.root),
        "catalog": to_forward_slash(&args.catalog),
        "note": "full required-evidence verification lands in Sprint B5",
    });
    if args.json {
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("reports verify reserved");
        println!("root: {}", args.root.display());
        println!("catalog: {}", args.catalog.display());
        println!("note: full required-evidence verification lands in Sprint B5");
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_report_catalog(args: &ReportCatalogArgs) -> Result<i32, CommandError> {
    let catalog = ReportCatalog::load(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    if args.json {
        let payload = serde_json::json!({
            "catalog_path": to_forward_slash(&catalog.catalog_path),
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

pub(crate) fn run_report_render_many(args: &ReportRenderManyArgs) -> Result<i32, CommandError> {
    let template_selector = args.template.clone().unwrap_or_default();
    let request = RenderManyRequest {
        root: args.root.clone(),
        source_set: SourceSetDefinition {
            id: args.id.clone(),
            glob: args.glob.clone(),
            template_selector,
            template_family: args.template_family.clone(),
            output_dir: args.output_dir.clone(),
        },
    };
    let result = render_many(&request).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    if args.json {
        let payload = serde_json::json!({
            "manifest_path": to_forward_slash(&result.manifest_path),
            "output_count": result.generated_outputs.len(),
            "generated_outputs": result.generated_outputs.iter().map(|path| to_forward_slash(path)).collect::<Vec<_>>(),
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
