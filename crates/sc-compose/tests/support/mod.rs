#![allow(
    dead_code,
    reason = "shared helpers are selected by separate integration-test binaries"
)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

// macOS can report the same wall-clock timestamp for parallel test threads.
// Keep fixture paths unique within a test process so one fixture's Drop cannot
// remove another test's current directory or subprocess executable.
static TEMP_ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn temp_root(label: &str) -> PathBuf {
    temp_root_with_prefix(label, "sc-compose-test")
}

fn temp_root_with_prefix(label: &str, prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = TEMP_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{label}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    root
}

pub struct TempFixture {
    pub path: PathBuf,
}

#[derive(Clone, Copy)]
pub struct CheckedInFixture<'a> {
    pub group: &'a str,
    pub name: &'a str,
    pub target: &'a str,
}

#[derive(Clone, Copy)]
pub struct FakeCargoOptions {
    pub xwin_available: bool,
    pub test_failure: bool,
    pub fail_closed: bool,
}

impl TempFixture {
    pub fn new(label: &str) -> Self {
        Self {
            path: temp_root_with_prefix(label, "sc-compose-fixture"),
        }
    }

    pub fn from_checked_in_fixture(spec: CheckedInFixture<'_>) -> Self {
        let fixture_root = Self::new(&format!("{}-{}", spec.group, spec.name));
        let source = repo_root()
            .join("tests/fixtures/sc-lint")
            .join(spec.group)
            .join(spec.name);
        copy_directory(&source, &fixture_root.path);

        let target_dir = fixture_root.path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).unwrap();
        fs::copy(
            repo_root()
                .join(".sc/sc-lint/targets")
                .join(format!("{}.toml", spec.target)),
            target_dir.join(format!("{}.toml", spec.target)),
        )
        .unwrap();
        fixture_root
    }

    pub fn path_with_fake_tools(&self) -> String {
        let mut paths = vec![self.path.join("fake-bin")];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths)
            .expect("PATH entries")
            .to_string_lossy()
            .into_owned()
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

pub struct ScComposeCommand {
    command: Command,
    log_root: TempFixture,
}

impl ScComposeCommand {
    pub fn log_root_path(&self) -> &Path {
        &self.log_root.path
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.command.env(key, value);
        self
    }

    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Self {
        self.command.env_remove(key);
        self
    }

    pub fn stdin(&mut self, cfg: Stdio) -> &mut Self {
        self.command.stdin(cfg);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.command.current_dir(dir);
        self
    }

    pub fn status(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.command.status()
    }

    pub fn output(&mut self) -> std::io::Result<std::process::Output> {
        self.command.output()
    }
}

pub fn sc_compose() -> ScComposeCommand {
    let log_root = TempFixture::new("sc-compose-logs");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-compose"));
    command.env("SC_LOG_ROOT", &log_root.path);
    ScComposeCommand { command, log_root }
}

/// Put the verified release binary and installed wheel interpreter on PATH.
pub fn installed_sc_lint_path(prefix: Option<&Path>) -> std::ffi::OsString {
