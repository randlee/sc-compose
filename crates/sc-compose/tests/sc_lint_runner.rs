use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
mod support;
use support::{TempFixture, parse_stdout, sc_compose, write_file};

#[test]
fn sc_compose_log_root_is_removed_when_command_finishes() {
    let (log_root, output) = {
        let mut command = sc_compose();
        let log_root = command.log_root_path().to_path_buf();
        let output = command.arg("--help").output().expect("run sc-compose help");
        (log_root, output)
    };
    assert!(output.status.success());
    assert!(
        !log_root.exists(),
        "sc-compose log root leaked: {}",
        log_root.display()
    );
}

#[test]
fn fixture_cleanup_survives_a_panicking_assertion() {
    let mut fixture_path = None;
    let mut log_root = None;
    let result = catch_unwind(AssertUnwindSafe(|| {
        let fixture = TempFixture::new("sc-lint-runner-panic");
        fixture_path = Some(fixture.path.clone());

        let command = sc_compose();
        log_root = Some(command.log_root_path().to_path_buf());

        let should_pass = std::hint::black_box(false);
        assert!(should_pass, "intentional panic to exercise RAII cleanup");
    }));

    assert!(result.is_err());
    assert!(!fixture_path.expect("fixture path").exists());
    assert!(!log_root.expect("log root").exists());
}

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
        support::fake_exec::install(
            &path,
            "#!/bin/sh\nprintf '%s\\n' '{\"ok\":true,\"command\":\"lint.sc-boundary\",\"data\":{\"findings\":[]},\"diagnostics\":[]}'\n",
        );
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
    let output = sc_compose()
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
        .output()
        .expect("run sc-compose lint");
    assert_eq!(
        output.status.code(),
        Some(0),
        "sc-compose lint failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = parse_stdout(&output);
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
}

// A concurrent fork inheriting a write descriptor on a fake's script must not
// break its exec (ETXTBSY): the descriptor is held open while sc-compose runs
// the fake, and the run is unchanged.
#[cfg(unix)]
#[test]
fn runner_execs_fake_sc_lint_while_its_script_is_held_open_for_writing() {
    let root = TempFixture::new("sc-lint-runner-held-writer");
    write_file(
        &root.path.join(".sc/sc-lint/targets/sc-boundary.toml"),
        "command = \"lint.sc-boundary\"\nreport_kind = \"lint\"\n",
    );
    let fake_bin = fake_sc_lint_bin(&root.path);
    let _inherited_writer = fs::OpenOptions::new()
        .append(true)
        .open(support::fake_exec::script_path(&fake_bin))
        .expect("hold the fake script's write descriptor open");
    let output = sc_compose()
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
        .output()
        .expect("run sc-compose lint");
    assert_eq!(
        output.status.code(),
        Some(0),
        "sc-compose lint failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(
        parse_stdout(&output)["payload"]["raw_payload"]["command"],
        "lint.sc-boundary"
    );
}

// Every shared fake tool is exec'd through the committed shim; no file the
// test wrote is itself executable.
#[cfg(unix)]
#[test]
fn shared_fake_tools_are_links_to_the_committed_shim() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempFixture::new("sc-lint-runner-fake-links");
    support::write_fake_cargo(
        &root.path,
        support::FakeCargoOptions {
            xwin_available: true,
            test_failure: false,
            fail_closed: false,
        },
    );
    support::write_fake_cargo_deny(&root.path);
    let shim = fs::canonicalize(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test-support/fake-exec"
    ))
    .expect("committed shim");
    let fakes = [
        root.path.join("fake-bin/cargo"),
        root.path.join("fake-bin/cargo-deny"),
        fake_sc_lint_bin(&root.path),
    ];
    for fake in fakes {
        assert_eq!(
            fs::canonicalize(&fake).expect("fake target"),
            shim,
            "{fake:?}"
        );
        let script = fs::metadata(support::fake_exec::script_path(&fake)).expect("fake script");
        assert_eq!(
            script.permissions().mode() & 0o111,
            0,
            "{fake:?} script is executable"
        );
    }
}

#[test]
fn runner_rejects_commands_without_a_descriptor() {
    let root = TempFixture::new("sc-lint-runner");
    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.path.to_str().expect("UTF-8 root"),
            "--target",
            "sh -c touch /tmp/not-allowed",
            "--json",
        ])
        .output()
        .expect("run sc-compose lint");
    assert_eq!(output.status.code(), Some(3));
    let envelope = parse_stdout(&output);
    assert_eq!(envelope["diagnostics"][0]["code"], "ERR_CONFIG_READ");
}
