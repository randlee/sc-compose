//! Helper binary for process-tree containment regression coverage.

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;

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

    let mut ready = [0_u8; 1];
    descendant
        .stderr
        .take()
        .expect("descendant stderr is piped")
        .read_exact(&mut ready)?;
    if ready != [b'R'] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "descendant emitted an invalid ready signal",
        ));
    }

    let mut stdout = io::stdout().lock();
    stdout.write_all(&vec![b'x'; overflow_bytes])?;
    stdout.flush()
}

fn run_descendant(mut arguments: impl Iterator<Item = String>) -> io::Result<()> {
    let state_file = next_path(&mut arguments, "missing descendant state path")?;
    fs::write(state_file, "started")?;
    io::stderr().write_all(b"R")?;
    io::stderr().flush()?;

    // Retain the inherited stdout pipe until process-group/job containment.
    let (_never_send, never_receive) = mpsc::channel::<()>();
    never_receive
        .recv()
        .map_err(|_| io::Error::other("descendant lifetime channel disconnected"))
}

fn next_path(arguments: &mut impl Iterator<Item = String>, message: &str) -> io::Result<PathBuf> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, message))
}
