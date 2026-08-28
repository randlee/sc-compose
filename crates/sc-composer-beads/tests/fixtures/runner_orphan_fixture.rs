//! Helper binary for process-tree containment regression coverage.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("root") => run_root(arguments),
        Some("descendant") => run_descendant(arguments),
        Some("exit") => exit_with(arguments),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected `root <overflow-bytes>`, `descendant`, or `exit <status>`",
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
    let ready_file = next_path(&mut arguments, "missing descendant ready path")?;
    let executable = std::env::current_exe()?;

    Command::new(executable)
        .arg("descendant")
        .arg(state_file)
        .arg(ready_file.clone())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    for _ in 0..200 {
        if ready_file.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !ready_file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "descendant did not become ready",
        ));
    }

    let mut stdout = io::stdout().lock();
    stdout.write_all(&vec![b'x'; overflow_bytes])?;
    stdout.flush()
}

fn run_descendant(mut arguments: impl Iterator<Item = String>) -> io::Result<()> {
    let state_file = next_path(&mut arguments, "missing descendant state path")?;
    let ready_file = next_path(&mut arguments, "missing descendant ready path")?;
    fs::write(ready_file, "ready")?;

    let mut generation = 0_u64;
    loop {
        fs::write(&state_file, generation.to_string())?;
        generation += 1;
        thread::sleep(Duration::from_millis(10));
    }
}

fn next_path(arguments: &mut impl Iterator<Item = String>, message: &str) -> io::Result<PathBuf> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, message))
}
