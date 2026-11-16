---
layout: default
title: Install — sc-compose
description: Install sc-compose on macOS, Windows, Linux, or via pip and cargo.
---

# Install sc-compose

## Choose Your Platform

### macOS — Homebrew

```bash
brew install randlee/tap/sc-compose
```

Bundled examples are installed to `$(brew --prefix)/share/sc-compose/examples/`
and discovered automatically.

### Windows — Winget

```powershell
winget install randlee.sc-compose
```

Winget example discovery has not been qualified. If examples are missing,
set `SC_COMPOSE_DATA_DIR` to a directory containing `examples/`.

### Pinned versions (CI and production)

For a pinned version, install the **GitHub Release binary**. It is the supported
install contract for a tagged version: each release publishes per-platform
archives (`sc-compose_<version>_<target>.tar.gz` or `.zip`) plus
`checksums.txt`, and those assets stay valid after later releases. Verify the
download against `checksums.txt`. Release binaries for `1.7.0` keep working
when `1.8.0` ships, so consumers on different pins do not affect each other.

### Any Platform — crates.io

```bash
cargo install sc-compose
```

`cargo install` ships the binary only. Bundled examples are NOT included. Set
`SC_COMPOSE_DATA_DIR` to point at a manual copy of the examples root if you
want `examples list` and `examples <name>`.

For a versioned source install, use
`cargo install sc-compose --version '=X.Y.Z' --locked`. Cargo uses the
published lockfile when available; without `--locked`, Cargo can resolve newer
compatible dependencies. A source build still requires a compatible toolchain
and available dependencies. See the [Cargo install documentation](https://doc.rust-lang.org/cargo/commands/cargo-install.html#dealing-with-the-lockfile).

The workspace crates (`sc-sha`, `sc-composer`, `sc-composer-beads`, and
`sc-compose`) are released together at the same version and are not designed
to be mixed across versions. Starting with 1.9.0, their workspace dependency
requirements pin sibling crates to that exact version.

Or build without installing:

```bash
cargo build --release -p sc-compose
./target/release/sc-compose --help
```

### Any Platform — PyPI

```bash
pip install sc-compose
```

Pre-built wheels for macOS, Linux, Windows (Python 3.9+; Windows ARM64 requires 3.11+). Pre-release builds
available on TestPyPI:

```bash
pip install -i https://test.pypi.org/simple/ sc-compose
```

### Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
sc-composer = "1.9.0"
```

The crate re-exports `compose`, `compose_with_observer`,
`validate_with_observer`, `resolve_profile_with_observer`,
`frontmatter_init`, `init_workspace`, plus request/result types and the
diagnostic envelope.

## Configuration

| Variable | Purpose |
|----------|---------|
| `SC_COMPOSE_DATA_DIR` | Override bundled examples location (useful for `cargo install` and CI) |
| `SC_COMPOSE_TEMPLATE_DIR` | Override user template store location |

## Next Steps

- [Examples]({{ '/examples' | relative_url }}) — try the bundled starter templates
- [GitHub README](https://github.com/randlee/sc-compose) — full CLI reference and feature docs
- [Documentation](https://github.com/randlee/sc-compose/tree/develop/docs) — requirements, architecture, error codes
