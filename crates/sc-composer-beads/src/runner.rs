//! Direct executable invocation without a shell.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Maximum number of bytes retained from each `bd` output stream.
///
/// The runner terminates a child that exceeds this limit, rather than allowing
/// an untrusted subprocess to consume unbounded memory.
pub const PROCESS_OUTPUT_LIMIT_BYTES: usize = 64 * 1024;

const OUTPUT_LIMIT_ERROR_MARKER: &str = "sc-composer-beads process output limit exceeded";

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
        let mut child = Command::new(&spec.executable)
            .args(&spec.args)
            .current_dir(&spec.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("child stdout was not captured"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("child stderr was not captured"))?;
        let (overflow_sender, overflow_receiver) = mpsc::channel();
        let stdout_reader = spawn_capture(stdout, overflow_sender.clone());
        let stderr_reader = spawn_capture(stderr, overflow_sender);

        let mut exceeded_limit = false;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            match overflow_receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(()) => {
                    exceeded_limit = true;
                    if let Err(error) = child.kill()
                        && error.kind() != io::ErrorKind::InvalidInput
                    {
                        return Err(error);
                    }
                    break child.wait()?;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        };
        let stdout = join_capture(stdout_reader)?;
        let stderr = join_capture(stderr_reader)?;
        exceeded_limit |= stdout.exceeded || stderr.exceeded;
        if exceeded_limit {
            return Err(process_output_limit_error());
        }
        Ok(ProcessOutput {
            exit_status: status.code(),
            stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
            stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
            elapsed: started.elapsed(),
        })
    }
}

#[derive(Debug)]
struct CapturedStream {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn spawn_capture<R>(
    stream: R,
    overflow_sender: mpsc::Sender<()>,
) -> thread::JoinHandle<io::Result<CapturedStream>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || capture_stream(stream, &overflow_sender))
}

fn capture_stream<R>(
    mut stream: R,
    overflow_sender: &mpsc::Sender<()>,
) -> io::Result<CapturedStream>
where
    R: Read,
{
    let mut bytes = Vec::with_capacity(PROCESS_OUTPUT_LIMIT_BYTES);
    let mut exceeded = false;
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Ok(CapturedStream { bytes, exceeded });
        }
        let remaining = PROCESS_OUTPUT_LIMIT_BYTES.saturating_sub(bytes.len());
        let retained = read.min(remaining);
        bytes.extend_from_slice(&buffer[..retained]);
        if retained < read && !exceeded {
            exceeded = true;
            let _ = overflow_sender.send(());
        }
    }
}

fn join_capture(
    reader: thread::JoinHandle<io::Result<CapturedStream>>,
) -> io::Result<CapturedStream> {
    reader
        .join()
        .map_err(|_panic_payload| io::Error::other("child output reader panicked"))?
}

pub(crate) fn process_output_limit_error() -> io::Error {
    io::Error::other(OUTPUT_LIMIT_ERROR_MARKER)
}

