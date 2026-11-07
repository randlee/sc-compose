//! Deterministic fake executables for Unix tests.
//!
//! Writing a script and then exec'ing it races with process-spawning tests on
//! other libtest threads: a concurrent fork can inherit the still-open write
//! descriptor, and exec then fails with `ETXTBSY`. Here no file the test writes
//! is ever exec'd. The fake path is a symlink to the committed `fake-exec`
//! shim, which runs the test's script, written beside it as `<path>.sh`, with
//! `/bin/sh` as plain input.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const SHIM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../test-support/fake-exec");

/// Makes `path` an executable that runs the `/bin/sh` script `body` with the
/// caller's arguments.
pub fn install(path: &Path, body: &str) {
    fs::write(script_path(path), body).expect("write fake executable script");
    std::os::unix::fs::symlink(SHIM, path).expect("link fake executable to the committed shim");
}

/// Returns `<path>.sh`, where `install` writes the script. A test that edits
/// a fake's behavior edits this file; writing through `path` would write the
/// committed shim.
#[allow(
    dead_code,
    reason = "each including test binary uses only some helpers"
)]
pub fn script_path(path: &Path) -> PathBuf {
    let mut script = OsString::from(path.as_os_str());
    script.push(".sh");
    PathBuf::from(script)
}
