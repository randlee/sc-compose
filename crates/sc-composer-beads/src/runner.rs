//! Direct executable invocation without a shell.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use process_wrap::std::{ChildWrapper, CommandWrap, JobObject};
#[cfg(unix)]
use process_wrap::std::{ChildWrapper, CommandWrap, ProcessGroup};

/// Maximum bytes retained per legacy `bd` stream and graph diagnostic stderr.
///
/// The runner terminates its contained process tree when this limit is exceeded,
/// rather than allowing an untrusted subprocess to consume unbounded memory.
pub const PROCESS_OUTPUT_LIMIT_BYTES: usize = 64 * 1024;

/// Maximum graph JSON stdout capture; diagnostic stderr retains the legacy cap.
pub const GRAPH_OUTPUT_LIMIT_BYTES: usize = 16 * 1024 * 1024;

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
    /// On a per-stream output-cap breach, the production runner terminates the
    /// contained process tree before returning an error. Unix uses a dedicated
    /// process group and Windows uses a Job Object. Other platforms retain the
    /// direct-child behavior because they have no equivalent containment
    /// primitive in this implementation; they make no descendant-tree cleanup
    /// guarantee.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the executable could not be started or its
    /// output could not be captured.
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput>;

    /// Run a graph JSON command with bounded graph output capture.
    ///
    /// The default preserves existing custom runners. The production runner
    /// permits up to 16 MiB of graph stdout and 64 KiB of diagnostic stderr,
    /// with the same process-tree containment as [`Self::run`].
    ///
    /// # Errors
    /// Returns an I/O error on spawn, capture failure, or output-cap overflow.
    fn run_graph(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        self.run(spec)
    }
}

/// Production runner backed by [`std::process::Command`].
#[derive(Clone, Copy, Debug, Default)]
pub struct StdProcessRunner;

impl ProcessRunner for StdProcessRunner {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        run_bounded(spec, PROCESS_OUTPUT_LIMIT_BYTES)
    }

    fn run_graph(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        run_bounded(spec, GRAPH_OUTPUT_LIMIT_BYTES)
    }
}

fn run_bounded(spec: &CommandSpec, stdout_limit: usize) -> io::Result<ProcessOutput> {
    let started = Instant::now();
    let mut child = spawn_contained(spec)?;
    let stdout = child
        .take_stdout()
        .ok_or_else(|| io::Error::other("child stdout was not captured"))?;
    let stderr = child
        .take_stderr()
        .ok_or_else(|| io::Error::other("child stderr was not captured"))?;
    let (capture_sender, capture_receiver) = mpsc::channel();
    let stdout_reader = spawn_capture(
        stdout,
        StreamKind::Stdout,
        capture_sender.clone(),
        stdout_limit,
    );
    let stderr_reader = spawn_capture(
        stderr,
        StreamKind::Stderr,
        capture_sender,
        PROCESS_OUTPUT_LIMIT_BYTES,
    );

    let mut capture = CaptureTracker::new();
    let mut terminated_for_limit = false;
    let mut capture_disconnected = false;
    while !capture.is_complete() {
        match capture_receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(event) => capture.observe(event),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                capture_disconnected = true;
            }
        }

        terminate_capture_failure(
            capture_disconnected || capture.requires_contained_termination(),
            child.as_mut(),
            &mut terminated_for_limit,
        )?;

        if capture_disconnected {
            break;
        }
    }

    join_capture_readers(stdout_reader, stderr_reader)?;
    if capture_disconnected {
        return Err(io::Error::other(
            "child output capture closed before both streams completed",
        ));
    }

    let (stdout, stderr) = capture.finish()?;
    debug_assert!(
        !terminated_for_limit,
        "a contained termination must have a capture failure result"
    );
    Ok(ProcessOutput {
        exit_status: child.wait()?.code(),
        stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
        stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
        elapsed: started.elapsed(),
    })
}

