use std::path::PathBuf;

use clap::{Args, Subcommand};

pub(crate) mod publish;
pub(crate) mod render;
pub(crate) mod scaffold;

pub(crate) use publish::{run_reports_finalize, run_reports_publish_manifest};
pub(crate) use render::{run_report_catalog, run_report_render_many, run_reports_render_spec};
pub(crate) use scaffold::{
    run_reports_index, run_reports_init, run_reports_smoke, run_reports_verify,
};

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
