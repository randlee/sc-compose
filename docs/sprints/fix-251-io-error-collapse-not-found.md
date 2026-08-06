---
id: FIX-251
title: "Distinguish PermissionDenied / IsADirectory / FilesystemLoop from NotFound in include and resolve I/O errors"
status: complete
branch: fix/251-io-error-collapse-not-found
worktree: ../sc-compose-worktrees/fix/251-io-error-collapse-not-found
target: develop
---

## Root Cause

Two separate I/O-error sites in `crates/sc-composer/src` each match on
`std::io::Error` but only special-case `ErrorKind::InvalidData`, collapsing
every other `io::ErrorKind` — including `NotFound`, `PermissionDenied`,
`IsADirectory`, and symlink loops (`FilesystemLoop`) — into a single "not
found" diagnostic code.

**Site 1 — `crates/sc-composer/src/include.rs:131-144`** (`expand_file`'s
`read_to_string` error handling):

```rust
let raw = std::fs::read_to_string(path).map_err(|error| {
    let (code, message) = if error.kind() == std::io::ErrorKind::InvalidData {
        (
            DiagnosticCode::ErrConfigRead,
            format!("template file is not valid UTF-8: {}", path.display()),
        )
    } else {
        (
            DiagnosticCode::ErrIncludeNotFound,
            format!("include file not found: {}", path.display()),
        )
    };
    IncludeError::new(code, message, stack.clone()).with_source(error)
})?;
```

Any `error.kind()` other than `InvalidData` — `PermissionDenied`,
`IsADirectory` (e.g. an include directive pointing at a directory), or a
symlink loop reported as `FilesystemLoop` — falls into the `else` branch and
is reported as `ErrIncludeNotFound` with the message "include file not
found", even though the file exists and the real problem is permissions,
directory-vs-file, or a cyclical symlink.

