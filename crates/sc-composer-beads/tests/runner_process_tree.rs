//! Process-tree containment regression coverage.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sc_composer_beads::runner::PROCESS_OUTPUT_LIMIT_BYTES;
use sc_composer_beads::{CommandSpec, ProcessRunner, StdProcessRunner};

const FIXTURE: &str = env!("CARGO_BIN_EXE_sc-composer-beads-runner-fixture");
const MAX_RUN_DURATION: Duration = Duration::from_secs(3);

#[test]
fn output_cap_terminates_a_pipe_holding_descendant() {
    let root = temporary_directory();
    let state_file = root.join("descendant-state");
    let ready_file = root.join("descendant-ready");
    let spec = CommandSpec {
        executable: PathBuf::from(FIXTURE),
        args: vec![
            "root".to_owned(),
            (PROCESS_OUTPUT_LIMIT_BYTES + 1).to_string(),
            state_file.display().to_string(),
            ready_file.display().to_string(),
        ],
        working_directory: root.clone(),
    };

    let started = Instant::now();
    let error = StdProcessRunner
        .run(&spec)
        .expect_err("output cap must terminate the process tree");

    assert!(
        started.elapsed() < MAX_RUN_DURATION,
        "runner exceeded the deadline plus grace: {:?}",
        started.elapsed()
    );
    assert!(
        error.to_string().contains("process output limit exceeded"),
        "expected output-limit failure, got: {error}"
    );
    assert_stopped(&state_file);
    fs::remove_dir_all(root).expect("cleanup fixture directory");
}

fn assert_stopped(state_file: &Path) {
    for _ in 0..50 {
        if state_file.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let before = fs::read(state_file).expect("descendant state file exists before containment");
    thread::sleep(Duration::from_millis(150));
    let after =
        fs::read(state_file).expect("descendant state file remains readable after containment");
    assert_eq!(
        before, after,
        "descendant continued updating its state after runner containment"
    );
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
