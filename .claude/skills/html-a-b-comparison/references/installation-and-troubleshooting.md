# CLI installation and troubleshooting

The skill requires two command-line tools: `sc-compose` and `python3`.

## Check first

```bash
which sc-compose && sc-compose --version
which python3 && python3 --version
```

If both commands resolve, use those binaries. The skill's Step 1 also checks
common full-path locations when the current shell `PATH` is incomplete.

## Find an existing install

On macOS and Linux, check these locations before installing:

```bash
for name in sc-compose python3; do
  for path in \
    "$HOME/.local/bin/$name" \
    "$HOME/.cargo/bin/$name" \
    "/opt/homebrew/bin/$name" \
    "/usr/local/bin/$name" \
    "/usr/bin/$name"; do
    [ -x "$path" ] && echo "Found $name at $path"
  done
done
```

On Windows, use `Get-Command sc-compose, python3` in PowerShell and check for
`sc-compose.exe` and `python.exe` in the reported locations.

## Install

- **macOS:** Install Python from python.org or Homebrew (`brew install
  python`). Build the checked-out CLI with `cargo build --release` and use
  `target/release/sc-compose`, or install it with `cargo install --path
  crates/sc-compose --locked`.
- **Linux:** Install Python 3 with the distribution package manager. Install
  Rust with rustup, then build or install `sc-compose` using the commands
  above.
- **Windows:** Install Python 3 from python.org or `winget`, install Rust
  with rustup, and run the equivalent Cargo build from PowerShell.

## Minimum version

- `sc-compose`: the version built from the checked-out repository (confirm it
  with `sc-compose --version`).
- `python3`: Python 3.9 or newer. The workflow uses only the standard library;
  no pip package is required.

## PATH troubleshooting

If an executable exists but is not found, use its absolute path in
`SC_COMPOSE_BIN` or `PYTHON3_BIN`, or add its containing directory to `PATH`
before running Step 1. Claude Code shells may not load interactive-shell
startup files, so verify the exported `PATH` in the same shell that runs the
render command. Avoid changing `HOME` or `USERPROFILE` to repair PATH.

## Validation

From the repository or worktree, run:

```bash
"$SC_COMPOSE_BIN" --version
"$PYTHON3_BIN" --version
"$SC_COMPOSE_BIN" render --help
"$PYTHON3_BIN" -c 'import base64, tempfile; print(tempfile.gettempdir())'
```

Proceed only when all commands succeed.

## Known issues

- A Cargo-installed `sc-compose` may be older than the checked-out source;
  prefer the worktree build when testing a new template feature.
- A Python virtual environment can hide the system `python3`; use the
  explicitly resolved interpreter from Step 1.
- On Windows, `start` is a shell builtin rather than a standalone executable;
  use `cmd.exe /c start "" <path>` or PowerShell `Start-Process`.
- If an output path is supplied by a caller, resolve it and verify that it is
  beneath the approved scratchpad or repository root before writing. Never
  follow a symlink or `..` segment outside that root.
