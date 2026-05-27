use std::path::PathBuf;

use anyhow::anyhow;
use clap::Args;
use sc_composer::DiagnosticCode;

use crate::CommandError;
use crate::reporting::catalog::ReportCatalog;

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportCatalogArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

pub(crate) fn run_report_catalog(args: &ReportCatalogArgs) -> Result<i32, CommandError> {
    let catalog = ReportCatalog::load(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;

    if args.json {
        let payload = serde_json::json!({
            "catalog_path": catalog.catalog_path.display().to_string(),
            "report_count": catalog.reports.len(),
            "reports": catalog.reports,
        });
        crate::print_json(payload, Vec::new()).map_err(CommandError::usage)?;
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

    Ok(crate::exit_codes::SUCCESS)
}
