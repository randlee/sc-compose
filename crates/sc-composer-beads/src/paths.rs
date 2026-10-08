//! User-visible path display.
//!
//! Canonicalized Windows paths carry a verbatim (`\\?\`) prefix that must
//! never reach receipts, errors or `bd` arguments; every displayed path goes
//! through [`public_path_display`].

use std::path::{Path, PathBuf};

#[cfg(windows)]
pub(crate) fn public_path_display(path: &Path) -> String {
    use std::{
        ffi::OsString,
        path::{Component, Prefix},
    };

    let mut displayed = PathBuf::new();
    let mut skip_root = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::VerbatimDisk(drive) => displayed.push(format!("{}:", drive as char)),
                Prefix::VerbatimUNC(server, share) => {
                    let separator = std::path::MAIN_SEPARATOR_STR;
                    let mut base = OsString::from(separator);
                    base.push(separator);
                    base.push(server);
                    base.push(separator);
                    base.push(share);
                    displayed = PathBuf::from(base);
                    skip_root = true;
                }
                _ => displayed.push(prefix.as_os_str()),
            },
            Component::RootDir if skip_root => skip_root = false,
            component => displayed.push(component.as_os_str()),
        }
    }
    displayed.to_string_lossy().into_owned()
}

/// Owned form of [`public_path_display`] for storing in user-visible errors.
pub(crate) fn public_path_buf(path: &Path) -> PathBuf {
    PathBuf::from(public_path_display(path))
}

#[cfg(all(not(windows), not(test)))]
pub(crate) fn public_path_display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Test seam: non-Windows canonical paths never carry a verbatim prefix, so a
/// test can install a marker that `public_path_display` prepends. Any error
/// path that bypasses `public_path_display` then lacks the marker.
#[cfg(all(not(windows), test))]
pub(crate) fn public_path_display(path: &Path) -> String {
    let displayed = path.to_string_lossy().into_owned();
    PUBLIC_PATH_MARKER.with(|marker| match marker.borrow().as_deref() {
        Some(marker) => format!("{marker}{displayed}"),
        None => displayed,
    })
}

#[cfg(all(not(windows), test))]
thread_local! {
    pub(crate) static PUBLIC_PATH_MARKER: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::public_path_display;

    #[cfg(windows)]
    #[test]
    fn public_path_display_strips_windows_verbatim_prefix() {
        assert_eq!(
            public_path_display(Path::new(r"\\?\C:\Users\test\sample.formula.toml")),
            r"C:\Users\test\sample.formula.toml"
        );
    }

    #[cfg(windows)]
    #[test]
    fn public_path_display_converts_verbatim_unc_to_unc() {
        let separator = std::path::MAIN_SEPARATOR;
        let path = format!(
            "{separator}{separator}?{separator}UNC{separator}server{separator}share{separator}f"
        );
        assert_eq!(
            public_path_display(Path::new(&path)),
            format!("{separator}{separator}server{separator}share{separator}f")
        );
    }

    #[test]
    fn public_path_display_preserves_non_verbatim_paths() {
        assert_eq!(
            public_path_display(Path::new("plain.formula.toml")),
            "plain.formula.toml"
        );
    }
}
