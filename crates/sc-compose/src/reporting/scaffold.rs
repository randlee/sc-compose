use std::fs;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::DiagnosticCode;

use crate::CommandError;
use crate::reporting::catalog::REPORT_CATALOG_RELATIVE_PATH;
use crate::reporting::output::{ARCHIVE_ROOT_RELATIVE_PATH, LATEST_ROOT_RELATIVE_PATH};

const STARTER_TEMPLATES_RELATIVE_PATH: &str = "reports/templates";
const STARTER_SMOKE_DIR_RELATIVE_PATH: &str = "reports/smoke";
const STARTER_SMOKE_OUTPUT_DIR_RELATIVE_PATH: &str = "reports/latest/smoke";
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

struct ScaffoldFile {
    relative_path: &'static str,
    contents: &'static str,
}

pub(crate) fn write_report_scaffold(workspace_root: &Path) -> Result<Vec<String>, CommandError> {
    let mut created_paths = Vec::new();
    for relative_dir in scaffold_directories() {
        ensure_dir(workspace_root, relative_dir, &mut created_paths)?;
    }
    for file in scaffold_files() {
        write_if_missing(
            workspace_root,
            file.relative_path,
            file.contents,
            &mut created_paths,
        )?;
    }
    Ok(created_paths)
}

fn scaffold_directories() -> [&'static str; 5] {
    [
        LATEST_ROOT_RELATIVE_PATH,
        ARCHIVE_ROOT_RELATIVE_PATH,
        STARTER_TEMPLATES_RELATIVE_PATH,
        STARTER_SMOKE_DIR_RELATIVE_PATH,
        STARTER_SMOKE_OUTPUT_DIR_RELATIVE_PATH,
    ]
}

fn scaffold_files() -> [ScaffoldFile; 3] {
    [
        ScaffoldFile {
            relative_path: REPORT_CATALOG_RELATIVE_PATH,
            contents: STARTER_REPORTS_TOML,
        },
        ScaffoldFile {
            relative_path: STARTER_SMOKE_FIXTURE_RELATIVE_PATH,
            contents: STARTER_SMOKE_TEMPLATE,
        },
        ScaffoldFile {
            relative_path: STARTER_SMOKE_VARS_RELATIVE_PATH,
            contents: STARTER_SMOKE_VARS,
        },
    ]
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
