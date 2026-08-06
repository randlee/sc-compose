---
id: FIX-247
title: "Cap include-chain recursion depth to eliminate native stack overflow"
status: assigned
branch: fix/247-expand-file-stack-overflow
worktree: ../sc-compose-worktrees/fix/247-expand-file-stack-overflow
target: develop
---

## Root Cause

`crates/sc-composer/src/include.rs::expand_file` (lines 87-171) recurses
natively once per include level with no bound other than the
caller-supplied `ComposePolicy::max_include_depth`
(`crates/sc-composer/src/types.rs:190-204`, a bare `u16` wrapper with no
internal cap, default `32` at `types.rs:407`).

The depth-limit check does run — `include.rs:96-104`:

```rust
let path_buf = path.to_path_buf();
if depth.get() > max_depth.get() {
    return Err(IncludeError::new(
        DiagnosticCode::ErrIncludeDepth,
        format!("include depth exceeded maximum of {}", max_depth.get()),
        stack.clone(),
    )
    .into());
}
```

— but it is the **first statement inside `expand_file` itself**, not a
guard the caller evaluates before invoking the next level. The recursive
call site is `include.rs:154-162`:

```rust
let nested = expand_file(
    &resolved_include,
    root,
    allowed_roots,
    max_depth,
    depth.next(),
    stack,
    state,
)?;
```

