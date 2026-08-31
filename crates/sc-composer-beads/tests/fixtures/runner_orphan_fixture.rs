//! Helper binary for process-tree containment regression coverage.

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;

const READY_MARKER: &[u8] = b"\x1esc-compose-descendant-ready\x1f";
const MAX_READINESS_DIAGNOSTICS: usize = 1024;

fn main() -> io::Result<()> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("root") => run_root(arguments),
        Some("descendant") => run_descendant(arguments),
        Some("exit") => exit_with(arguments),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected `root <overflow-bytes> <descendant-state>`, `descendant`, or `exit <status>`",
        )),
    }
}

fn exit_with(mut arguments: impl Iterator<Item = String>) -> io::Result<()> {
    let code = arguments
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing exit status"))?
        .parse::<i32>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    std::process::exit(code);
}

fn run_root(mut arguments: impl Iterator<Item = String>) -> io::Result<()> {
    let overflow_bytes = arguments
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing overflow byte count"))?
        .parse::<usize>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let state_file = next_path(&mut arguments, "missing descendant state path")?;
    let executable = std::env::current_exe()?;

    let mut descendant = Command::new(executable)
        .arg("descendant")
        .arg(state_file)
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()?;

    wait_for_ready_marker(
        descendant
            .stderr
            .take()
            .expect("descendant stderr is piped"),
    )?;

    let mut stdout = io::stdout().lock();
    stdout.write_all(&vec![b'x'; overflow_bytes])?;
    stdout.flush()
}

fn run_descendant(mut arguments: impl Iterator<Item = String>) -> io::Result<()> {
    let state_file = next_path(&mut arguments, "missing descendant state path")?;
    fs::write(state_file, "started")?;
    io::stderr().write_all(READY_MARKER)?;
    io::stderr().flush()?;

    // Retain the inherited stdout pipe until process-group/job containment.
    let (_never_send, never_receive) = mpsc::channel::<()>();
    never_receive.recv().map_err(|error| {
        io::Error::other(format!("descendant lifetime channel disconnected: {error}"))
    })
}

fn wait_for_ready_marker(mut reader: impl Read) -> io::Result<()> {
    let mut matched = 0;
    let mut diagnostics = Vec::with_capacity(MAX_READINESS_DIAGNOSTICS);
    let mut buffer = [0_u8; 256];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "descendant closed readiness stream before marker; bytes observed: {:?}",
                    String::from_utf8_lossy(&diagnostics),
                ),
            ));
        }

        for &byte in &buffer[..count] {
            if diagnostics.len() < MAX_READINESS_DIAGNOSTICS {
                diagnostics.push(byte);
            }

            if byte == READY_MARKER[matched] {
                matched += 1;
                if matched == READY_MARKER.len() {
                    return Ok(());
                }
            } else {
                matched = usize::from(byte == READY_MARKER[0]);
            }
        }
    }
}

fn next_path(arguments: &mut impl Iterator<Item = String>, message: &str) -> io::Result<PathBuf> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, message))
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use super::{READY_MARKER, wait_for_ready_marker};

    #[test]
    fn readiness_marker_ignores_diagnostic_prefix() {
        let mut stream = b"macOS runtime diagnostic\n".to_vec();
        stream.extend_from_slice(READY_MARKER);

        wait_for_ready_marker(Cursor::new(stream)).expect("ready marker should be found");
    }

    #[test]
    fn readiness_marker_requires_complete_frame() {
        let error = wait_for_ready_marker(Cursor::new(&READY_MARKER[..READY_MARKER.len() - 1]))
            .expect_err("incomplete marker must not signal readiness");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
