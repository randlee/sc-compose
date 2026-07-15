use std::path::{Path, PathBuf};

pub(crate) fn to_forward_slash(path: &Path) -> String {
    let path = path.to_string_lossy();
    let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
    path.replace('\\', "/")
}

#[allow(
    clippy::ptr_arg,
    reason = "serde serialize_with passes &PathBuf for PathBuf fields"
)]
pub(crate) fn serialize_path<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&to_forward_slash(path))
}

#[allow(
    clippy::ptr_arg,
    clippy::ref_option,
    reason = "serde serialize_with passes &Option<PathBuf> so std Serialize is skipped"
)]
pub(crate) fn serialize_opt_path<S>(
    path: &Option<PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match path {
        Some(path) => serializer.serialize_some(&to_forward_slash(path)),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn serialize_paths<S>(paths: &[PathBuf], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_seq(paths.iter().map(|path| to_forward_slash(path)))
}

pub(crate) fn is_normalized_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

pub(crate) fn normalize_relative_path(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("path must not be empty".to_owned());
    }
    if path.is_absolute() {
        return Err("path must be relative".to_owned());
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(value) => normalized.push(value),
            std::path::Component::CurDir => {
                return Err("path must not contain '.' segments".to_owned());
            }
            std::path::Component::ParentDir => {
                return Err("path must not contain '..' segments".to_owned());
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("path must be relative".to_owned());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("path must not be empty".to_owned());
    }

    Ok(normalized)
}
