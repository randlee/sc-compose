---
id: FIX-249
title: "Path-confinement diagnostics leak path-existence as an oracle"
status: complete
branch: fix/249-path-confinement-existence-oracle
worktree: ../sc-compose-worktrees/fix/249-path-confinement-existence-oracle
target: develop
---

## Root Cause

`crates/sc-composer/src/resolver.rs::canonicalize_with_roots` (lines
117-167) decides which of two differently-worded `ErrResolveNotFound`
diagnostics to return based purely on whether `std::fs::canonicalize`
succeeds — i.e. on filesystem existence — rather than on confinement:

```rust
let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
    ResolveError::new(
        DiagnosticCode::ErrResolveNotFound,
        format!("template path not found: {}", candidate.display()),
        vec![candidate.clone()],
    )
    .with_source(error)
})?;

// ... build `allowed` roots ...

if allowed.iter().any(|allowed_root| canonical.starts_with(allowed_root)) {
    Ok(canonical)
} else {
    Err(ResolveError::new(
        DiagnosticCode::ErrResolveNotFound,
        format!("template path escapes configured roots: {}", candidate.display()),
        vec![candidate],
    ).into())
}
```

Both branches raise the same `DiagnosticCode::ErrResolveNotFound`, but the
`message` text differs: `"template path not found: ..."` if
`canonicalize` fails (path doesn't exist, at any location), versus
`"template path escapes configured roots: ..."` if it exists but resolves
outside every allowed root. A caller who can observe which message came
back (directly via non-JSON stderr, or via the `message` field in JSON
mode) can binary-search absolute paths outside the confined roots and
learn, path by path, whether each one exists on the host filesystem —
e.g. distinguishing `/etc/shadow` (exists, out of root: "escapes
configured roots") from `/etc/does-not-exist-xyz` (doesn't exist, out of
root: "template path not found"). This is a path-existence oracle for
arbitrary absolute paths on the host, reachable through ordinary file-mode
compose requests.

**Reference fix already in this codebase**: `crates/sc-composer/src/include.rs::canonicalize_include`
(lines 199-263) solves the identical problem for include resolution. Its
`Err` branch (candidate does not canonicalize) does not immediately
return "not found" — it first re-checks confinement *lexically*, via a
private `normalize_path` helper (component-based normalization, no
filesystem access, lines 265-282), against both the raw candidate and the
allowed roots:

```rust
Err(error) => {
    let normalized_candidate = normalize_path(candidate);
    let allowed_normalized = allowed_canonical
        .iter()
        .map(|allowed_root| normalize_path(allowed_root))
        .collect::<Vec<_>>();

    if !allowed_normalized
        .iter()
        .any(|allowed_root| normalized_candidate.starts_with(allowed_root))
    {
        return Err(IncludeError::new(
            DiagnosticCode::ErrIncludeEscape,
            format!("include path escapes confinement root: {}", candidate.display()),
            stack.to_vec(),
        ).into());
    }

    Err(IncludeError::new(
        DiagnosticCode::ErrIncludeNotFound,
        format!("include file not found: {}", candidate.display()),
        stack.to_vec(),
    )
    .with_source(error)
    .into())
}
```

Because confinement is checked lexically first, a nonexistent path
outside the allowed roots gets the *same* "escapes confinement root"
result as an existing path outside the allowed roots — existence is no
longer observable for out-of-root paths. Only a path that is lexically
inside an allowed root but doesn't exist reaches "not found". This sprint
ports that exact pattern to `canonicalize_with_roots`.

## Exact Target

`crates/sc-composer/src/resolver.rs::canonicalize_with_roots` — restructure
to match on `std::fs::canonicalize(&candidate)` the same way
`canonicalize_include` does, instead of using `?` to bail out of the
function on failure before any confinement check runs:

```rust
pub(crate) fn canonicalize_with_roots(
    path: impl AsRef<Path>,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
) -> Result<PathBuf, ComposeError> {
    let path = path.as_ref();
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };

    let mut allowed = Vec::with_capacity(allowed_roots.len() + 1);
    allowed.push(std::fs::canonicalize(root).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!("failed to canonicalize root: {}", root.display()),
        )
        .with_source(error)
    })?);
    allowed.extend(
        allowed_roots
            .iter()
            .map(|root| root.as_path().to_path_buf()),
    );

    match std::fs::canonicalize(&candidate) {
        Ok(canonical) => {
            if allowed
                .iter()
                .any(|allowed_root| canonical.starts_with(allowed_root))
            {
                Ok(canonical)
            } else {
                Err(ResolveError::new(
                    DiagnosticCode::ErrResolveNotFound,
                    format!("template path escapes configured roots: {}", candidate.display()),
                    vec![candidate],
                )
                .into())
            }
        }
        Err(error) => {
            let normalized_candidate = crate::include::normalize_path(&candidate);
            let allowed_normalized = allowed
                .iter()
                .map(|allowed_root| crate::include::normalize_path(allowed_root))
                .collect::<Vec<_>>();

            if !allowed_normalized
                .iter()
                .any(|allowed_root| normalized_candidate.starts_with(allowed_root))
            {
                return Err(ResolveError::new(
                    DiagnosticCode::ErrResolveNotFound,
                    format!("template path escapes configured roots: {}", candidate.display()),
                    vec![candidate],
                )
                .into());
            }

            Err(ResolveError::new(
                DiagnosticCode::ErrResolveNotFound,
                format!("template path not found: {}", candidate.display()),
                vec![candidate],
            )
            .with_source(error)
            .into())
        }
    }
}
```

The root-canonicalization step (building `allowed`) moves earlier,
unchanged in logic, only reordered so it runs before the candidate match
(it was already unconditionally required in both outcomes; moving it
doesn't change its own error behavior — `ErrConfigParse` for an
uncanonicalizable `root` is unaffected and out of scope). The two
`ErrResolveNotFound` construction sites reachable for an out-of-root
path — the `Ok`-branch confinement miss and the `Err`-branch lexical
confinement miss — now emit **byte-identical** message text
(`"template path escapes configured roots: {candidate}"`), regardless of
whether the path exists. `"template path not found"` is now reachable
only when the candidate is lexically within an allowed root but does not
exist — the case that was always intended to mean "not found" and never
carried oracle value in the first place, since the caller already knows
the path they asked to confine is one they're allowed to see the
existence of.

`crates/sc-composer/src/include.rs`'s private `normalize_path` (line 265)
becomes `pub(crate)` so `resolver.rs` can reuse it verbatim instead of
duplicating the same lexical-normalization logic — both `include` and
`resolver` are already `pub mod` siblings under the same crate root
(`lib.rs:22,31`), so this is a visibility-only change with no new public
API surface.

## This Sprint Does NOT Change

- `DiagnosticCode::ErrResolveNotFound`'s value or any other diagnostic
  code — both messages continue to use the same code they always did.
- `canonicalize_include` / `include.rs`'s own confinement logic — it is
  already correct and is only the *source* of the reused `normalize_path`
  helper (made `pub(crate)`, body unchanged).
- The root-canonicalization failure path (`ErrConfigParse` if `root`
  itself can't canonicalize) — logic and message unchanged, only its
  position in the function moves earlier.
- `resolve_profile_impl` or its separate `canonicalize` call at
  `resolver.rs:191-198` (per-candidate profile matching) — a different
  code path entirely, out of scope, per FIX-251's sprint doc which
  already establishes this boundary for adjacent work in this file.
- Any `DiagnosticCode` variant additions — unlike FIX-251, this fix needs
  no new variants; both messages already share
  `DiagnosticCode::ErrResolveNotFound`, so only message-text reachability
  changes.

## Required Test Matrix

New unit tests in `crates/sc-composer/src/resolver.rs`'s existing
`mod tests` (using the established `temp_root` / `write_file` helpers,
lines 501+), asserting on `canonicalize_with_roots` directly (it is
`pub(crate)`, already unit-testable from this module):

(a) **Red-baseline regression test (mandatory `#[ignore]`d test — see
Process section)**: build two `canonicalize_with_roots` calls against the
same confined root and the same absolute candidate outside the allowed
root. Create the candidate, capture the existing-path error, remove it,
then capture the missing-path error. Assert both calls return
`ComposeError::Resolve` with `DiagnosticCode::ErrResolveNotFound` **and
identical message text**. **Before the fix, this test fails** — the
nonexistent case's message is `"template path not found: ..."` while the
existing case's message is `"template path escapes configured roots:
..."` (a normal in-process string-equality assertion failure, not a
crash — standard, non-crash-mode verification applies). Using the same
candidate keeps the assertion focused on the existence oracle rather than
on unrelated path text.

(b) A candidate path that is lexically inside the allowed root but does
not exist on disk still returns `ErrResolveNotFound` with the
`"template path not found: ..."` message (confirms the legitimate
not-found case is preserved for in-bounds paths).

(c) A candidate path that is inside the allowed root and does exist
resolves successfully (`Ok`) — confirms the happy path is unaffected
(reuse/extend an existing passing case if the current test module already
covers this via `resolve_profile_impl`/file-mode paths; add one directly
against `canonicalize_with_roots` if not already covered at that level).

(d) A candidate path that escapes via `..` traversal (lexically outside
the root, e.g. `root.join("../../etc/passwd")`) and does not exist
returns the same `"escapes configured roots"` message as a `..`-escaping
path that does exist — confirms the lexical normalization in the new
`Err` branch actually resolves `..` components rather than just
string-prefix-matching the unnormalized candidate.

## Mandatory Process (two-commit red -> green, standing requirement, confirmed clean 3/3 on FIX-245/244/247)

1. **First commit**: land test (a) above as `#[ignore]`d in
   `crates/sc-composer/src/resolver.rs`'s test module. Team-lead
   independently confirms it genuinely fails before any fix code is
   written (`cargo test -p sc-composer -- --ignored <test_name>`,
   standard assertion-failure verification, not crash-mode).
2. **Second commit**: land the `canonicalize_with_roots` restructure
   above, make `include.rs::normalize_path` `pub(crate)`, add tests
   (b)-(d), and remove the single `#[ignore]` line from test (a). No
   other test-logic changes in this commit. Team-lead independently
   re-runs the same command from step 1 and confirms it now passes.
3. Sprint-doc closeout narrative must state accurate, verifiable
   provenance — the regression tests are created fresh on this branch,
   never described as promoted from elsewhere.

## Acceptance Criteria

- `cargo test --workspace` passes, including the now-unignored test (a)
  and new tests (b)-(d).
- An out-of-root candidate path returns the identical `ErrResolveNotFound`
  message (`"template path escapes configured roots: {path}"`) whether or
  not the path exists on disk.
- An in-root candidate path that doesn't exist still returns
  `"template path not found: {path}"`.
- In-root, existing candidates continue to resolve successfully.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #249 can be closed referencing the merged PR.

## Closeout Evidence

Status: **complete**.

- Red regression commit: `7d92880` (`test: reproduce path existence oracle`).
  The ignored baseline failed with different messages for an existing versus
  missing out-of-root candidate.
- Green implementation commit: `6aa2912` (`fix: close path existence
  oracle`). The resolver now performs lexical confinement checks when
  canonicalization fails, reusing `include::normalize_path`; the raw root
  spelling is also retained so macOS `/var` and `/private/var` aliases do not
  misclassify an in-root missing candidate as an escape.
- Focused resolver tests: 9 passed.
- Workspace tests: `cargo test --workspace` passed.
- Quality gates: `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo fmt --all --check`, and `git diff --check` passed.

The regression test was created fresh on this branch and is now unignored;
it was not promoted from another branch or previously closed issue.

## References

- GitHub issue #249
- `crates/sc-composer/src/resolver.rs` (`canonicalize_with_roots`, lines
  117-167)
- `crates/sc-composer/src/include.rs` (`canonicalize_include`, lines
  199-263; `normalize_path`, lines 265-282) — reference confinement
  pattern this sprint ports
- `crates/sc-composer/src/lib.rs` (module declarations, lines 22, 31)
