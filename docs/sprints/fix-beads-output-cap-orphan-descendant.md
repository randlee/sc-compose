---
status: complete
branch: fix/beads-output-cap-orphan-descendant
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/beads-output-cap-orphan-descendant
---

# FIX-BEADS-FUZZ-OUT-001: orphaned descendant defeats output-cap termination

## Source

PR #564's Beads execution/filesystem-safety fuzz campaign found that
`StdProcessRunner::run` killed only the direct child after its 64 KiB output
cap was breached. A normal descendant that inherited stdout or stderr could
then keep the pipe open after the child exited, causing the runner to block.

## Contract

On a per-stream output-cap breach, the production runner terminates its
contained process tree before returning `BEADS_PROCESS_OUTPUT_LIMIT`.

- Unix: the child leads a dedicated process group; termination kills the group.
- Windows: the child is created suspended, assigned to a Job Object, resumed,
  and termination calls `TerminateJobObject`.
- Other platforms retain direct-child behavior because this implementation has
  no equivalent containment primitive there.

Version 1 has no elapsed-process timeout; callers own cancellation and outer
deadlines. This fix covers the output-cap liveness boundary only.

## Deliverables

- [x] Use the safe `process-wrap` containment abstraction rather than local
  unsafe platform FFI.
- [x] Add Unix process-group containment and Windows Job Object containment in
  the same runner contract.
- [x] Replace the shell-based pinned reproducer with a helper binary that
  starts a pipe-inheriting descendant, overflows output, and exits the root.
- [x] Verify that the runner returns promptly and that the descendant stops
  updating its cross-platform state marker after containment.
- [x] Update ADR-0021, architecture, CLAUDE, and the sc-lint dependency
  boundary for the approved platform-containment dependency.

## Validation

```text
cargo test -p sc-composer-beads --test runner_process_tree
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

The helper-binary regression is compiled and run on the existing Linux,
macOS, and Windows workspace-test matrix.
