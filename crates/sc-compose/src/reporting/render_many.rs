use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use glob::glob;
use sc_composer::{LoadedTemplateRequest, RenderError, RenderedArtifact, render_loaded_template};
use serde::Serialize;
use serde_json::Value;

use crate::reporting::source_entry::{SourceEntry, SourceEntryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceSetDefinition {
    pub(crate) id: String,
    pub(crate) glob: String,
    pub(crate) template_path: PathBuf,
    pub(crate) output_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderManyRequest {
    pub(crate) root: PathBuf,
    pub(crate) source_set: SourceSetDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RenderManyManifestEntry {
    pub(crate) source_path: PathBuf,
    pub(crate) output_path: PathBuf,
    pub(crate) metadata: BTreeMap<String, Value>,
    pub(crate) sets: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RenderManyResult {
    pub(crate) manifest_path: PathBuf,
    pub(crate) entries: Vec<RenderManyManifestEntry>,
    pub(crate) generated_outputs: Vec<PathBuf>,
}

#[derive(Debug)]
pub(crate) enum RenderManyError {
    InvalidGlob {
        glob: String,
        message: String,
    },
    GlobWalk {
        glob: String,
        message: String,
    },
    ReadTemplate {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidTemplatePath {
        path: PathBuf,
        message: String,
    },
    CreateOutputDir {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteOutput {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteManifest {
        path: PathBuf,
        source: std::io::Error,
    },
    SerializeManifest {
        source: serde_json::Error,
    },
    Render {
        source_path: PathBuf,
        source: RenderError,
    },
    SourceEntry(SourceEntryError),
}

impl RenderManyRequest {
    pub(crate) fn output_extension(&self) -> Result<String, RenderManyError> {
        let file_name = self
            .source_set
            .template_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| RenderManyError::InvalidTemplatePath {
                path: self.source_set.template_path.clone(),
                message: "template path must have a valid UTF-8 filename".to_owned(),
            })?;
        let stripped =
            file_name
                .strip_suffix(".j2")
                .ok_or_else(|| RenderManyError::InvalidTemplatePath {
                    path: self.source_set.template_path.clone(),
                    message: "template path must end with .j2".to_owned(),
                })?;
        let extension = Path::new(stripped)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("out");
        Ok(extension.to_owned())
    }
}

pub(crate) fn render_many(
    request: &RenderManyRequest,
) -> Result<RenderManyResult, RenderManyError> {
    let template_path = request.root.join(&request.source_set.template_path);
    let template_text = std::fs::read_to_string(&template_path).map_err(|source| {
        RenderManyError::ReadTemplate {
            path: template_path.clone(),
            source,
        }
    })?;
    let output_root = request.root.join(&request.source_set.output_dir);
    std::fs::create_dir_all(&output_root).map_err(|source| RenderManyError::CreateOutputDir {
        path: output_root.clone(),
        source,
    })?;

    let mut discovered = discover_sources(request)?;
    discovered.sort_by(|left, right| left.source_path.cmp(&right.source_path));

    let mut entries = Vec::with_capacity(discovered.len());
    let mut generated_outputs = Vec::with_capacity(discovered.len());
    for entry in discovered {
        let rendered =
            render_entry(&entry, &request.source_set.id, &template_text).map_err(|source| {
                RenderManyError::Render {
                    source_path: entry.source_path.clone(),
                    source,
                }
            })?;
        let absolute_output = request.root.join(&entry.output_path);
        if let Some(parent) = absolute_output.parent() {
            std::fs::create_dir_all(parent).map_err(|source| RenderManyError::CreateOutputDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&absolute_output, rendered.rendered).map_err(|source| {
            RenderManyError::WriteOutput {
                path: absolute_output.clone(),
                source,
            }
        })?;
        generated_outputs.push(entry.output_path.clone());
        entries.push(RenderManyManifestEntry {
            source_path: entry.source_path.clone(),
            output_path: entry.output_path.clone(),
            metadata: entry.metadata.clone(),
            sets: entry.sets.clone(),
        });
    }

    let manifest_path = request.source_set.output_dir.join("manifest.json");
    let manifest_bytes = serde_json::to_vec_pretty(&entries)
        .map_err(|source| RenderManyError::SerializeManifest { source })?;
    std::fs::write(request.root.join(&manifest_path), manifest_bytes).map_err(|source| {
        RenderManyError::WriteManifest {
            path: request.root.join(&manifest_path),
            source,
        }
    })?;
    generated_outputs.push(manifest_path.clone());

    Ok(RenderManyResult {
        manifest_path,
        entries,
        generated_outputs,
    })
}

fn discover_sources(request: &RenderManyRequest) -> Result<Vec<SourceEntry>, RenderManyError> {
    let pattern = request
        .root
        .join(&request.source_set.glob)
        .display()
        .to_string();
    let paths = glob(&pattern).map_err(|error| RenderManyError::InvalidGlob {
        glob: request.source_set.glob.clone(),
        message: error.to_string(),
    })?;
    let output_extension = request.output_extension()?;

    let mut entries = Vec::new();
    for path in paths {
        let absolute_source = path.map_err(|error| RenderManyError::GlobWalk {
            glob: request.source_set.glob.clone(),
            message: error.to_string(),
        })?;
        let relative_source = absolute_source
            .strip_prefix(&request.root)
            .map_err(|error| RenderManyError::InvalidTemplatePath {
                path: absolute_source.clone(),
                message: error.to_string(),
            })?;
        let output_path = derive_output_path(
            &request.source_set.output_dir,
            relative_source,
            &output_extension,
        );
        entries.push(
            SourceEntry::load(&absolute_source, relative_source, output_path)
                .map_err(RenderManyError::SourceEntry)?,
        );
    }
    Ok(entries)
}

fn derive_output_path(output_dir: &Path, source_path: &Path, extension: &str) -> PathBuf {
    let mut output_path = output_dir.join(source_path);
    output_path.set_extension(extension);
    output_path
}

fn render_entry(
    entry: &SourceEntry,
    template_name: &str,
    template_text: &str,
) -> Result<RenderedArtifact, RenderError> {
    let mut context = BTreeMap::new();
    context.insert(
        "source_path".to_owned(),
        Value::String(entry.source_path.display().to_string()),
    );
    context.insert(
        "output_path".to_owned(),
        Value::String(entry.output_path.display().to_string()),
    );
    context.insert("metadata".to_owned(), serde_json::json!(entry.metadata));
    context.insert(
        "sets".to_owned(),
        entry
            .sets
            .clone()
            .map_or(Value::Null, |sets| serde_json::json!(sets)),
    );
    context.insert(
        "raw_source".to_owned(),
        Value::String(entry.raw_source.clone()),
    );
    context.insert("body".to_owned(), Value::String(entry.body.clone()));

    render_loaded_template(LoadedTemplateRequest {
        template_name: template_name.to_owned(),
        template_text: template_text.to_owned(),
        context,
    })
}

impl fmt::Display for RenderManyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGlob { glob, message } => {
                write!(f, "invalid source glob '{glob}': {message}")
            }
            Self::GlobWalk { glob, message } => {
                write!(f, "failed to walk source glob '{glob}': {message}")
            }
            Self::ReadTemplate { path, source } => {
                write!(
                    f,
                    "failed to read render-many template {}: {source}",
                    path.display()
                )
            }
            Self::InvalidTemplatePath { path, message } => {
                write!(
                    f,
                    "invalid render-many template path {}: {message}",
                    path.display()
                )
            }
            Self::CreateOutputDir { path, source } => {
                write!(
                    f,
                    "failed to create render-many output dir {}: {source}",
                    path.display()
                )
            }
            Self::WriteOutput { path, source } => {
                write!(
                    f,
                    "failed to write render-many output {}: {source}",
                    path.display()
                )
            }
            Self::WriteManifest { path, source } => {
                write!(
                    f,
                    "failed to write render-many manifest {}: {source}",
                    path.display()
                )
            }
            Self::SerializeManifest { source } => {
                write!(f, "failed to serialize render-many manifest: {source}")
            }
            Self::Render {
                source_path,
                source,
            } => {
                write!(
                    f,
                    "failed to render source entry {}: {source}",
                    source_path.display()
                )
            }
            Self::SourceEntry(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for RenderManyError {}
