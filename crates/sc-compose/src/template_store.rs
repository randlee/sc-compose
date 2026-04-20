use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use sc_composer::{InputValue, VariableName, validate_input_value};
use serde::Deserialize;

const TEMPLATE_ROOT_README: &str = "# sc-compose templates\n\n\
Add one directory per user template under this root.\n\
\n\
- `sc-compose templates list` discovers template directory names here.\n\
- `sc-compose templates <name>` renders the single root-level `*.j2` file in the pack.\n\
- `sc-compose templates add <src> [name]` imports a file or directory as one pack.\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StoreKind {
    Examples,
    Templates,
}

#[derive(Clone, Debug)]
pub(crate) struct TemplateStore {
    pub(crate) source_dir: PathBuf,
    kind: StoreKind,
}

#[derive(Clone, Debug)]
pub(crate) struct TemplateMeta {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) description: Option<String>,
    pub(crate) version: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct TemplatePack {
    pub(crate) root: PathBuf,
    pub(crate) template_path: PathBuf,
    pub(crate) input_defaults: BTreeMap<VariableName, InputValue>,
}

#[derive(Clone, Debug)]
pub(crate) struct TemplateAddResult {
    pub(crate) name: String,
    pub(crate) source: PathBuf,
    pub(crate) destination: PathBuf,
    pub(crate) changed: bool,
}

#[derive(Debug)]
pub(crate) enum AddError {
    AlreadyExists { destination: PathBuf },
    Other(anyhow::Error),
}

#[derive(Debug)]
pub(crate) enum GetTemplateError {
    Parse(anyhow::Error),
    NotRenderable(anyhow::Error),
}

impl std::fmt::Display for AddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists { destination } => {
                write!(
                    f,
                    "template pack already exists at {}",
                    destination.display()
                )
            }
            Self::Other(error) => write!(f, "{error:#}"),
        }
    }
}

impl std::error::Error for AddError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyExists { .. } => None,
            Self::Other(error) => Some(error.as_ref()),
        }
    }
}

impl std::fmt::Display for GetTemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) | Self::NotRenderable(error) => write!(f, "{error:#}"),
        }
    }
}

