---
name: adversarial-fuzzing
description: Generate and triage adversarial templates, var-files, and rendering inputs against sc-compose by coordinating bounded background agents, differential and metamorphic checks, minimization, and regression-test promotion. Use when trying to break a rendering subsystem, validating a risky change, hunting parser/validator/rendering edge cases, or turning a confirmed fuzz failure into a unit or CLI test.
---

# Adversarial Fuzzing

Use this skill to attack one sc-compose rendering subsystem with several
focused, isolated probes. Preserve useful failures as minimal regression tests
only after reproducing and classifying them as real product bugs.

## Step 1 — Verify required CLI dependencies

Run this before selecting agents or executing a fuzz campaign:

```bash
which cargo && cargo --version
which git && git --version
```

If either command is unavailable, stop and read
`references/installation-and-troubleshooting.md` before proceeding. Do not
silently run a degraded campaign.

## Campaign contract

Require a JSON input object with these fields. Reject missing or unsafe paths
before delegating work.

```json
{
  "worktree_path": "/absolute/path/to/sc-compose-worktree",
  "target": "var-file | frontmatter | resolver | renderer | includes | cli | full",
  "baseline_ref": "optional git ref for differential checks",
  "seed": 157,
  "max_workers": 4,
  "cases_per_worker": 100,
  "per_worker_timeout_s": 120,
  "promote_regressions": true,
  "notes": "optional target-specific context"
}
```

Require `worktree_path` to be an existing absolute path inside the repository
or an explicitly approved isolated worktree. Resolve and validate all generated
paths beneath it; reject path traversal and paths outside the worktree.

Use a deterministic seed by default, cap concurrency at four workers, set a
per-worker timeout, and record the seed and campaign correlation ID in every
result. Use a fresh temporary directory for generated inputs and clean it up
after the campaign unless a failure artifact is being preserved.

## Workflow

1. Verify the CLI dependencies above.
2. Inspect the target subsystem, current tests, repository boundary rules, and
   the requested baseline. Keep the user's existing changes intact.
3. Use the Agent Runner to invoke the registered
   `sc-adversarial-fuzz-coordinator` agent with the campaign contract. Do not
   invoke an unregistered agent path.
4. Have the coordinator spawn focused workers in the background with
   `run_in_background: true`, one target per worker and a unique
   `correlation_id`.
5. Aggregate worker results in correlation-ID order. Surface partial failures;
   never discard a timeout, malformed response, or worker error.
6. Reproduce each candidate failure in the coordinator's worktree, minimize the
   template and input while retaining the failure, and classify the outcome
   using the oracle rules below.
7. When a candidate is a confirmed product bug and `promote_regressions` is
   true, add the smallest durable test to the nearest existing test suite.
   Prefer inline fixtures for small cases and checked-in fixtures for complex
   templates. Do not change production code or silently implement a fix during
   a fuzz campaign.
8. Run targeted tests, then the repository's required formatting, test, lint,
   and boundary checks when a regression test was added.
9. Return the coordinator's fenced JSON report and summarize confirmed bugs,
   promoted tests, unresolved candidates, and campaign limits.

## Worker portfolio

Ask the coordinator to deploy only the workers relevant to `target`; use all
four for `full`:

| Worker | Focus | Primary probes |
| --- | --- | --- |
| `shape-probe` | Values and ingress | recursive JSON/YAML trees, mixed arrays, empty values, numeric edge cases, object/map nesting |
| `template-probe` | Jinja behavior | nested loops, conditionals, includes, delimiters, whitespace, missing/optional fields, Unicode |
| `boundary-probe` | Negative contract | malformed files, non-object var-files, invalid YAML keys, invalid names, stable diagnostics, path confinement |
| `differential-probe` | Change/regression oracle | baseline-vs-head behavior, JSON/YAML parity, deterministic output, timeout/panic/hang detection |

Each worker must stay within its target, use bounded generation, and return a
standard fenced JSON envelope. Workers may create temporary inputs and logs,
but must not edit production code, commit changes, or delete user files.

## Oracle and triage rules

Classify a candidate as a confirmed bug only when at least one condition holds:

- the process panics, hangs, exceeds the configured timeout, or exits with an
  unexplained internal error;
- a valid input that the documented contract accepts renders incorrectly;
- equivalent JSON and YAML inputs produce different semantics without a
  documented reason;
- a metamorphic relation fails, such as adding an unused object field changing
  output or reordering independent inputs changing deterministic output;
- a formerly valid input regresses against `baseline_ref`;
- a stable diagnostic code or top-level boundary is violated.

Do not file a bug for an intentional validation error, a malformed test input,
an unsupported feature explicitly documented by the target contract, or an
oracle that cannot establish expected behavior. Preserve such cases as
negative-coverage notes when they reveal a missing contract.

For every candidate, require:

```json
{
  "id": "FUZZ-001",
  "status": "confirmed_bug | intentional_boundary | inconclusive",
  "subsystem": "var-file",
  "seed": 157,
  "command": "cargo run ...",
  "minimal_template": "...",
  "minimal_input": "...",
  "observed": "...",
  "expected_oracle": "...",
  "diagnostic": "optional code",
  "reproduction_count": 3,
  "recommended_test": "..."
}
```

Minimize by deleting unrelated template blocks, variables, fields, array
members, and nesting while re-running the exact command. Keep the smallest
reproducer that fails at least three times, or record nondeterminism explicitly
if it cannot be reproduced deterministically.

## Regression-test promotion

Promote only `confirmed_bug` candidates. Place tests according to behavior:

- pure value conversion/validation: `crates/sc-composer` unit tests;
- CLI ingress, diagnostics, or output: `crates/sc-compose/tests/cli.rs` or
  `json_cli.rs`;
- cross-platform path or boundary behavior: the existing boundary test suite.

Every promoted test must include the minimized input, expected output or stable
diagnostic, and a short comment naming the fuzz finding ID. Avoid comments that
leak campaign-only assumptions; document the user-visible invariant instead.

After promotion, run:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
```

Also run the repository's boundary checks and retain the generated campaign
report as an artifact when the team workflow requires it. A test that cannot
be made deterministic or cross-platform should remain a finding, not be
promoted as a flaky regression.

## Aggregation contract

The coordinator must return fenced JSON only, with results ordered by
`correlation_id`:

```json
{
  "parallel": true,
  "concurrency": 4,
  "per_task_timeout_s": 120,
  "results": [],
  "summary": {
    "all_successful": true,
    "confirmed_bugs": 0,
    "promoted_tests": 0,
    "inconclusive": 0,
    "failed_workers": []
  },
  "error": null
}
```

Use `success: true` when the campaign completed even if findings exist. Use
`success: false` for invalid input, coordinator failure, or a malformed worker
contract. Represent timeouts as recoverable worker failures and do not retry
more than once.

## Safety and cleanup

- Never run destructive commands, network-facing services, or arbitrary code
  from generated templates.
- Do not expose secrets, environment dumps, or unrelated repository contents in
  findings.
- Keep all writes inside the approved worktree and temporary campaign directory.
- Never reset, clean, or overwrite the user's existing changes.
- Remove temporary inputs after collecting confirmed artifacts.
- Do not commit production fixes automatically; commit only promoted tests when
  the user or repository workflow explicitly authorizes that mutation.
