# CLI installation and troubleshooting

The skill requires `cargo` and `git`.

## Check first

```bash
which cargo && cargo --version
which git && git --version
```

If both commands resolve, use the reported binaries and continue. Do not
replace them with an alternate toolchain without recording that choice.

## Find an existing install

```bash
for p in "$HOME/.cargo/bin/cargo" "/opt/homebrew/bin/cargo" "/usr/local/bin/cargo"; do
  [ -x "$p" ] && echo "Found cargo at: $p" && break
done
for p in "$HOME/.local/bin/git" "/opt/homebrew/bin/git" "/usr/local/bin/git"; do
  [ -x "$p" ] && echo "Found git at: $p" && break
done
```

Use the full path if the binary is installed but absent from the agent PATH.

## Install or repair

- Install Rust through the official `rustup` workflow and ensure the stable
  toolchain is available to the shell running the campaign.
- Install Git through the operating system package manager or Xcode command
  line tools on macOS.
- Do not modify repository toolchain files as part of dependency repair.

## Validation

From the sc-compose worktree, confirm:

```bash
cargo metadata --no-deps --format-version 1
git rev-parse --show-toplevel
```

If either command fails, stop the campaign and report the exact command and
error instead of generating partial fuzz results.