impl std::error::Error for GetTemplateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) | Self::NotRenderable(error) => Some(error.as_ref()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TemplateManifest {
    description: Option<String>,
    version: Option<String>,
    #[serde(default)]
    input_defaults: BTreeMap<String, serde_json::Value>,
}

impl TemplateStore {
    pub(crate) fn examples() -> Result<Self> {
        Ok(Self {
            source_dir: data_dir()?.join("examples"),
            kind: StoreKind::Examples,
        })
    }

    pub(crate) fn templates() -> Result<Self> {
        Ok(Self {
            source_dir: user_templates_dir()?,
            kind: StoreKind::Templates,
        })
    }

    pub(crate) fn ensure_templates_root(&self) -> Result<()> {
        if self.kind != StoreKind::Templates {
            return Ok(());
        }

        fs::create_dir_all(&self.source_dir).with_context(|| {
            format!(
                "failed to create user template root {}",
                self.source_dir.display()
            )
        })?;
        let readme = self.source_dir.join("README.md");
        if !readme.exists() {
            fs::write(&readme, TEMPLATE_ROOT_README)
                .with_context(|| format!("failed to write {}", readme.display()))?;
        }
        Ok(())
    }

    pub(crate) fn list(&self) -> Result<Vec<TemplateMeta>> {
        if !self.source_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packs = match self.kind {
            StoreKind::Examples => self.list_examples()?,
            StoreKind::Templates => self.list_templates()?,
        };
        packs.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(packs)
    }

    pub(crate) fn get_example(&self, name: &str) -> Result<Option<TemplatePack>> {
        assert!(
            self.kind == StoreKind::Examples,
            "TemplateStore::get_example requires StoreKind::Examples"
        );
        if !self.source_dir.exists() {
            return Ok(None);
        }

        let entries = self.discover_example_entries()?;
        match entries.get(name).map(Vec::as_slice) {
            None => Ok(None),
            Some([path]) => {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| anyhow!("missing example filename for {}", path.display()))?;
                Ok(Some(TemplatePack {
                    root: self.source_dir.clone(),
                    template_path: PathBuf::from(file_name),
                    input_defaults: BTreeMap::default(),
                }))
            }
            Some(paths) => Err(example_name_collision_error(name, paths)),
        }
    }

    pub(crate) fn get_template(
        &self,
        name: &str,
    ) -> std::result::Result<Option<TemplatePack>, GetTemplateError> {
        assert!(
            self.kind == StoreKind::Templates,
            "TemplateStore::get_template requires StoreKind::Templates"
        );
        self.find_template_dir(name)
            .map(|path| Self::load_template_pack(&path))
            .transpose()
    }

    pub(crate) fn add(
        &self,
        source: &Path,
        requested_name: Option<&str>,
    ) -> std::result::Result<TemplateAddResult, AddError> {
        assert!(
            self.kind == StoreKind::Templates,
            "TemplateStore::add requires StoreKind::Templates"
        );
        self.ensure_templates_root().map_err(AddError::Other)?;

        let source = fs::canonicalize(source)
            .with_context(|| format!("failed to resolve template source {}", source.display()))
            .map_err(AddError::Other)?;
        let pack_name = requested_name.map_or_else(
            || default_pack_name(&source),
            std::borrow::ToOwned::to_owned,
        );
        let destination = self.source_dir.join(&pack_name);
        if destination.exists() {
            return Err(AddError::AlreadyExists { destination });
        }

        if source.is_dir() {
            copy_directory_recursive(&source, &destination).map_err(AddError::Other)?;
        } else {
            fs::create_dir_all(&destination)
                .with_context(|| {
                    format!("failed to create template pack {}", destination.display())
                })
                .map_err(AddError::Other)?;
            let file_name = source
                .file_name()
                .ok_or_else(|| anyhow!("missing source filename for {}", source.display()))
                .map_err(AddError::Other)?;
            fs::copy(&source, destination.join(file_name))
                .with_context(|| {
                    format!(
                        "failed to copy {} into {}",
                        source.display(),
                        destination.display()
                    )
                })
                .map_err(AddError::Other)?;
        }

        Ok(TemplateAddResult {
            name: pack_name,
            source,
            destination,
            changed: true,
        })
    }

    fn list_examples(&self) -> Result<Vec<TemplateMeta>> {
        let entries = self.discover_example_entries()?;
        let mut packs = Vec::new();
        for (name, paths) in entries {
            match paths.as_slice() {
                [path] => packs.push(TemplateMeta {
                    name,
                    path: path.clone(),
                    description: None,
                    version: None,
                }),
                _ => return Err(example_name_collision_error(&name, &paths)),
            }
        }
        Ok(packs)
    }

    fn discover_example_entries(&self) -> Result<BTreeMap<String, Vec<PathBuf>>> {
        let mut entries = BTreeMap::<String, Vec<PathBuf>>::new();
        for entry in fs::read_dir(&self.source_dir)
            .with_context(|| format!("failed to read {}", self.source_dir.display()))?
        {
            let entry = entry
                .with_context(|| format!("failed to enumerate {}", self.source_dir.display()))?;
            let path = entry.path();
            if !path.is_file() || !is_j2_template(&path) {
                continue;
            }
            if let Some(name) = example_name_for_path(&path) {
                entries.entry(name).or_default().push(path);
            }
        }
        Ok(entries)
    }

    fn list_templates(&self) -> Result<Vec<TemplateMeta>> {
        let mut packs = Vec::new();
        for entry in fs::read_dir(&self.source_dir)
            .with_context(|| format!("failed to read {}", self.source_dir.display()))?
        {
            let entry = entry
                .with_context(|| format!("failed to enumerate {}", self.source_dir.display()))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| anyhow!("invalid template directory name {}", path.display()))?
                .to_owned();
            let manifest = load_manifest(&path)?;
            packs.push(TemplateMeta {
                name,
                path,
                description: manifest
                    .as_ref()
                    .and_then(|manifest| manifest.description.clone()),
                version: manifest
                    .as_ref()
                    .and_then(|manifest| manifest.version.clone()),
            });
        }
        Ok(packs)
    }

    fn find_template_dir(&self, name: &str) -> Option<PathBuf> {
        if !self.source_dir.exists() {
            return None;
        }
        let candidate = self.source_dir.join(name);
        candidate.is_dir().then_some(candidate)
    }

    fn load_template_pack(path: &Path) -> std::result::Result<TemplatePack, GetTemplateError> {
        let manifest = load_manifest(path).map_err(GetTemplateError::Parse)?;
        let input_defaults = manifest
            .map(|manifest| validate_manifest_defaults(path, manifest.input_defaults))
            .transpose()
            .map_err(GetTemplateError::Parse)?
            .unwrap_or_default();
        let template_path = resolve_template_entrypoint(path)?;
        Ok(TemplatePack {
            root: path.to_path_buf(),
            template_path,
            input_defaults,
        })
    }
}