pub(crate) fn is_process_output_limit_error(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::Other && error.to_string() == OUTPUT_LIMIT_ERROR_MARKER
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::fs;
    use std::io::Cursor;
    #[cfg(unix)]
    use std::path::PathBuf;
    use std::sync::mpsc;
    #[cfg(unix)]
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        CommandSpec, PROCESS_OUTPUT_LIMIT_BYTES, ProcessRunner, StdProcessRunner, capture_stream,
        is_process_output_limit_error, process_output_limit_error,
    };

    #[test]
    fn capture_stream_retains_only_the_hard_limit_and_reports_overflow() {
        let (sender, receiver) = mpsc::channel();
        let stream = Cursor::new(vec![b'x'; PROCESS_OUTPUT_LIMIT_BYTES + 1]);

        let captured = capture_stream(stream, &sender).expect("capture stream");

        assert!(captured.exceeded);
        assert_eq!(captured.bytes.len(), PROCESS_OUTPUT_LIMIT_BYTES);
        receiver.try_recv().expect("overflow signal");
    }

    #[test]
    fn process_output_limit_error_has_a_distinct_internal_marker() {
        assert!(is_process_output_limit_error(&process_output_limit_error()));
    }

    #[cfg(unix)]
    #[test]
    fn std_runner_terminates_a_child_that_exceeds_the_output_limit() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_directory();
        let script = root.join("overflow.sh");
        fs::write(&script, "#!/bin/sh\nwhile :; do printf x; done\n")
            .expect("write overflow process");
        let mut permissions = fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("make script executable");
        let spec = CommandSpec {
            executable: script,
            args: Vec::new(),
            working_directory: root.clone(),
        };

        let error = StdProcessRunner
            .run(&spec)
            .expect_err("unbounded fake process must be stopped");

        assert!(is_process_output_limit_error(&error));
        fs::remove_dir_all(root).expect("cleanup");
    }

    // FUZZ-4177-OUT-01 (adversarial fuzz campaign 20260817-2, output-bound-probe):
    // confirmed bug, not yet fixed. `StdProcessRunner::run` kills only the direct
    // child process (`Child::kill`, one pid) once the 64 KiB per-stream output
    // limit is exceeded. If that child has already spawned a detached descendant
    // that inherited the piped stdout file descriptor (a `&`-backgrounded
    // subshell, reparented once its parent exits), that descriptor stays open
    // even after the direct child is killed and reaped. The reader thread's
    // blocking `read()` never observes EOF until every process holding the
    // write end closes it, so `join_capture` -- and therefore `run()` -- blocks
    // for as long as the orphaned descendant stays alive. This defeats the
    // bounded-capture guarantee ("an over-limit child is terminated ... rather
    // than consuming unbounded memory", ADR-0021) in practice: the child is
    // reaped, but the call still hangs. This test bounds the orphan's lifetime
    // to ~1s (via `sleep 1` rather than an unbounded flood) so the suite stays
    // fast and deterministic, while still proving the runner remains blocked
    // well past the point a process-group-aware kill would have returned.
    // Update this test once the runner terminates the whole process group
    // instead of only the direct child pid.
    #[cfg(unix)]
    #[test]
    fn orphaned_descendant_holding_the_output_pipe_open_hangs_the_runner() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::{Duration, Instant};

        let root = temporary_directory();
        let script = root.join("orphan-flood.sh");
        fs::write(
            &script,
            "#!/bin/sh\n( sleep 1 ) &\nwhile :; do printf 'XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX'; done\n",
        )
        .expect("write orphan-descendant overflow process");
        let mut permissions = fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("make script executable");
        let spec = CommandSpec {
            executable: script,
            args: Vec::new(),
            working_directory: root.clone(),
        };

        let (sender, receiver) = mpsc::channel();
        let started = Instant::now();
        std::thread::spawn(move || {
            let result = StdProcessRunner.run(&spec);
            let _ = sender.send(result.is_err());
        });

        // A runner that terminated the whole process group would return almost
        // immediately once the flooding direct child is killed. Give it a
        // generous margin short of the orphan's ~1s held-open descriptor.
        let quick = receiver.recv_timeout(Duration::from_millis(400));
        assert!(
            quick.is_err(),
            "expected the runner to still be blocked on the orphaned \
             descendant's open descriptor at 400ms, but it returned early -- \
             the hang may have been fixed; update this regression test"
        );

        let eventually = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("the orphan's ~1s sleep must eventually close the descriptor");
        assert!(
            eventually,
            "the overflowed run must still resolve to an error"
        );
        assert!(
            started.elapsed() >= Duration::from_millis(400),
            "run() should have blocked past the direct child's kill until the \
             orphaned descendant exited"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    fn temporary_directory() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sc-composer-beads-runner-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create test directory");
        root
    }
}