trait ManagedChild {
    fn take_stdout(&mut self) -> Option<ChildStdout>;
    fn take_stderr(&mut self) -> Option<ChildStderr>;
    fn wait(&mut self) -> io::Result<ExitStatus>;
    fn terminate(&mut self) -> io::Result<ExitStatus>;
}

impl ManagedChild for std::process::Child {
    fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.stdout.take()
    }

    fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.stderr.take()
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        std::process::Child::wait(self)
    }

    fn terminate(&mut self) -> io::Result<ExitStatus> {
        self.kill()?;
        self.wait()
    }
}

#[cfg(any(unix, windows))]
impl ManagedChild for Box<dyn ChildWrapper> {
    fn take_stdout(&mut self) -> Option<ChildStdout> {
        ChildWrapper::stdout(self.as_mut()).take()
    }

    fn take_stderr(&mut self) -> Option<ChildStderr> {
        ChildWrapper::stderr(self.as_mut()).take()
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        ChildWrapper::wait(self.as_mut())
    }

    fn terminate(&mut self) -> io::Result<ExitStatus> {
        ChildWrapper::start_kill(self.as_mut())?;
        ChildWrapper::wait(self.as_mut())
    }
}

fn configure_command(command: &mut Command, spec: &CommandSpec) {
    command
        .args(&spec.args)
        .current_dir(&spec.working_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
}

fn terminate_capture_failure(
    requires_termination: bool,
    child: &mut dyn ManagedChild,
    terminated: &mut bool,
) -> io::Result<()> {
    if requires_termination && !*terminated {
        child.terminate().map(|_status| ())?;
        *terminated = true;
    }
    Ok(())
}

#[cfg(unix)]
fn spawn_contained(spec: &CommandSpec) -> io::Result<Box<dyn ManagedChild>> {
    let mut command =
        CommandWrap::with_new(&spec.executable, |command| configure_command(command, spec));
    command.wrap(ProcessGroup::leader());
    command
        .spawn()
        .map(|child| Box::new(child) as Box<dyn ManagedChild>)
}

#[cfg(windows)]
fn spawn_contained(spec: &CommandSpec) -> io::Result<Box<dyn ManagedChild>> {
    let mut command =
        CommandWrap::with_new(&spec.executable, |command| configure_command(command, spec));
    command.wrap(JobObject);
    command
        .spawn()
        .map(|child| Box::new(child) as Box<dyn ManagedChild>)
}

#[cfg(not(any(unix, windows)))]
fn spawn_contained(spec: &CommandSpec) -> io::Result<Box<dyn ManagedChild>> {
    let mut command = Command::new(&spec.executable);
    configure_command(&mut command, spec);
    command
        .spawn()
        .map(|child| Box::new(child) as Box<dyn ManagedChild>)
}

#[derive(Debug)]
struct CapturedStream {
    bytes: Vec<u8>,
    exceeded: bool,
}

#[derive(Clone, Copy, Debug)]
enum StreamKind {
    Stdout,
    Stderr,
}

enum CaptureEvent {
    ExceededLimit(usize),
    Completed(StreamKind, io::Result<CapturedStream>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaptureState {
    Waiting,
    OutputLimitExceeded,
    ReaderFailed,
    Completed,
}

struct CaptureTracker {
    state: CaptureState,
    limit_bytes: usize,
    stdout: Option<io::Result<CapturedStream>>,
    stderr: Option<io::Result<CapturedStream>>,
}

impl CaptureTracker {
    fn new() -> Self {
        Self {
            state: CaptureState::Waiting,
            limit_bytes: PROCESS_OUTPUT_LIMIT_BYTES,
            stdout: None,
            stderr: None,
        }
    }

    fn observe(&mut self, event: CaptureEvent) {
        match event {
            CaptureEvent::ExceededLimit(limit) => {
                self.limit_bytes = limit;
                self.state = CaptureState::OutputLimitExceeded;
            }
            CaptureEvent::Completed(stream_kind, captured) => {
                let is_reader_failure = captured.is_err();
                let exceeded_limit = captured.as_ref().is_ok_and(|captured| captured.exceeded);
                match stream_kind {
                    StreamKind::Stdout => self.stdout = Some(captured),
                    StreamKind::Stderr => self.stderr = Some(captured),
                }
                if exceeded_limit {
                    self.state = CaptureState::OutputLimitExceeded;
                } else if is_reader_failure && self.state != CaptureState::OutputLimitExceeded {
                    self.state = CaptureState::ReaderFailed;
                } else if self.is_complete() && self.state == CaptureState::Waiting {
                    self.state = CaptureState::Completed;
                }
            }
        }
    }

    fn is_complete(&self) -> bool {
        self.stdout.is_some() && self.stderr.is_some()
    }

    fn requires_contained_termination(&self) -> bool {
        matches!(
            self.state,
            CaptureState::OutputLimitExceeded | CaptureState::ReaderFailed
        )
    }

    fn finish(self) -> io::Result<(CapturedStream, CapturedStream)> {
        if self.state == CaptureState::OutputLimitExceeded {
            return Err(output_limit_error(self.limit_bytes));
        }

        let stdout = self
            .stdout
            .expect("stdout completion checked before capture finalization")?;
        let stderr = self
            .stderr
            .expect("stderr completion checked before capture finalization")?;
        if stdout.exceeded || stderr.exceeded {
            return Err(output_limit_error(self.limit_bytes));
        }
        Ok((stdout, stderr))
    }
}

fn spawn_capture<R>(
    stream: R,
    stream_kind: StreamKind,
    capture_sender: mpsc::Sender<CaptureEvent>,
    limit_bytes: usize,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let captured = capture_stream(stream, &capture_sender, limit_bytes);
        let _ = capture_sender.send(CaptureEvent::Completed(stream_kind, captured));
    })
}

fn capture_stream<R>(
    mut stream: R,
    capture_sender: &mpsc::Sender<CaptureEvent>,
    limit_bytes: usize,
) -> io::Result<CapturedStream>
where
    R: Read,
{
    let mut bytes = Vec::with_capacity(PROCESS_OUTPUT_LIMIT_BYTES.min(limit_bytes));
    let mut exceeded = false;
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Ok(CapturedStream { bytes, exceeded });
        }
        let remaining = limit_bytes.saturating_sub(bytes.len());
        let retained = read.min(remaining);
        bytes.extend_from_slice(&buffer[..retained]);
        if retained < read && !exceeded {
            exceeded = true;
            let _ = capture_sender.send(CaptureEvent::ExceededLimit(limit_bytes));
        }
    }
}

