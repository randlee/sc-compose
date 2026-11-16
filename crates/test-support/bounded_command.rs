#![allow(
    dead_code,
    reason = "shared test helper is used by multiple test binaries"
)]
use std::io::{self, Read};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn drain_then_wait<T>(
    stdout: impl FnOnce() -> Vec<u8>,
    stderr: impl FnOnce() -> Vec<u8>,
    wait: impl FnOnce() -> T,
) -> (Vec<u8>, Vec<u8>, T) {
    let stdout = stdout();
    let stderr = stderr();
    let status = wait();
    (stdout, stderr, status)
}

/// Read continuously so a timed-out child can report output captured so far.
fn capture_pipe(mut pipe: impl Read, bytes: &Mutex<Vec<u8>>) -> std::io::Result<()> {
    let mut buffer = [0; 8192];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => bytes
                .lock()
                .expect("captured output")
                .extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// A ready result wins; the deadline is solely a failure guard.
pub fn wait_for_completion<T>(
    mut poll: impl FnMut() -> std::io::Result<Option<T>>,
    mut expired: impl FnMut() -> bool,
    mut pause: impl FnMut(),
) -> std::io::Result<Option<T>> {
    loop {
        if let Some(result) = poll()? {
            return Ok(Some(result));
        }
        if expired() {
            return Ok(None);
        }
        pause();
    }
}

pub fn run_command(command: &mut Command) -> Output {
    try_run_command(command).expect("bd spawn")
}

#[allow(
    clippy::zombie_processes,
    reason = "a failed kill cannot be followed by a potentially unbounded wait"
)]
pub fn try_run_command(command: &mut Command) -> io::Result<Output> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("piped bd stdout");
    let stderr = child.stderr.take().expect("piped bd stderr");
    let stdout_bytes = Arc::new(Mutex::new(Vec::new()));
    let stderr_bytes = Arc::new(Mutex::new(Vec::new()));
    let out = Arc::clone(&stdout_bytes);
    let err = Arc::clone(&stderr_bytes);
    let stdout_reader = std::thread::spawn(move || capture_pipe(stdout, &out));
    let stderr_reader = std::thread::spawn(move || capture_pipe(stderr, &err));
    let started = Instant::now();
    let mut exited = None;
    let status = wait_for_completion(
        || {
            if exited.is_none() {
                exited = child.try_wait()?;
            }
            Ok(exited.filter(|_| stdout_reader.is_finished() && stderr_reader.is_finished()))
        },
        || started.elapsed() >= Duration::from_secs(600),
        || std::thread::sleep(Duration::from_millis(10)),
    );
    let status = match status {
        Ok(Some(status)) => status,
        failure => {
            let kill_error = child.kill().err();
            if kill_error.is_none() {
                let _ = child.wait();
            }
            // Never join unfinished readers: a descendant might still own a pipe.
            panic!(
                "command {command:?} did not complete within the 600s failure deadline: {failure:?}; kill: {kill_error:?}\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&stdout_bytes.lock().expect("stdout snapshot")),
                String::from_utf8_lossy(&stderr_bytes.lock().expect("stderr snapshot"))
            );
        }
    };
    let (stdout, stderr, status) = drain_then_wait(
        || {
            stdout_reader
                .join()
                .expect("bd stdout reader")
                .expect("bd stdout");
            std::mem::take(&mut *stdout_bytes.lock().expect("stdout capture"))
        },
        || {
            stderr_reader
                .join()
                .expect("bd stderr reader")
                .expect("bd stderr");
            std::mem::take(&mut *stderr_bytes.lock().expect("stderr capture"))
        },
        || status,
    );
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}
