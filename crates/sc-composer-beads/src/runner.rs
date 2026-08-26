//! Direct executable invocation without a shell.

use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// One direct process invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    /// Executable path or executable name.
    pub executable: PathBuf,
    /// Arguments excluding the executable.
    pub args: Vec<String>,
    /// Process working directory.
    pub working_directory: PathBuf,
}

impl CommandSpec {
    /// Return the complete direct argv evidence.
    #[must_use]
    pub fn argv(&self) -> Vec<String> {
        std::iter::once(self.executable.to_string_lossy().into_owned())
            .chain(self.args.iter().cloned())
            .collect()
    }
}

/// Captured result from one direct process invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessOutput {
    /// Process exit status, if the process started.
    pub exit_status: Option<i32>,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// Process wall-clock duration.
    pub elapsed: Duration,
}

/// Executes a direct command for a Beads operation.
pub trait ProcessRunner {
    /// Run `spec` directly and capture its bounded outcome.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the executable could not be started or its
    /// output could not be captured.
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput>;
}

/// Production runner backed by [`std::process::Command`].
#[derive(Clone, Copy, Debug, Default)]
pub struct StdProcessRunner;

impl ProcessRunner for StdProcessRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        let started = Instant::now();
        let output = Command::new(&spec.executable)
            .args(&spec.args)
            .current_dir(&spec.working_directory)
            .output()?;
        Ok(ProcessOutput {
            exit_status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            elapsed: started.elapsed(),
        })
    }
}
