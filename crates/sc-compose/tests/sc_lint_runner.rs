use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

mod support;
use support::{TempFixture, write_file};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn fake_sc_lint_bin(root: &Path) -> PathBuf {
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("fake bin directory");

    #[cfg(windows)]
    {
        let path = bin_dir.join("sc-lint.cmd");
        fs::write(
            &path,
            "@echo {\"ok\":true,\"command\":\"lint.sc-boundary\",\"data\":{\"findings\":[]},\"diagnostics\":[]}\r\n",
        )
        .expect("fake sc-lint");
        path
    }

    #[cfg(unix)]
    {
        let path = bin_dir.join("sc-lint");
        fs::write(
            &path,
            "#!/bin/sh\nprintf '%s\\n' '{\"ok\":true,\"command\":\"lint.sc-boundary\",\"data\":{\"findings\":[]},\"diagnostics\":[]}'\n",
        )
        .expect("fake sc-lint");
        let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("fake executable");
        path
    }
}

fn path_with_fake_bin(bin_dir: &Path) -> String {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH entries")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn runner_preserves_sc_lint_envelope_and_writes_both_artifacts() {
    let root = TempFixture::new("sc-lint-runner");
    write_file(
        &root.path.join(".sc/sc-lint/targets/sc-boundary.toml"),
        "command = \"lint.sc-boundary\"\nreport_kind = \"lint\"\n",
    );
    let fake_bin = fake_sc_lint_bin(&root.path);
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            root.path.to_str().expect("UTF-8 root"),
            "--target",
            "sc-boundary",
            "--json",
        ])
        .env(
            "PATH",
            path_with_fake_bin(fake_bin.parent().expect("fake bin parent")),
        )
        .env("SC_LOG_ROOT", root.path.join("logs"))
        .output()
        .expect("run sc-compose lint");
    assert_eq!(
        output.status.code(),
        Some(0),
        "sc-compose lint failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope: Value = serde_json::from_slice(&output.stdout).expect("JSON envelope");
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-boundary");
    assert_eq!(payload["exit_status"], 0);
    assert!(
        root.path
            .join("reports/latest/sc-lint/raw/lint.sc-boundary.json")
            .is_file()
    );
    assert!(
        root.path
            .join("reports/latest/sc-lint/index.html")
            .is_file()
    );
    assert!(root.path.join("logs").is_dir());
}

#[test]
fn runner_rejects_commands_without_a_descriptor() {
    let root = TempFixture::new("sc-lint-runner");
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            root.path.to_str().expect("UTF-8 root"),
            "--target",
            "sh -c touch /tmp/not-allowed",
            "--json",
        ])
        .env("SC_LOG_ROOT", root.path.join("logs"))
        .output()
        .expect("run sc-compose lint");
    assert_eq!(output.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("JSON error envelope");
    assert_eq!(envelope["diagnostics"][0]["code"], "ERR_CONFIG_READ");
}
