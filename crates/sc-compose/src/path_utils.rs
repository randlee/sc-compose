use std::path::{Path, PathBuf};

pub(crate) fn to_forward_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
    dead_code,
    clippy::ptr_arg,
    clippy::ref_option,
    reason = "early Phase B branches do not yet serialize Option<PathBuf> but later branches do"
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