pub(crate) fn data_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SC_COMPOSE_DATA_DIR") {
        return absolute_path(PathBuf::from(path));
    }

    let executable = std::env::current_exe().context("failed to determine executable path")?;
    let executable_dir = executable
        .parent()
        .ok_or_else(|| anyhow!("failed to determine executable directory"))?;
    Ok(executable_dir.join("..").join("share").join("sc-compose"))
}

pub(crate) fn user_templates_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SC_COMPOSE_TEMPLATE_DIR") {
        return absolute_path(PathBuf::from(path));
    }

    let data_dir = platform_user_data_dir()
        .ok_or_else(|| anyhow!("failed to determine the platform user-data directory"))?;
    Ok(data_dir.join("sc-compose").join("templates"))
}

fn example_name_for_path(path: &Path) -> Option<String> {
    if !is_j2_template(path) {
        return None;
    }

    let without_j2 = path.file_stem()?.to_str()?;
    let stem = Path::new(without_j2)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(without_j2);
    Some(stem.to_owned())
}

fn default_pack_name(source: &Path) -> String {
    if source.is_dir() {
        return source
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("template")
            .to_owned();
    }

    example_name_for_path(source).unwrap_or_else(|| {
        source
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("template")
            .to_owned()
    })
}

fn example_name_collision_error(name: &str, paths: &[PathBuf]) -> anyhow::Error {
    anyhow!(
        "example pack name `{name}` is ambiguous between {}",
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn load_manifest(path: &Path) -> Result<Option<TemplateManifest>> {
    let manifest_path = path.join("template.json");
    if !manifest_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    serde_json::from_str::<TemplateManifest>(&contents)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))
        .map(Some)
}

fn validate_manifest_defaults(
    path: &Path,
    input_defaults: BTreeMap<String, serde_json::Value>,
) -> Result<BTreeMap<VariableName, InputValue>> {
    let mut values = BTreeMap::default();
    for (key, value) in input_defaults {
        validate_input_value(&value).with_context(|| {
            format!(
                "invalid template.json input_defaults value for `{key}` in {}",
                path.display()
            )
        })?;
        values.insert(
            VariableName::new(key.clone()).map_err(|error| {
                anyhow!("invalid template.json input_defaults key `{key}`: {error}")
            })?,
            value,
        );
    }
    Ok(values)
}

fn resolve_template_entrypoint(path: &Path) -> std::result::Result<PathBuf, GetTemplateError> {
    let mut templates = Vec::new();
    for entry in fs::read_dir(path)
        .with_context(|| format!("failed to read {}", path.display()))
        .map_err(GetTemplateError::Parse)?
    {
        let entry = entry
            .with_context(|| format!("failed to enumerate {}", path.display()))
            .map_err(GetTemplateError::Parse)?;
        let entry_path = entry.path();
        if entry_path.is_file() && is_j2_template(&entry_path) {
            templates.push(
                entry_path
                    .file_name()
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow!("missing template filename in {}", path.display()))
                    .map_err(GetTemplateError::Parse)?,
            );
        }
    }

    match templates.len() {
        1 => Ok(templates.remove(0)),
        0 => Err(GetTemplateError::NotRenderable(anyhow!(
            "template pack {} is not renderable because it has no root-level `*.j2` file",
            path.display()
        ))),
        _ => Err(GetTemplateError::NotRenderable(anyhow!(
            "template pack {} is not renderable because it has multiple root-level `*.j2` files",
            path.display()
        ))),
    }
}

fn is_j2_template(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("j2"))
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry.with_context(|| format!("failed to enumerate {}", source.display()))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy {} into {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn platform_user_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("LOCALAPPDATA"))
            .map(PathBuf::from)
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support"))
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".local").join("share"))
            })
    }
}

fn absolute_path(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()
            .context("failed to determine the current directory")?
            .join(path))
    }
}
