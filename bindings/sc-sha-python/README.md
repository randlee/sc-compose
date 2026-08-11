# sc-sha Python adapter

This package is a thin maturin/PyO3 adapter over the published `sc-sha` Rust
crate. It exposes the same two operations and delegates hashing and manifest
framing to Rust.

File input is explicitly bytes so callers cannot accidentally select a
platform-dependent encoding. Decode text as UTF-8 before passing it.
