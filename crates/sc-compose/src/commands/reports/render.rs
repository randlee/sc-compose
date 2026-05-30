use anyhow::anyhow;
use sc_composer::DiagnosticCode;

use crate::commands::reports::{ReportCatalogArgs, ReportRenderManyArgs, ReportsRenderSpecArgs};
use crate::exit_codes;
use crate::path_utils::to_forward_slash;
use crate::reporting::catalog::ReportCatalog;
use crate::reporting::render_many::{RenderManyRequest, SourceSetDefinition, render_many};
use crate::reporting::spec::run_render_spec_report;
use crate::{CommandError, print_json};

pub(crate) fn run_reports_render_spec(args: &ReportsRenderSpecArgs) -> Result<i32, CommandError> {
    let result = run_render_spec_report(&args.root, &args.spec_path, args.archive)?;
    if args.json {
        let payload = serde_json::json!({
            "report_id": result.report_id,
            "kind": result.kind,
            "produced_at": result.produced_at,
            "status": result.status,
            "entrypoint": to_forward_slash(&result.entrypoint),
            "metadata": to_forward_slash(&result.metadata),
            "artifacts": result.artifacts.iter().map(|path| to_forward_slash(path)).collect::<Vec<_>>(),
            "archived_artifacts": result.archived_artifacts.iter().map(|path| to_forward_slash(path)).collect::<Vec<_>>(),
        });
        print_json(payload, result.warnings).map_err(CommandError::usage)?;
    } else {
        println!("report_id: {}", result.report_id);
        println!("kind: {}", result.kind);
        println!("produced_at: {}", result.produced_at);
        println!("status: {}", result.status);
        println!("entrypoint: {}", to_forward_slash(&result.entrypoint));
        println!("metadata: {}", to_forward_slash(&result.metadata));
        for artifact in &result.artifacts {
            println!("artifact: {}", to_forward_slash(artifact));
        }
        for artifact in &result.archived_artifacts {
            println!("archived: {}", to_forward_slash(artifact));
        }
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
        println!("catalog: {}", to_forward_slash(&catalog.catalog_path));
        println!("reports: {}", catalog.reports.len());
        for report in &catalog.reports {
            println!(
                "{} kind={} producer={} required={} entrypoint={} metadata={}",
                report.id,
                report.kind,
                report.producer,
                report.required,
                to_forward_slash(&report.entrypoint),
                to_forward_slash(&report.metadata)
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
        println!("manifest: {}", to_forward_slash(&result.manifest_path));
        println!("outputs: {}", result.generated_outputs.len());
        for output in &result.generated_outputs {
            println!("generated: {}", to_forward_slash(output));
        }
    }

    Ok(exit_codes::SUCCESS)
}
