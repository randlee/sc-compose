use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{
    ComposeMode, ComposePolicy, ComposeRequest, CompositionObserver, ConfiningRoot, Diagnostic,
    DiagnosticCode, compose_with_observer,
};
use serde::Serialize;

use crate::CommandError;
use crate::reporting::output::{
    MaterializedReport, ReportOutputRequest, write_report_metadata_and_archive,
};
use crate::reporting::path::resolve_relative_path;
use crate::reporting::scaffold::{
    SMOKE_ENTRYPOINT_RELATIVE_PATH, SMOKE_METADATA_RELATIVE_PATH, write_report_scaffold,
};
use crate::var_file::load_var_file;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportsInitResult {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) workspace_root: PathBuf,
    pub(crate) created_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportsSmokeResult {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) produced_at: String,
    pub(crate) status: String,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) entrypoint: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) metadata: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_paths")]
    pub(crate) artifacts: Vec<PathBuf>,
    #[serde(serialize_with = "crate::path_utils::serialize_paths")]
    pub(crate) archived_artifacts: Vec<PathBuf>,
    #[serde(skip_serializing)]
    pub(crate) warnings: Vec<Diagnostic>,
}

pub(crate) fn init_report_scaffold(root: &Path) -> Result<ReportsInitResult, CommandError> {
    let workspace_root = fs::canonicalize(root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize workspace root {}",
                root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;

    let created_paths = write_report_scaffold(&workspace_root)?;
    Ok(ReportsInitResult {
        workspace_root,
        created_paths,
    })
}

pub(crate) fn run_smoke_report(
    root: &Path,
    fixture: &Path,
    vars: &Path,
    archive: bool,
    observer: &mut dyn CompositionObserver,
) -> Result<ReportsSmokeResult, CommandError> {
    let workspace_root = fs::canonicalize(root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize workspace root {}",
                root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let fixture_path = resolve_relative_path(&workspace_root, fixture)?;
    let vars_path = resolve_relative_path(&workspace_root, vars)?;
    let template_path = fixture_path
        .strip_prefix(&workspace_root)
        .map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!(
                    "smoke fixture {} must remain under workspace root {}",
                    fixture_path.display(),
                    workspace_root.display()
                )),
                DiagnosticCode::ErrConfigParse,
            )
        })?;

    let request = ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: template_path.to_path_buf(),
        },
        root: ConfiningRoot::new(&workspace_root).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!(
                    "failed to canonicalize workspace root {}",
                    workspace_root.display()
                )),
                DiagnosticCode::ErrConfigParse,
            )
        })?,
        vars_input: load_var_file(&vars_path)?,
        vars_env: BTreeMap::new(),
        vars_defaults: BTreeMap::new(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    };

    let result = compose_with_observer(&request, observer).map_err(CommandError::compose)?;
    let entrypoint = workspace_root.join(SMOKE_ENTRYPOINT_RELATIVE_PATH);
    fs::write(&entrypoint, &result.rendered_text).map_err(|error| {
        CommandError::render_write(
            anyhow!(error).context(format!("failed to write {}", entrypoint.display())),
        )
    })?;

    let artifacts = vec![PathBuf::from(SMOKE_ENTRYPOINT_RELATIVE_PATH)];
    let materialized = write_smoke_outputs(&workspace_root, archive, artifacts)?;

    Ok(ReportsSmokeResult {
        report_id: materialized.report_id,
        kind: materialized.kind,
        produced_at: materialized.produced_at,
        status: materialized.status,
        entrypoint: materialized.entrypoint,
        metadata: materialized.metadata,
        artifacts: materialized.latest_artifacts,
        archived_artifacts: materialized.archived_artifacts,
        warnings: result.warnings,
    })
}

fn write_smoke_outputs(
    workspace_root: &Path,
    archive: bool,
    latest_artifacts: Vec<PathBuf>,
) -> Result<MaterializedReport, CommandError> {
    write_report_metadata_and_archive(
        workspace_root,
        &ReportOutputRequest {
            report_id: "smoke".to_owned(),
            kind: "smoke".to_owned(),
            status: "pass".to_owned(),
            entrypoint: PathBuf::from(SMOKE_ENTRYPOINT_RELATIVE_PATH),
            metadata_path: PathBuf::from(SMOKE_METADATA_RELATIVE_PATH),
            latest_artifacts,
            archive,
        },
    )
    .map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("failed to materialize smoke report outputs"),
            DiagnosticCode::ErrConfigParse,
        )
    })
}
