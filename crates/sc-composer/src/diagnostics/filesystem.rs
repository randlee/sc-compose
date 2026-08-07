use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilesystemErrorClass {
    InvalidData,
    PermissionDenied,
    IsADirectory,
    FilesystemLoop,
    NotFound,
}

pub(crate) fn classify_filesystem_error(
    path: &Path,
    error: &std::io::Error,
) -> FilesystemErrorClass {
    if path.is_dir() {
        return FilesystemErrorClass::IsADirectory;
    }

    match error.kind() {
        std::io::ErrorKind::InvalidData => FilesystemErrorClass::InvalidData,
        std::io::ErrorKind::PermissionDenied => FilesystemErrorClass::PermissionDenied,
        std::io::ErrorKind::IsADirectory => FilesystemErrorClass::IsADirectory,
        _ if is_filesystem_loop(error) => FilesystemErrorClass::FilesystemLoop,
        _ => FilesystemErrorClass::NotFound,
    }
}

fn is_filesystem_loop(error: &std::io::Error) -> bool {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        error.raw_os_error() == Some(40)
    }

    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        error.raw_os_error() == Some(62)
    }

    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(1142 | 1921))
    }

    #[cfg(not(any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "windows"
    )))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{FilesystemErrorClass, classify_filesystem_error};

    #[test]
    fn classifies_stable_io_error_categories() {
        let missing = std::env::temp_dir().join("sc-compose-diagnostics-missing");
        assert_eq!(
            classify_filesystem_error(
                &missing,
                &std::io::Error::from(std::io::ErrorKind::InvalidData)
            ),
            FilesystemErrorClass::InvalidData
        );
        assert_eq!(
            classify_filesystem_error(
                &missing,
                &std::io::Error::from(std::io::ErrorKind::PermissionDenied)
            ),
            FilesystemErrorClass::PermissionDenied
        );
        assert_eq!(
            classify_filesystem_error(
                &missing,
                &std::io::Error::from(std::io::ErrorKind::NotFound)
            ),
            FilesystemErrorClass::NotFound
        );
        assert_eq!(
            classify_filesystem_error(
                &missing,
                &std::io::Error::from(std::io::ErrorKind::IsADirectory)
            ),
            FilesystemErrorClass::IsADirectory
        );
    }

    #[test]
    fn existing_directory_wins_over_error_kind() {
        let path = std::env::temp_dir();
        assert_eq!(
            classify_filesystem_error(&path, &std::io::Error::from(std::io::ErrorKind::NotFound)),
            FilesystemErrorClass::IsADirectory
        );
    }
}