fn join_capture(reader: thread::JoinHandle<()>) -> io::Result<()> {
    reader
        .join()
        .map_err(|_panic_payload| io::Error::other("child output reader panicked"))
}

fn join_capture_readers(
    stdout_reader: thread::JoinHandle<()>,
    stderr_reader: thread::JoinHandle<()>,
) -> io::Result<()> {
    let stdout_result = join_capture(stdout_reader);
    let stderr_result = join_capture(stderr_reader);
    stdout_result.and(stderr_result)
}

#[derive(Debug)]
struct OutputLimitError(usize);

impl std::fmt::Display for OutputLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(OUTPUT_LIMIT_ERROR_MARKER)
    }
}

impl std::error::Error for OutputLimitError {}

fn output_limit_error(limit: usize) -> io::Error {
    io::Error::other(OutputLimitError(limit))
}

#[cfg(test)]
pub(crate) fn process_output_limit_error() -> io::Error {
    output_limit_error(PROCESS_OUTPUT_LIMIT_BYTES)
}

pub(crate) fn output_limit_bytes(error: &io::Error) -> Option<usize> {
    error
        .get_ref()
        .and_then(|source| source.downcast_ref::<OutputLimitError>())
        .map(|source| source.0)
        .or_else(|| {
            (error.kind() == io::ErrorKind::Other && error.to_string() == OUTPUT_LIMIT_ERROR_MARKER)
                .then_some(PROCESS_OUTPUT_LIMIT_BYTES)
        })
}

