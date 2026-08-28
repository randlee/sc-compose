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

/// Maximum number of bytes retained from each `bd` output stream.
///
/// The runner terminates its contained process tree when this limit is exceeded,
/// rather than allowing an untrusted subprocess to consume unbounded memory.
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
    /// On a per-stream output-cap breach, the production runner terminates the
    /// contained process tree before returning an error. Unix uses a dedicated
    /// process group and Windows uses a Job Object. Other platforms retain the
    /// direct-child behavior because they have no equivalent containment
    /// primitive in this implementation.
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
        let mut child = spawn_contained(spec)?;
        let stdout = child
            .take_stdout()
            .ok_or_else(|| io::Error::other("child stdout was not captured"))?;
        let stderr = child
            .take_stderr()
            .ok_or_else(|| io::Error::other("child stderr was not captured"))?;
        let (capture_sender, capture_receiver) = mpsc::channel();
        let stdout_reader = spawn_capture(stdout, StreamKind::Stdout, capture_sender.clone());
        let stderr_reader = spawn_capture(stderr, StreamKind::Stderr, capture_sender);

        let mut exceeded_limit = false;
        let mut terminated_for_limit = false;
        let mut status = None;
        let mut stdout = None;
        let mut stderr = None;
        loop {
            if status.is_none() {
                status = child.try_wait()?;
            }

            match capture_receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(CaptureEvent::ExceededLimit) => {
                    exceeded_limit = true;
                    if !terminated_for_limit {
                        status = Some(child.terminate()?);
                        terminated_for_limit = true;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::other(
                        "child output capture closed before both streams completed",
                    ));
                }
                Ok(CaptureEvent::Completed(StreamKind::Stdout, captured)) => {
                    stdout = Some(captured?);
                }
                Ok(CaptureEvent::Completed(StreamKind::Stderr, captured)) => {
                    stderr = Some(captured?);
                }
            }

            if status.is_some() && stdout.is_some() && stderr.is_some() {
                break;
            }
        }
        join_capture(stdout_reader)?;
        join_capture(stderr_reader)?;
        let stdout = stdout.expect("stdout completion checked before loop exit");
        let stderr = stderr.expect("stderr completion checked before loop exit");
        exceeded_limit |= stdout.exceeded || stderr.exceeded;
        if exceeded_limit {
            return Err(process_output_limit_error());
        }
        Ok(ProcessOutput {
            exit_status: status
                .expect("child status checked before loop exit")
                .code(),
            stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
            stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
            elapsed: started.elapsed(),
        })
    }
}

trait ManagedChild {
    fn take_stdout(&mut self) -> Option<ChildStdout>;
    fn take_stderr(&mut self) -> Option<ChildStderr>;
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>>;
    fn terminate(&mut self) -> io::Result<ExitStatus>;
}

impl ManagedChild for std::process::Child {
    fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.stdout.take()
    }

    fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.stderr.take()
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        std::process::Child::try_wait(self)
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

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        ChildWrapper::try_wait(self.as_mut())
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
    ExceededLimit,
    Completed(StreamKind, io::Result<CapturedStream>),
}

fn spawn_capture<R>(
    stream: R,
    stream_kind: StreamKind,
    capture_sender: mpsc::Sender<CaptureEvent>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let captured = capture_stream(stream, &capture_sender);
        let _ = capture_sender.send(CaptureEvent::Completed(stream_kind, captured));
    })
}

fn capture_stream<R>(
    mut stream: R,
    capture_sender: &mpsc::Sender<CaptureEvent>,
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
            let _ = capture_sender.send(CaptureEvent::ExceededLimit);
        }
    }
}

fn join_capture(reader: thread::JoinHandle<()>) -> io::Result<()> {
    reader
        .join()
        .map_err(|_panic_payload| io::Error::other("child output reader panicked"))
}

pub(crate) fn process_output_limit_error() -> io::Error {
    io::Error::other(OUTPUT_LIMIT_ERROR_MARKER)
}

pub(crate) fn is_process_output_limit_error(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::Other && error.to_string() == OUTPUT_LIMIT_ERROR_MARKER
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::mpsc;

    use super::{
        CaptureEvent, PROCESS_OUTPUT_LIMIT_BYTES, capture_stream, is_process_output_limit_error,
        process_output_limit_error,
    };

    #[test]
    fn capture_stream_retains_only_the_hard_limit_and_reports_overflow() {
        let (sender, receiver) = mpsc::channel();
        let stream = Cursor::new(vec![b'x'; PROCESS_OUTPUT_LIMIT_BYTES + 1]);

        let captured = capture_stream(stream, &sender).expect("capture stream");

        assert!(captured.exceeded);
        assert_eq!(captured.bytes.len(), PROCESS_OUTPUT_LIMIT_BYTES);
        assert!(matches!(
            receiver.try_recv(),
            Ok(CaptureEvent::ExceededLimit)
        ));
    }

    #[test]
    fn process_output_limit_error_has_a_distinct_internal_marker() {
        assert!(is_process_output_limit_error(&process_output_limit_error()));
    }
}
