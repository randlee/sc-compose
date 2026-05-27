use std::path::PathBuf;

use anyhow::anyhow;
use clap::{Args, Subcommand};
use sc_composer::{CompositionObserver, DiagnosticCode};

use crate::exit_codes;
use crate::path_utils::to_forward_slash;
use crate::reporting::catalog::ReportCatalog;
use crate::reporting::index::{build_report_index, verify_required_reports};
use crate::reporting::init::{init_report_scaffold, run_smoke_report};
use crate::reporting::output::{FinalizeReportRequest, finalize_report_outputs};
use crate::reporting::publish_manifest::write_publish_manifest;
use crate::reporting::render_many::{RenderManyRequest, SourceSetDefinition, render_many};
use crate::reporting::spec::run_render_spec_report;
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
    pub(crate) archive: bool,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsFinalizeArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long = "report-id")]
    pub(crate) report_id: String,
    #[arg(long)]
    pub(crate) kind: String,
    #[arg(long, default_value = "pass")]
    pub(crate) status: String,
    #[arg(long)]
    pub(crate) entrypoint: PathBuf,
    #[arg(long = "artifact", action = clap::ArgAction::Append)]
    pub(crate) artifacts: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) archive: bool,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsRenderSpecArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long = "spec")]
    pub(crate) spec_path: PathBuf,
    #[arg(long)]
    pub(crate) archive: bool,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsIndexArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsVerifyArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReportsPublishManifestArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
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
        let payload = serde_json::json!({
            "report_id": result.report_id,
            "kind": result.kind,
            "produced_at": result.produced_at,
            "status": result.status,
            "entrypoint": to_forward_slash(&result.entrypoint),
            "metadata": to_forward_slash(&result.metadata),
            "artifacts": result.latest_artifacts.iter().map(|path| to_forward_slash(path)).collect::<Vec<_>>(),
            "archived_artifacts": result.archived_artifacts.iter().map(|path| to_forward_slash(path)).collect::<Vec<_>>(),
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

pub(crate) fn run_reports_render_spec(
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

pub(crate) fn run_reports_publish_manifest(
    args: &ReportsPublishManifestArgs,
) -> Result<i32, CommandError> {
    let result = write_publish_manifest(&args.root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })?;
    if args.json {
        let payload = serde_json::json!({
            "manifest_path": to_forward_slash(&result.manifest_path),
            "report_count": result.report_count,
            "manifest": result.manifest,
        });
        print_json(payload, Vec::new()).map_err(CommandError::usage)?;
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
