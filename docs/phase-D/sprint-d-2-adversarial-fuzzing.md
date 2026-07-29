---
id: D.2
title: Adversarial Fuzzing Workflow
status: planned
branch: sprint/d-2-adversarial-fuzzing
worktree: ../sc-compose-worktrees/sprint/d-2-adversarial-fuzzing
target: develop
---

# Sprint D.2 — Adversarial Fuzzing Workflow

## Goal

Create a reusable `$adversarial-fuzzing` skill for sc-compose rendering QA. A
primary coordinator agent must deploy several bounded background agents, each
focused on one rendering boundary, then aggregate, minimize, and classify
breaking templates or inputs. A confirmed product bug becomes a deterministic
unit or CLI regression test; an intentional boundary remains a documented QA
result rather than a false bug.

This is an agent-workflow/design sprint. It produces versioned skill and agent
specifications plus registry metadata, not sc-compose runtime code. It must be
explicitly reviewed as tooling that exercises the product, not as closure of a
new runtime feature.

## Hard Dependencies

- [Phase D plan](./phase-D-plan.md)
- [Sprint D.1 — Recursive Structured Input Support](./sprint-d-1-recursive-structured-input.md)
- [Claude Code skills and agents guidelines](https://github.com/randlee/synaptic-canvas/blob/main/docs/claude-code-skills-agents-guidelines.md)
- `.claude/agents/registry.yaml`
- `.claude/agents/quality-mgr.md`
- `.claude/skills/quality-management-gh/SKILL.md`

D.1 is a hard dependency because the first campaign must know the expanded
recursive-input contract and must not classify the former nested-array
restriction as a valid oracle.

## Exact Targets

- `.claude/skills/adversarial-fuzzing/SKILL.md`
- `.claude/skills/adversarial-fuzzing/agents/openai.yaml`
- `.claude/skills/adversarial-fuzzing/references/installation-and-troubleshooting.md`
- `.claude/agents/sc-adversarial-fuzz-coordinator.md`
- `.claude/agents/sc-adversarial-fuzz-probe.md`
- `.claude/agents/quality-mgr.md`
- `.claude/agents/registry.yaml`
- `docs/phase-D/sprint-d-2-adversarial-fuzzing.md`

All exact targets are skill, agent, registry, or planning files. No product
crate or runtime file is in scope for D.2.

## Deliverables

- `D1` — a discoverable `adversarial-fuzzing` skill whose description names
  rendering-breakage, risky-change validation, regression-hunting, and test
  promotion triggers.
- `D2` — a primary coordinator agent that validates campaign inputs, selects
  focused workers, spawns at most four background agents with deterministic
  correlation IDs and timeouts, aggregates partial failures, and returns a
  fenced JSON envelope.
- `D3` — a single-responsibility probe agent contract covering value/ingress,
  template behavior, negative boundaries, and differential/metamorphic checks
  through coordinator-assigned focus.
- `D4` — a promotion contract requiring reproduction, minimization, explicit
  expected oracles, and a durable test in the owning crate test suite before
  a failure is called a confirmed bug.
- `D5` — quality-mgr routing and registry metadata that make the skill and
  agents versioned, discoverable, and reviewable without arbitrary agent paths.
- `D6` — a first-campaign checklist and structured report contract that record
  seed, worker correlation, limits, findings, promoted tests, and unresolved
  candidates.

The deliverable list above is authoritative for D.2 closure.

## Coordinator contract

The skill must pass a contract equivalent to:

```json
{
  "worktree_path": "/absolute/path/to/worktree",
  "target": "var-file | frontmatter | resolver | renderer | includes | cli | full",
  "baseline_ref": "optional git ref",
  "seed": 157,
  "max_workers": 4,
  "cases_per_worker": 100,
  "per_worker_timeout_s": 120,
  "promote_regressions": true
}
```

The coordinator must reject unsafe paths, invalid limits, and unregistered
workers before execution. It must use the Agent Runner/registered agent
contract and must fail closed if the required runner is unavailable.

## Worker portfolio and parallel guardrails

Deploy only the workers relevant to the target, and deploy all four for a full
campaign:

| Correlation ID | Target | Required adversarial surface |
| --- | --- | --- |
| `shape-probe` | values/ingress | recursive JSON/YAML trees, mixed and empty arrays, JSON/YAML parity |
| `template-probe` | template engine | nested loops, conditionals, includes, delimiters, whitespace, Unicode |
| `boundary-probe` | validation/CLI | malformed inputs, top-level shape, stable diagnostics, path confinement |
| `differential-probe` | regression oracle | baseline comparison, metamorphic relations, determinism, panic/hang/timeout |

Use `run_in_background: true`, cap concurrency at four, assign each worker a
unique correlation ID, bound cases and wall time, order results by correlation
ID, and expose partial failures. Permit at most one retry only when the error
is explicitly recoverable.

## Finding and promotion contract

Require every candidate to carry a minimal template/input, exact command,
seed, expected oracle, observed result, diagnostic code where applicable, and
reproduction count. Classify it as:

- `confirmed_bug` for a stable panic, hang, wrong valid output, broken
  metamorphic relation, unexplained JSON/YAML semantic divergence, regression,
  or violated stable boundary;
- `intentional_boundary` for documented rejection or malformed input;
- `inconclusive` for nondeterministic or oracle-insufficient behavior.

Minimize by removing template blocks, variables, fields, array members, and
nesting while preserving the failure. Require three reproductions before
promotion. Add pure-library findings to `sc-composer` unit tests and CLI,
diagnostic, or output findings to `sc-compose` integration tests. Do not let
workers edit production code or commit fixes. Do not promote flaky tests.

Return a standard envelope:

```json
{
  "success": true,
  "data": {
    "parallel": true,
    "concurrency": 4,
    "per_task_timeout_s": 120,
    "results": [],
    "summary": {
      "confirmed_bugs": 0,
      "promoted_tests": 0,
      "inconclusive": 0,
      "failed_workers": []
    }
  },
  "error": null
}
```

`success: true` means the campaign completed, even if findings exist.
Malformed input, unsafe paths, unavailable runner, and malformed aggregate
output are fatal structured errors.

## Quality-mgr integration

Update quality-mgr instructions so an adversarial campaign can be selected as
an independent QA pass for a sprint or PR. The quality-mgr report must retain
the existing verdict and severity fields while adding:

- campaign seed, target, baseline, worker count, and timeout;
- confirmed, intentional-boundary, inconclusive, and failed-worker counts;
- finding IDs and promoted test paths;
- the next owner for every confirmed bug not fixed in the current sprint.

The quality-mgr must not convert an inconclusive result into PASS, hide a
worker timeout, or close a sprint solely because the campaign produced no
finding. A no-finding campaign is evidence only when the requested target,
case budget, and worker execution all completed successfully.

## This Sprint Does Not Close

- production fixes for any discovered bug;
- a general-purpose fuzzing engine or network service;
- a guarantee that every possible template is exhaustively explored;
- schema-language design or new sc-compose runtime behavior;
- automatic commits, merges, or destructive worktree cleanup;
- ATM-specific runtime dependencies in either Rust crate.

## Acceptance Criteria

- The skill passes structural validation and triggers on adversarial rendering,
  risky-change, regression-hunting, and test-promotion requests.
- The coordinator and probe agents have required versioned frontmatter,
  single responsibilities, fenced JSON output, explicit error semantics, and
  repository/path safety constraints.
- Registry entries resolve both agents and the skill with compatible versions;
  quality-mgr can route an adversarial pass without copying scope manually.
- A full campaign is bounded to four concurrent workers, deterministic seed and
  correlation ordering, explicit timeouts, partial-failure reporting, and no
  silent retries.
- Candidate failures are minimized and classified before promotion; confirmed
  bugs produce deterministic tests in the owning crate suite.
- Intentional boundaries and inconclusive results remain visible in the final
  report and cannot be reported as PASS.
- D.2 leaves product runtime code unchanged and passes the required structural
  and repository validation.

## Required Validation

Run from the D.2 worktree:

```bash
python3 /Users/randlee/.codex/skills/.system/skill-creator/scripts/quick_validate.py .claude/skills/adversarial-fuzzing
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
```

Also parse `.claude/agents/registry.yaml`, verify every registered path exists,
check skill/agent versions against registry entries, and run one bounded dry
campaign that exercises coordinator aggregation and a worker timeout without
mutating production code.