pub(crate) fn is_process_output_limit_error(error: &io::Error) -> bool {
    output_limit_bytes(error).is_some()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::{
        CaptureEvent, CaptureState, CaptureTracker, CapturedStream, PROCESS_OUTPUT_LIMIT_BYTES,
        StreamKind, capture_stream, is_process_output_limit_error, join_capture_readers,
        process_output_limit_error,
    };

    #[test]
    fn capture_stream_retains_only_the_hard_limit_and_reports_overflow() {
        let (sender, receiver) = mpsc::channel();
        let stream = Cursor::new(vec![b'x'; PROCESS_OUTPUT_LIMIT_BYTES + 1]);

        let captured =
            capture_stream(stream, &sender, PROCESS_OUTPUT_LIMIT_BYTES).expect("capture stream");

        assert!(captured.exceeded);
        assert_eq!(captured.bytes.len(), PROCESS_OUTPUT_LIMIT_BYTES);
        assert!(matches!(
            receiver.try_recv(),
            Ok(CaptureEvent::ExceededLimit(PROCESS_OUTPUT_LIMIT_BYTES))
        ));
    }

    #[test]
    fn process_output_limit_error_has_a_distinct_internal_marker() {
        assert!(is_process_output_limit_error(&process_output_limit_error()));
    }

    #[test]
    fn output_completion_waits_for_both_streams() {
        let mut capture = CaptureTracker::new();
        capture.observe(CaptureEvent::Completed(
            StreamKind::Stdout,
            Ok(CapturedStream {
                bytes: b"stdout".to_vec(),
                exceeded: false,
            }),
        ));

        assert_eq!(capture.state, CaptureState::Waiting);
        assert!(!capture.is_complete());

        capture.observe(CaptureEvent::Completed(
            StreamKind::Stderr,
            Ok(CapturedStream {
                bytes: b"stderr".to_vec(),
                exceeded: false,
            }),
        ));

        assert_eq!(capture.state, CaptureState::Completed);
        assert!(capture.is_complete());
        assert!(!capture.requires_contained_termination());
    }

    #[test]
    fn cap_breach_and_reader_failure_require_contained_termination() {
        let mut cap_breach = CaptureTracker::new();
        cap_breach.observe(CaptureEvent::ExceededLimit(PROCESS_OUTPUT_LIMIT_BYTES));
        assert_eq!(cap_breach.state, CaptureState::OutputLimitExceeded);
        assert!(cap_breach.requires_contained_termination());

        let mut reader_failure = CaptureTracker::new();
        reader_failure.observe(CaptureEvent::Completed(
            StreamKind::Stdout,
            Err(std::io::Error::other("synthetic read failure")),
        ));
        assert_eq!(reader_failure.state, CaptureState::ReaderFailed);
        assert!(reader_failure.requires_contained_termination());
    }

    #[test]
    fn disconnected_capture_receiver_is_directly_observable() {
        let (sender, receiver) = mpsc::channel::<CaptureEvent>();
        drop(sender);

        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn reader_join_waits_for_the_second_reader_after_a_first_reader_panic() {
        let (release_sender, release_receiver) = mpsc::channel();
        let (join_result_sender, join_result_receiver) = mpsc::channel();
        let stdout_reader = thread::spawn(|| panic!("synthetic reader panic"));
        let stderr_reader = thread::spawn(move || {
            let _ = release_receiver.recv();
        });
        let joiner = thread::spawn(move || {
            let _ = join_result_sender.send(join_capture_readers(stdout_reader, stderr_reader));
        });

        let early_result = join_result_receiver
            .recv_timeout(Duration::from_millis(50))
            .ok();
        assert!(
            early_result.is_none(),
            "joining must not return while the second reader is blocked"
        );

        release_sender.send(()).expect("release second reader");
        let error = join_result_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("join must complete after the second reader exits")
            .expect_err("reader panic must remain a capture error");
        joiner.join().expect("joiner thread");

        assert_eq!(error.to_string(), "child output reader panicked");
    }
}
