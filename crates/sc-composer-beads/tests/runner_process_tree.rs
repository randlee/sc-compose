//! Process-tree containment regression coverage.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use sc_composer_beads::runner::PROCESS_OUTPUT_LIMIT_BYTES;
use sc_composer_beads::{CommandSpec, ProcessRunner, StdProcessRunner};

const FIXTURE: &str = env!("CARGO_BIN_EXE_sc-composer-beads-runner-fixture");

#[test]
fn normal_nonzero_exit_status_is_preserved() {
    let root = temporary_directory();
    let spec = CommandSpec {
        executable: PathBuf::from(FIXTURE),
        args: vec!["exit".to_owned(), "9".to_owned()],
        working_directory: root.clone(),
    };

    let output = StdProcessRunner
        .run(&spec)
        .expect("run nonzero-exit fixture");

    assert_eq!(output.exit_status, Some(9));
    fs::remove_dir_all(root).expect("cleanup fixture directory");
}

#[cfg(unix)]
#[test]
fn unix_process_group_terminates_a_pipe_holding_descendant() {
    assert_contained_descendant_is_terminated();
}

#[cfg(windows)]
#[test]
fn windows_job_object_terminates_a_pipe_holding_descendant() {
    assert_contained_descendant_is_terminated();
}

fn assert_contained_descendant_is_terminated() {
    let root = temporary_directory();
    let state_file = root.join("descendant-state");
    let spec = CommandSpec {
        executable: PathBuf::from(FIXTURE),
        args: vec![
            "root".to_owned(),
            (PROCESS_OUTPUT_LIMIT_BYTES + 1).to_string(),
            state_file.display().to_string(),
        ],
        working_directory: root.clone(),
    };

    let error = StdProcessRunner
        .run(&spec)
        .expect_err("output cap must terminate the process tree");

    assert!(
        error.to_string().contains("process output limit exceeded"),
        "expected output-limit failure, got: {error}"
    );
    assert_eq!(
        fs::read_to_string(&state_file).expect("descendant started before overflow"),
        "started"
    );
    fs::remove_dir_all(root).expect("cleanup fixture directory");
}

fn temporary_directory() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sc-composer-beads-runner-process-tree-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create fixture directory");
    root
}
