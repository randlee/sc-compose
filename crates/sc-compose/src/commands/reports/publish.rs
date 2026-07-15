use anyhow::anyhow;
use sc_composer::DiagnosticCode;

use crate::commands::reports::{ReportsFinalizeArgs, ReportsPublishManifestArgs};
use crate::exit_codes;
use crate::path_utils::to_forward_slash;
use crate::reporting::output::{FinalizeReportRequest, finalize_report_outputs};
use crate::reporting::publish_manifest::write_publish_manifest;
use crate::{CommandError, print_json};

pub(crate) fn run_reports_finalize(args: &ReportsFinalizeArgs) -> Result<i32, CommandError> {
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
        print_json(&result, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("report_id: {}", result.report_id);
        println!("kind: {}", result.kind);
        println!("produced_at: {}", result.produced_at);
        println!("status: {}", result.status);
        println!("entrypoint: {}", to_forward_slash(&result.entrypoint));
        println!("metadata: {}", to_forward_slash(&result.metadata));
        for artifact in &result.latest_artifacts {
            println!("artifact: {}", to_forward_slash(artifact));
        }
        for artifact in &result.archived_artifacts {
            println!("archived: {}", to_forward_slash(artifact));
        }
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_reports_publish_manifest(
    args: &ReportsPublishManifestArgs,
) -> Result<i32, CommandError> {
    let result = write_publish_manifest(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;
    if args.json {
        print_json(&result, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!("manifest: {}", to_forward_slash(&result.manifest_path));
        println!("reports: {}", result.report_count);
        for report in &result.manifest.reports {
            println!(
                "{} kind={} entrypoint={}",
                report.report_id,
                report.kind,
                to_forward_slash(&report.entrypoint)
            );
            if let Some(archive_root) = &report.archive_root {
                println!("archive_root: {}", to_forward_slash(archive_root));
            }
            for file in &report.files {
                println!(
                    "file role={} path={} publish_to={}",
                    file.role,
                    to_forward_slash(&file.path),
                    to_forward_slash(&file.publish_to)
                );
            }
        }
    }
    Ok(exit_codes::SUCCESS)
}
