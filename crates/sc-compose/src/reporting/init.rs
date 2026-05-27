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
use crate::path_utils::to_forward_slash;
use crate::reporting::catalog::REPORT_CATALOG_RELATIVE_PATH;
use crate::var_file::load_var_file;

pub(crate) const STARTER_LATEST_RELATIVE_PATH: &str = "reports/latest";
pub(crate) const STARTER_ARCHIVE_RELATIVE_PATH: &str = "reports/archive";
pub(crate) const STARTER_TEMPLATES_RELATIVE_PATH: &str = "reports/templates";
pub(crate) const STARTER_SMOKE_DIR_RELATIVE_PATH: &str = "reports/smoke";
pub(crate) const STARTER_SMOKE_OUTPUT_DIR_RELATIVE_PATH: &str = "reports/latest/smoke";
pub(crate) const STARTER_SMOKE_FIXTURE_RELATIVE_PATH: &str =
    "reports/smoke/reference-template.html.j2";
pub(crate) const STARTER_SMOKE_VARS_RELATIVE_PATH: &str = "reports/smoke/sample-vars.json";
pub(crate) const SMOKE_ENTRYPOINT_RELATIVE_PATH: &str = "reports/latest/smoke/index.html";
pub(crate) const SMOKE_METADATA_RELATIVE_PATH: &str = "reports/latest/smoke/report.json";

const STARTER_REPORTS_TOML: &str = r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#;

const STARTER_SMOKE_TEMPLATE: &str = r#"---
required_variables:
  - title
  - summary
---
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>{{ title }}</title>
</head>
<body>
  <main>
    <h1>{{ title }}</h1>
    <p>{{ summary }}</p>
  </main>
</body>
</html>
"#;

const STARTER_SMOKE_VARS: &str = r#"{
  "title": "Smoke Report",
  "summary": "Shared reporting scaffold smoke test."
}
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportsInitResult {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) workspace_root: PathBuf,
    pub(crate) created_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportsSmokeResult {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) entrypoint: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) metadata: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_paths")]
    pub(crate) artifacts: Vec<PathBuf>,
    pub(crate) warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SmokeMetadata {
    report_id: String,
    kind: String,
    status: String,
    entrypoint: String,
    artifacts: Vec<String>,
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

    let mut created_paths = Vec::new();
    ensure_dir(
        &workspace_root,
        STARTER_LATEST_RELATIVE_PATH,
        &mut created_paths,
    )?;
    ensure_dir(
        &workspace_root,
        STARTER_ARCHIVE_RELATIVE_PATH,
        &mut created_paths,
    )?;
    ensure_dir(
        &workspace_root,
        STARTER_TEMPLATES_RELATIVE_PATH,
        &mut created_paths,
    )?;
    ensure_dir(
        &workspace_root,
        STARTER_SMOKE_DIR_RELATIVE_PATH,
        &mut created_paths,
    )?;
    ensure_dir(
        &workspace_root,
        STARTER_SMOKE_OUTPUT_DIR_RELATIVE_PATH,
        &mut created_paths,
    )?;
    write_if_missing(
        &workspace_root,
        REPORT_CATALOG_RELATIVE_PATH,
        STARTER_REPORTS_TOML,
        &mut created_paths,
    )?;
    write_if_missing(
        &workspace_root,
        STARTER_SMOKE_FIXTURE_RELATIVE_PATH,
        STARTER_SMOKE_TEMPLATE,
        &mut created_paths,
    )?;
    write_if_missing(
        &workspace_root,
        STARTER_SMOKE_VARS_RELATIVE_PATH,
        STARTER_SMOKE_VARS,
        &mut created_paths,
    )?;

    Ok(ReportsInitResult {
        workspace_root,
        created_paths,
    })
}

pub(crate) fn run_smoke_report(
    root: &Path,
    fixture: &Path,
    vars: &Path,
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
    let metadata = workspace_root.join(SMOKE_METADATA_RELATIVE_PATH);
    fs::write(&entrypoint, &result.rendered_text).map_err(|error| {
        CommandError::render_write(
            anyhow!(error).context(format!("failed to write {}", entrypoint.display())),
        )
    })?;

    let artifacts = vec![
        PathBuf::from(SMOKE_ENTRYPOINT_RELATIVE_PATH),
        PathBuf::from(SMOKE_METADATA_RELATIVE_PATH),
    ];
    let metadata_payload = SmokeMetadata {
        report_id: "smoke".to_owned(),
        kind: "smoke".to_owned(),
        status: "pass".to_owned(),
        entrypoint: SMOKE_ENTRYPOINT_RELATIVE_PATH.to_owned(),
        artifacts: artifacts
            .iter()
            .map(|path| to_forward_slash(path))
            .collect(),
    };
    let metadata_json = serde_json::to_string_pretty(&metadata_payload).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("failed to serialize smoke metadata"),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    fs::write(&metadata, metadata_json).map_err(|error| {
        CommandError::render_write(
            anyhow!(error).context(format!("failed to write {}", metadata.display())),
        )
    })?;

    Ok(ReportsSmokeResult {
        entrypoint: PathBuf::from(SMOKE_ENTRYPOINT_RELATIVE_PATH),
        metadata: PathBuf::from(SMOKE_METADATA_RELATIVE_PATH),
        artifacts,
        warnings: result.warnings,
    })
}

fn ensure_dir(
    workspace_root: &Path,
    relative: &str,
    created_paths: &mut Vec<String>,
) -> Result<(), CommandError> {
    let path = workspace_root.join(relative);
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!("failed to create {}", path.display())),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
        created_paths.push(format!("{relative}/"));
    }
    Ok(())
}

fn write_if_missing(
    workspace_root: &Path,
    relative: &str,
    contents: &str,
    created_paths: &mut Vec<String>,
) -> Result<(), CommandError> {
    let path = workspace_root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!("failed to create {}", parent.display())),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
    }
    if !path.exists() {
        fs::write(&path, contents).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!("failed to write {}", path.display())),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
        created_paths.push(relative.to_owned());
    }
    Ok(())
}

fn resolve_relative_path(workspace_root: &Path, path: &Path) -> Result<PathBuf, CommandError> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    fs::canonicalize(&joined).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!("failed to resolve {}", joined.display())),
            DiagnosticCode::ErrConfigParse,
        )
    })
}
