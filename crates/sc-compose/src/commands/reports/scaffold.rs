use anyhow::anyhow;
use sc_composer::{CompositionObserver, DiagnosticCode};

use crate::commands::reports::{
    ReportsIndexArgs, ReportsInitArgs, ReportsSmokeArgs, ReportsVerifyArgs,
};
use crate::exit_codes;
use crate::path_utils::to_forward_slash;
use crate::reporting::index::{build_report_index, verify_required_reports};
use crate::reporting::init::{init_report_scaffold, run_smoke_report};
use crate::{CommandError, print_diagnostic_messages, print_json};

pub(crate) fn run_reports_init(args: &ReportsInitArgs) -> Result<i32, CommandError> {
    let result = init_report_scaffold(&args.root)?;
    if args.json {
        let payload = serde_json::json!({
            "workspace_root": to_forward_slash(&result.workspace_root),
            "created_paths": result.created_paths,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!(
            "workspace_root: {}",
            to_forward_slash(&result.workspace_root)
        );
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
        if !result.warnings.is_empty() {
            print_diagnostic_messages(&result.warnings);
        }
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_reports_index(args: &ReportsIndexArgs) -> Result<i32, CommandError> {
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
                to_forward_slash(&entry.entrypoint),
                to_forward_slash(&entry.metadata)
            );
            if !entry.missing_paths.is_empty() {
                for missing in &entry.missing_paths {
                    println!("missing: {}", to_forward_slash(missing));
                }
            }
        }
    }
    Ok(exit_codes::SUCCESS)
}

pub(crate) fn run_reports_verify(args: &ReportsVerifyArgs) -> Result<i32, CommandError> {
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
