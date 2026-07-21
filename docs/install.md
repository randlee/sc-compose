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

Bundled examples are included in the package.

### Any Platform — crates.io

```bash
cargo install sc-compose
```

`cargo install` ships the binary only. Bundled examples are NOT included. Set
`SC_COMPOSE_DATA_DIR` to point at a manual copy of the examples root if you
want `examples list` and `examples <name>`.

Or build without installing:

```bash
cargo build --release -p sc-compose
./target/release/sc-compose --help
```

### Any Platform — PyPI

```bash
pip install sc-compose
```

Pre-built wheels for macOS, Linux, Windows (Python 3.11+). Pre-release builds
available on TestPyPI:

```bash
pip install -i https://test.pypi.org/simple/ sc-compose
```

### Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
sc-composer = "1.3.0"
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
