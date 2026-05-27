use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::{Args, Subcommand};
use sc_composer::DiagnosticCode;

use crate::exit_codes;
use crate::reporting::catalog::{
    REPORT_CATALOG_RELATIVE_PATH, REPORT_METADATA_BASENAME, REPORTS_ARCHIVE_ROOT_RELATIVE_PATH,
    REPORTS_LATEST_ROOT_RELATIVE_PATH, ReportCatalog,
};
use crate::{CommandError, print_json};

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

pub(crate) fn run_reports_init(args: &ReportsInitArgs) -> Result<i32, CommandError> {
    let payload = serde_json::json!({
        "status": "reserved",
        "subcommand": "reports init",
        "root": args.root.display().to_string(),
        "note": "shared scaffold creation lands in Sprint B2",
    });
    if args.json {
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("reports init reserved");
        println!("root: {}", args.root.display());
        println!("note: shared scaffold creation lands in Sprint B2");
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_reports_smoke(args: &ReportsSmokeArgs) -> Result<i32, CommandError> {
    let payload = serde_json::json!({
        "status": "reserved",
        "subcommand": "reports smoke",
        "root": args.root.display().to_string(),
        "note": "shared smoke harness behavior lands in Sprint B2",
    });
    if args.json {
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("reports smoke reserved");
        println!("root: {}", args.root.display());
        println!("note: shared smoke harness behavior lands in Sprint B2");
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_reports_index(args: &ReportsIndexArgs) -> Result<i32, CommandError> {
    run_catalog_summary(&args.root, args.json)
}

pub(crate) fn run_reports_verify(args: &ReportsVerifyArgs) -> Result<i32, CommandError> {
    let catalog = ReportCatalog::load(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    let required_reports = catalog
        .reports
        .iter()
        .filter(|report| report.required)
        .count();
    let payload = serde_json::json!({
        "catalog_path": catalog.catalog_path.display().to_string(),
        "report_count": catalog.reports.len(),
        "required_reports": required_reports,
        "latest_root": REPORTS_LATEST_ROOT_RELATIVE_PATH,
        "archive_root": REPORTS_ARCHIVE_ROOT_RELATIVE_PATH,
        "metadata_sidecar_basename": REPORT_METADATA_BASENAME,
        "status": "validated",
    });
    if args.json {
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("catalog: {}", catalog.catalog_path.display());
        println!("reports: {}", catalog.reports.len());
        println!("required_reports: {required_reports}");
        println!("latest_root: {REPORTS_LATEST_ROOT_RELATIVE_PATH}");
        println!("archive_root: {REPORTS_ARCHIVE_ROOT_RELATIVE_PATH}");
        println!("metadata_sidecar_basename: {REPORT_METADATA_BASENAME}");
        println!("status: validated");
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_report_catalog(args: &ReportCatalogArgs) -> Result<i32, CommandError> {
    run_catalog_summary(&args.root, args.json)
}

fn run_catalog_summary(root: &Path, json: bool) -> Result<i32, CommandError> {
    let catalog = ReportCatalog::load(root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    if json {
        let payload = serde_json::json!({
            "catalog_path": catalog.catalog_path.display().to_string(),
            "report_count": catalog.reports.len(),
            "latest_root": REPORTS_LATEST_ROOT_RELATIVE_PATH,
            "archive_root": REPORTS_ARCHIVE_ROOT_RELATIVE_PATH,
            "metadata_sidecar_basename": REPORT_METADATA_BASENAME,
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