This call happens unconditionally for every include directive found in the
current file's body — the native call (and its stack frame) is already
made before the callee's first line ever runs the depth check. So the
diagnostic-vs-crash race is not really about statement ordering inside one
frame; it is that **traversal depth is bounded only by
`max_include_depth`, a value the caller fully controls (0..=65535), with no
relationship to the host's actual native stack size.** `IncludeDepth`
places no ceiling on what an embedder may configure (confirmed via
`types.rs:190-204` — `new()` accepts any `u16` unmodified), and the Python
bindings expose `max_include_depth` directly as a constructor parameter
(per issue #247's body, `bindings/python/src/types/policy.rs`).

Observed crash thresholds from the issue (reproduced 3x, deterministic):
debug build aborts (SIGABRT / stack overflow, exit 134) at include depth
1820+; release build aborts at depth 10000+. Both are far inside the
`0..=65535` range `IncludeDepth` accepts without rejection, so any operator
or embedder who raises `max_include_depth` into the low thousands (a
plausible ask for deep skill/agent composition trees) and renders a
semi-trusted include tree that deep takes down the **whole host process**,
not just the one request — this is a memory-safety/availability bug, not a
data-correctness one.

The CLI is not reachable this way today: there is no `--max-include-depth`
flag, so the CLI binary always uses the default of `32`. This is a
library/Python-bindings-level DoS only.

## Exact Target

Add a fixed internal safety ceiling on effective include depth, independent
of whatever `ComposePolicy::max_include_depth` the caller configures, and
apply it once in `expand_includes` before the traversal begins.

In `crates/sc-composer/src/include.rs`, near the top of the file (module
level, alongside `CurrentIncludeDepth`):

```rust
/// Hard ceiling on include-chain recursion depth, independent of any
/// caller-configured `ComposePolicy::max_include_depth`. `expand_file`
/// recurses natively once per include level; this bound keeps native
/// stack usage far below the observed stack-overflow thresholds (debug
/// ~1820, release ~10000) on any reasonably sized thread stack, so the
/// `ErrIncludeDepth` diagnostic can always fire before the process could
/// abort, no matter how high an embedder sets `max_include_depth`.
const MAX_SAFE_INCLUDE_DEPTH: u16 = 128;
```

In `expand_includes` (`include.rs:48-77`), clamp the effective depth passed
into the traversal instead of passing `policy.max_include_depth` straight
through:

```rust
pub fn expand_includes(
    template_path: impl AsRef<Path>,
    root: &ConfiningRoot,
    policy: &ComposePolicy,
) -> Result<ExpandedTemplate, ComposeError> {
    let template_path = canonicalize_include(
        template_path.as_ref(),
        root.as_path(),
        &policy.allowed_roots,
        &[],
    )?;

    let effective_max_depth = IncludeDepth::new(
        policy.max_include_depth.get().min(MAX_SAFE_INCLUDE_DEPTH),
    );

    let mut state = ExpansionState::default();
    let text = expand_file(
        &template_path,
        root.as_path(),
        &policy.allowed_roots,
        effective_max_depth,
        CurrentIncludeDepth::root(),
        &mut Vec::new(),
        &mut state,
    )?;

    Ok(ExpandedTemplate {
        text,
        resolved_files: state.resolved_files,
        frontmatters: state.frontmatters,
        include_chains: state.include_chains,
    })
}
```

`expand_file`'s existing depth check (`include.rs:96-104`) is unchanged —
it already correctly rejects with `DiagnosticCode::ErrIncludeDepth` when
`depth.get() > max_depth.get()`. The only change is that `max_depth` is now
always `min(configured, 128)`, so the check fires (and the diagnostic
returns cleanly) well before native recursion could reach a depth anywhere
near the observed crash thresholds, regardless of what `max_include_depth`
the caller configures. The initial 512 value was corrected after independent
validation showed that cargo test's default 2 MiB worker stack could still
overflow near that depth.

`128` is chosen with substantial margin below both observed thresholds (debug
1820+, release 10000+) to stay safe across smaller thread stacks (e.g. a
non-main thread in an embedding host, or the Python bindings' calling
thread) that were not directly measured. This is a plain `const`, not a
policy field — it is not meant to be caller-configurable; it exists purely
as a safety net, not a feature.

## This Sprint Does NOT Change

- `expand_file`'s cycle-detection logic (`include.rs:105-114`) — untouched.
- `CurrentIncludeDepth`'s `root()`/`next()`/`get()` semantics
  (`include.rs:24-39`) — untouched.
- The existing `ErrIncludeDepth` diagnostic code, message format, or the
  `depth_overflow_is_rejected` unit test's expectations
  (`include.rs:355-380`) — that test uses `IncludeDepth::new(1)`, far below
  the new `128` ceiling, so its behavior is identical before and after this
  change.
- `IncludeDepth`'s public API (`types.rs:190-204`) — no new validation is
  added there; `new()` still accepts any `u16` unmodified. The clamp lives
  entirely inside `expand_includes`, not in the type itself, to keep this a
  narrow, single-call-site fix rather than a public-API/behavior change
  that could affect other consumers of `IncludeDepth`.
- No iterative/trampolined rewrite of `expand_file`'s recursion. The issue
  proposes this as one option; it is out of scope for this sprint as a
  larger, higher-risk diff than a fixed safety ceiling. If a legitimate use
  case ever needs more than 128 levels of include nesting, that is a
  separate, deliberate follow-up (raising the constant with new stack-usage
  measurements, or doing the iterative rewrite) — not silently permitted by
  this fix.
- The CLI surface (no `--max-include-depth` flag exists and none is added
  here) and the Python bindings' `ComposePolicy` constructor signature
  (`bindings/python/src/types/policy.rs` is untouched — the clamp is
  transparent to it, it still accepts any `u16` for `max_include_depth`,
  just no longer causes a process abort when set unreasonably high).

## Required Test Matrix

All new tests live in `crates/sc-composer/src/include.rs`'s existing
`#[cfg(test)] mod tests` (library-level, per the issue's own
recommendation — this bug is not CLI-reachable today, so no
`crates/sc-compose/tests/fuzz_regressions.rs` CLI-level test is added for
it).

(a) **Red-baseline regression test (the mandatory `#[ignore]`d test — see
Process section)**: build a linear include chain of depth comfortably above
`MAX_SAFE_INCLUDE_DEPTH` (matching the issue's repro shape — e.g. 1900
files, `max_include_depth: IncludeDepth::new(1905)`) under a temp confined
root, call `expand_includes`, and assert it returns
`Err(ComposeError::Include(e))` with `e.code() ==
DiagnosticCode::ErrIncludeDepth` — i.e. a clean diagnostic, not a process
abort. **Before the fix lands, running this test genuinely crashes the
test process** (native stack overflow — the OS aborts the process, the
test harness reports the whole binary as having crashed/aborted rather
than printing a normal `FAILED` assertion message). See the Process
section below for how team-lead independently verifies this "genuinely
fails" condition given it is a process-level abort, not an in-process
assertion failure.

(b) A chain whose depth is at or under `MAX_SAFE_INCLUDE_DEPTH` (e.g. depth
50, well within both the old default of 32's typical range and the new
128 ceiling) with `max_include_depth` set high (e.g. `IncludeDepth::new(1000)`)
still expands successfully — confirms the clamp does not regress any
legitimate, moderately-deep include chain.

(c) A chain whose depth is between the caller's configured
`max_include_depth` and `MAX_SAFE_INCLUDE_DEPTH` where the caller's value
is the *lower* of the two (e.g. `max_include_depth: IncludeDepth::new(5)`,
chain depth 10) still returns `ErrIncludeDepth` at the caller's configured
bound, not at 128 — confirms `min()` picks the tighter of the two bounds
correctly, not always the constant.

(d) The existing `depth_overflow_is_rejected` test (`include.rs:355-380`,
`IncludeDepth::new(1)`) continues to pass unmodified — confirms no
behavior change for the common/default case.

(e) The existing `expands_successful_include_chain`,
`missing_include_reports_not_found`, and `cycle_detection_is_rejected`
tests (`include.rs:294-353`) continue to pass unmodified.

## Mandatory Process (two-commit red -> green, now standing requirement)

Per the corrective process quality-mgr required after SC-QA-255-001 /
SC-QA-256-001, and now confirmed clean 2/2 on FIX-245 and FIX-244:

1. **First commit**: land test (a) above as `#[ignore]`d in
   `include.rs`'s test module. Team-lead independently confirms it
   genuinely fails before any fix code is written.
   - **This test's failure mode is different from every prior fix in this
     queue**: it is not a normal assertion failure inside a passing test
     process — the test process itself aborts (SIGABRT / "fatal runtime
     error: stack overflow", non-zero/signal exit). Team-lead's
     verification for the red commit must run
     `cargo test --workspace -p sc-composer -- --ignored
     <test_name> --nocapture` directly and confirm the command's own exit
     status is non-zero with stack-overflow output (not a `FAILED` test
     line) — a normal `cargo test` "ok"/"FAILED" summary line for this
     test at this stage means the red commit is wrong (either the test
     doesn't reproduce the crash, or the fix was accidentally already
     applied) and must not be accepted as a valid red baseline.
2. **Second commit**: land the `MAX_SAFE_INCLUDE_DEPTH` clamp fix plus
   tests (b)-(c) above, and remove the single `#[ignore]` line from test
   (a). No other test-logic changes in this commit. Team-lead
   independently re-runs the same command from step 1 and confirms it now
   exits cleanly with a normal passing/failing test summary (not a crash),
   and that the test's assertion (`ErrIncludeDepth`) passes.
3. Sprint-doc closeout narrative must state accurate, verifiable
   provenance — the regression test is created fresh on this branch, never
   described as "promoted" from elsewhere.

## Acceptance Criteria

- `cargo test --workspace` passes, including the now-unignored test (a)
  and new tests (b)-(c), with no process crash anywhere in the run.
- The issue #247 repro snippet (depth ~1900, `max_include_depth: 1905`)
  returns a normal `Err` with `DiagnosticCode::ErrIncludeDepth` instead of
  aborting the process, on both debug and release builds.
- `depth_overflow_is_rejected` and the other three pre-existing
  `include.rs` unit tests still pass unmodified.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- GitHub issue #247 can be closed referencing the merged PR.

## References

- GitHub issue #247
- `crates/sc-composer/src/include.rs` (`expand_includes`, `expand_file`,
  `CurrentIncludeDepth`, existing test module)
- `crates/sc-composer/src/types.rs` (`IncludeDepth`, lines 190-204;
  `ComposePolicy::default`, lines 402-413)
