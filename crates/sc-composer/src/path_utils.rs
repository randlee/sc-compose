use std::path::Path;

/// Normalize a path string to forward slashes for stable JSON and diagnostics.
#[must_use]
pub fn to_forward_slash(path: &Path) -> String {
    let path = path.to_string_lossy();
    let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
    path.replace('\\', "/")
}