**Site 2 — `crates/sc-composer/src/resolver.rs:128-135`**
(`canonicalize_with_roots`'s `std::fs::canonicalize` error handling):

```rust
let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
    ResolveError::new(
        DiagnosticCode::ErrResolveNotFound,
        format!("template path not found: {}", candidate.display()),
        vec![candidate.clone()],
    )
    .with_source(error)
})?;
```

This site does not inspect `error.kind()` at all — every `canonicalize`
failure (`NotFound`, `PermissionDenied`, `FilesystemLoop`, etc.) is reported
as `ErrResolveNotFound` with a "template path not found" message.

Confirmed via `crates/sc-composer/src/diagnostics.rs:39-93`: the
`DiagnosticCode` enum currently has no variant distinguishing permission
errors, directory-vs-file errors, or symlink loops from a genuine
not-found — `ErrIncludeNotFound` (line 45) and `ErrResolveNotFound` (line
39) are each used for all of these cases today. `ErrConfigRead` (line 89)
already exists and is reused here as the natural home for the
`InvalidData` case at Site 1 (it is not otherwise touched by this sprint).

The Rust toolchain floor for this workspace is `1.94.1`
(`docs/cross-platform-guidelines.md`), well past the `1.83.0` stabilization
of the `io_error_more` variants — `std::io::ErrorKind::IsADirectory`,
`NotADirectory`, `PermissionDenied` (already stable), and `FilesystemLoop`
are all available as stable, cross-platform `ErrorKind` matches (the
standard library maps the platform-specific errno, e.g. `ELOOP` on Unix, to
`FilesystemLoop` internally) — no `cfg(unix)` gating is required to match on
them.

## Exact Target

Add two new `DiagnosticCode` variants and match explicitly on `io::ErrorKind`
at both sites, keeping the existing `InvalidData` -> `ErrConfigRead` handling
at Site 1 unchanged and falling back to the current not-found code only for
`ErrorKind::NotFound` (and any other kind not explicitly enumerated, to stay
forward-compatible with new `ErrorKind` variants the standard library may add
later).

In `crates/sc-composer/src/diagnostics.rs`, add alongside `ErrIncludeNotFound`
/ `ErrResolveNotFound`:

```rust
/// An include target or resolved template path exists but could not be
/// read due to filesystem permissions.
ErrIncludePermissionDenied,
/// An include directive resolved to a directory instead of a file.
ErrIncludeIsADirectory,
/// An include chain traversed a filesystem symlink loop.
ErrIncludeFilesystemLoop,
```

and their `&'static str` mappings, following the existing pattern at
`diagnostics.rs:181-201`:

```rust
Self::ErrIncludePermissionDenied => "ERR_INCLUDE_PERMISSION_DENIED",
Self::ErrIncludeIsADirectory => "ERR_INCLUDE_IS_A_DIRECTORY",
Self::ErrIncludeFilesystemLoop => "ERR_INCLUDE_FILESYSTEM_LOOP",
```

In `crates/sc-composer/src/include.rs:131-144`, replace the two-way match
with an explicit `match error.kind()`:

```rust
let raw = std::fs::read_to_string(path).map_err(|error| {
    let (code, message) = match error.kind() {
        std::io::ErrorKind::InvalidData => (
            DiagnosticCode::ErrConfigRead,
            format!("template file is not valid UTF-8: {}", path.display()),
        ),
        std::io::ErrorKind::PermissionDenied => (
            DiagnosticCode::ErrIncludePermissionDenied,
            format!("permission denied reading include file: {}", path.display()),
        ),
        std::io::ErrorKind::IsADirectory => (
            DiagnosticCode::ErrIncludeIsADirectory,
            format!("include target is a directory, not a file: {}", path.display()),
        ),
        std::io::ErrorKind::FilesystemLoop => (
            DiagnosticCode::ErrIncludeFilesystemLoop,
            format!("include target is a filesystem symlink loop: {}", path.display()),
        ),
        _ => (
            DiagnosticCode::ErrIncludeNotFound,
            format!("include file not found: {}", path.display()),
        ),
    };
    IncludeError::new(code, message, stack.clone()).with_source(error)
})?;
```

In `crates/sc-composer/src/resolver.rs:128-135`, add the same explicit match
(reusing the same three new `DiagnosticCode` variants — this sprint does not
introduce a separate `ErrResolve*` family, since `ResolveError::new` already
accepts any `DiagnosticCode`, confirmed by its existing use of
`DiagnosticCode::ErrConfigParse` at `resolver.rs:140`):

```rust
let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
    let (code, message) = match error.kind() {
        std::io::ErrorKind::PermissionDenied => (
            DiagnosticCode::ErrIncludePermissionDenied,
            format!("permission denied reading template path: {}", candidate.display()),
        ),
        std::io::ErrorKind::FilesystemLoop => (
            DiagnosticCode::ErrIncludeFilesystemLoop,
            format!("template path is a filesystem symlink loop: {}", candidate.display()),
        ),
        _ => (
            DiagnosticCode::ErrResolveNotFound,
            format!("template path not found: {}", candidate.display()),
        ),
    };
    ResolveError::new(code, message, vec![candidate.clone()]).with_source(error)
})?;
```

Note: `std::fs::canonicalize` on a directory target succeeds (canonicalizing
a directory path is not itself an error), so `IsADirectory` is not a
meaningful case at Site 2 and is intentionally not matched there — the
directory-vs-file distinction only applies to Site 1's `read_to_string` call.

## This Sprint Does NOT Change

- The existing `InvalidData` -> `ErrConfigRead` handling at Site 1 — untouched
  wording and code.
- `ErrIncludeNotFound` / `ErrResolveNotFound`'s meaning for genuine
  `ErrorKind::NotFound` cases, or for any future `io::ErrorKind` variant not
  explicitly enumerated above — the `_` fallback arm preserves current
  behavior for everything not listed.
- No new `DiagnosticCode` variant for `NotADirectory` — every current caller
  of these two sites passes a file path expected to be a regular file, and
  `NotADirectory` cannot occur from `read_to_string`/`canonicalize` on a path
  string (it only arises when a *parent component* is not a directory, which
  the standard library already reports as `NotFound` on most platforms for
  these APIs) — not worth a speculative diagnostic code without a reachable
  repro.
- No change to `IncludeError`/`ResolveError`'s constructors, `with_source`,
  or any other error-type plumbing beyond passing a different
  `DiagnosticCode`/message pair.
- No CLI-surface change — these are library-level diagnostic codes, consumed
  the same way `ErrIncludeNotFound`/`ErrResolveNotFound` already are today.

## Required Test Matrix

All new tests live in `crates/sc-composer/src/include.rs`'s and
`crates/sc-composer/src/resolver.rs`'s existing `#[cfg(test)] mod tests`
modules, following the `temp_root`/`write_file` helper patterns already used
by `depth_overflow_is_rejected` and sibling tests (`include.rs:355-380`).

(a) **Red-baseline regression test**: an include directive pointing at a
*directory* (not a file) under a temp confined root returns
`Err(DiagnosticCode::ErrIncludeIsADirectory)`, not
`ErrIncludeNotFound`. Before the fix, this currently asserts
`ErrIncludeIsADirectory` and genuinely fails (the current code returns
`ErrIncludeNotFound` instead) — a normal in-process assertion failure, no
process-abort verification needed (unlike FIX-247).

(b) A permission-denied file (create a file, `chmod 0o000` it via
`std::os::unix::fs::PermissionsExt`, `#[cfg(unix)]`-gated since Windows ACL
permission removal is not portably testable the same way) returns
`Err(DiagnosticCode::ErrIncludePermissionDenied)` at Site 1.

(c) A permission-denied file passed as the top-level `--file`/resolve target
(via `canonicalize_with_roots` directly, or `resolve_template_path`) returns
`Err(DiagnosticCode::ErrIncludePermissionDenied)` at Site 2. `#[cfg(unix)]`
for the same reason as (b).

(d) A symlink loop (`#[cfg(unix)]`, `std::os::unix::fs::symlink` — create
`a -> b`, `b -> a`) included from a template returns
`Err(DiagnosticCode::ErrIncludeFilesystemLoop)` at Site 1.

(e) A genuinely missing file (the pre-existing case) still returns
`Err(DiagnosticCode::ErrIncludeNotFound)` / `ErrResolveNotFound` unchanged —
confirms the fallback arm preserves current behavior.

(f) The 4 pre-existing `include.rs` unit tests
(`expands_successful_include_chain`, `missing_include_reports_not_found`,
`cycle_detection_is_rejected`, `depth_overflow_is_rejected`) pass unmodified.

## Mandatory Process (two-commit red -> green, standing requirement)

Confirmed clean 3/3 on FIX-245, FIX-244, and FIX-247. This sprint's red
baseline is a normal in-process assertion failure (test (a), the
directory-as-include-target case — it does not require `#[cfg(unix)]` and is
the simplest deterministic red case), not a process abort like FIX-247's
unique case.

1. **First commit**: land test (a) as `#[ignore]`d in `include.rs`'s test
   module. Team-lead independently confirms it genuinely fails (asserts
   `ErrIncludeIsADirectory`, actual result is `ErrIncludeNotFound`) via
   `cargo test --workspace -p sc-composer -- --ignored <test_name>` before
   any fix code is written.
2. **Second commit**: land the `DiagnosticCode` variants, both match-arm
   fixes, and tests (b)-(e), and remove the single `#[ignore]` line from
   test (a). No other test-logic changes in this commit. Team-lead
   independently re-runs `cargo fmt --all --check`,
   `cargo test --workspace`, and
   `cargo clippy --all-targets --all-features -- -D warnings` directly.
3. Sprint-doc closeout narrative must state accurate, verifiable
   provenance — all tests created fresh on this branch.

## Acceptance Criteria

- `cargo test --workspace` passes, including the now-unignored test (a) and
  new tests (b)-(e).
- A directory passed as an include target or resolve target returns a
  distinct `ErrIncludeIsADirectory`/existing-not-found diagnostic instead of
  a misleading "not found".
- A permission-denied file at either site returns
  `ErrIncludePermissionDenied` instead of "not found".
- A symlink loop in an include chain returns `ErrIncludeFilesystemLoop`
  instead of "not found".
- Genuinely missing files still return the existing not-found codes
  unchanged.
- All 4 pre-existing `include.rs` unit tests still pass unmodified.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #251 can be closed referencing the merged PR.

## Closeout Evidence

All regression tests were created fresh on this branch and were not promoted
from another worktree.

- `96dd4c5` is the red baseline. The ignored directory-target test failed
  normally with `ERR_INCLUDE_NOT_FOUND` before the fix.
- `993251f` adds the three stable diagnostic codes, explicit permission and
  directory handling at the include read site, permission and loop handling
  at resolver canonicalization, and the required green tests.
- Include tests: PASS (15/15), including directory, permission-denied,
  symlink-loop, and genuine missing-file cases.
- Resolver tests: PASS (7/7), including permission-denied and genuine
  missing-file cases.
- Workspace tests: PASS (`cargo test --workspace`).
- Clippy: PASS (`cargo clippy --all-targets --all-features -- -D warnings`).
- Formatting and whitespace checks: PASS (`cargo fmt --all --check` and
  `git diff --check`).

### QA-1 Follow-up (PR #262)

Quality review identified a deterministic Windows failure: reading a directory
with `std::fs::read_to_string` can report `PermissionDenied` rather than
`IsADirectory`. Commit `5f4b05e` closes that gap by making the shared
classifier check `Path::is_dir()` before inspecting the platform-specific
error kind. The same classifier now serves `expand_file`,
`canonicalize_include`, and `canonicalize_with_roots`, preserving each
call site's existing diagnostic code and message text.

- QA-1 implementation commit: `5f4b05e`.
- Focused directory and resolver permission tests: PASS.
- Full workspace tests after QA-1: PASS.
- Clippy, formatting, and whitespace checks after QA-1: PASS.
- The required Windows behavior is covered by the platform-independent
  `is_dir()` branch; no Windows-only test or logic change was needed.

### Implementation notes

The sprint plan incorrectly describes `std::io::ErrorKind::FilesystemLoop`
as stable. Rust 1.94.1 still gates it behind `io_error_more`, so the fix uses
an internal platform-gated `raw_os_error()` helper instead of an unstable API.
The symlink-loop mapping is also applied in `canonicalize_include`, where an
include loop fails before `expand_file` reaches `read_to_string`; this is
required for the sprint's stated symlink-loop acceptance test. Confinement
escape handling and genuine not-found fallback remain unchanged.

## References

- GitHub issue #251
- `crates/sc-composer/src/include.rs` (`expand_file`, lines 131-144;
  existing test module)
- `crates/sc-composer/src/resolver.rs` (`canonicalize_with_roots`, lines
  117-167)
- `crates/sc-composer/src/diagnostics.rs` (`DiagnosticCode` enum, lines
  39-93 and its `&'static str` mapping, lines 181-201)
- `docs/cross-platform-guidelines.md` (Rust toolchain floor 1.94.1,
  `io_error_more` remains unstable on the toolchain floor)
