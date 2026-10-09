//! Request-owned input files kept alive while bd consumes them.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::BeadComposeError;
use crate::paths::public_path_buf;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) struct InputSnapshot {
    path: PathBuf,
}

impl InputSnapshot {
    pub(crate) fn reserve(destination: &Path) -> Result<Self, BeadComposeError> {
        let parent = destination
            .parent()
            .ok_or_else(|| BeadComposeError::TemplatePathInvalid {
                path: public_path_buf(destination),
            })?;
        // bd chooses TOML only for the complete .formula.toml suffix.
        let name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let suffix = if name.ends_with(".formula.toml") {
            "formula.toml"
        } else if name.ends_with(".formula.json") {
            "formula.json"
        } else {
            destination
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("json")
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(render_error)?
            .as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".sc-compose-input-{}-{timestamp}-{sequence}.{suffix}",
            std::process::id()
        ));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(render_error)?;
        Ok(Self { path })
    }

    pub(crate) fn write(destination: &Path, contents: &[u8]) -> Result<Self, BeadComposeError> {
        let snapshot = Self::reserve(destination)?;
        let mut file = OpenOptions::new()
            .write(true)
            .open(snapshot.path())
            .map_err(render_error)?;
        file.write_all(contents).map_err(render_error)?;
        file.sync_all().map_err(render_error)?;
        Ok(snapshot)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn read(&self) -> Result<Vec<u8>, BeadComposeError> {
        fs::read(&self.path).map_err(render_error)
    }

    /// Publish complete bytes without relinquishing the request-owned bd input.
    pub(crate) fn publish_copy(&self, destination: &Path) -> Result<(), BeadComposeError> {
        let contents = self
            .read()
            .map_err(|error| output_error(destination, error))?;
        crate::render::atomic_write(destination, &contents)
            .map_err(|error| output_error(destination, error))
    }
}

impl Drop for InputSnapshot {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn render_error(error: impl std::fmt::Display) -> BeadComposeError {
    BeadComposeError::RenderFailed {
        message: error.to_string(),
    }
}

pub(crate) fn output_error(destination: &Path, error: BeadComposeError) -> BeadComposeError {
    match error {
        BeadComposeError::RenderFailed { message } => BeadComposeError::OutputPathInvalid {
            path: public_path_buf(destination),
            rule: format!("cannot publish output: {message}"),
        },
        other => other,
    }
}
