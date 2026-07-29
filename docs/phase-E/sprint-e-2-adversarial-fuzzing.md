---
id: E.2
title: Adversarial Fuzzing Workflow
status: complete
branch: sprint/e-2-adversarial-fuzzing
worktree: ../sc-compose-worktrees/sprint/e-2-adversarial-fuzzing
target: integrate/phase-e
---

# Sprint E.2 — Adversarial Fuzzing Workflow

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

- [Phase E plan](./phase-E-plan.md)
- [Sprint E.1 — Recursive Structured Input Support](./sprint-e-1-recursive-structured-input.md)
- [Claude Code skills and agents guidelines](https://github.com/randlee/synaptic-canvas/blob/main/docs/claude-code-skills-agents-guidelines.md)
- `.claude/agents/registry.yaml`
- `.claude/agents/quality-mgr.md`
- `.claude/skills/quality-management-gh/SKILL.md`

E.1 is a hard dependency because the first campaign must know the expanded
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
- `docs/phase-E/sprint-e-2-adversarial-fuzzing.md`

All exact targets are skill, agent, registry, or planning files. No product
crate or runtime file is in scope for E.2.

## Current Implementation State

The initial skill, coordinator, probe, registry, and installation-reference
artifacts already landed before this sprint on commit `5fb6cb2` and are present
on this branch as commit `63d3c83`. Those artifacts correspond to the initial
skill/coordinator/probe deliverables; they are not falsely treated as the
closure of the sprint.

E.2 remains open for quality-mgr routing, adversarial campaign evidence, and
the real classify/minimize/promote workflow owned by E.3. Any E.2 edits must
build on the pre-existing implementation rather than silently replacing it or
rewriting its history.

At closure: E.1 merged into `integrate/phase-e` via PR #159 (commit `42b784b`),
confirming the recursive-input runtime contract before E.3's campaign started.
E.3 subsequently ran its first campaign against that merged contract and
merged via PR #161 (commit `ca40c6b`).

### Phase-plan dependency exception (historical)

E.2 was a parallel workflow-authoring sprint. Its `complete` status covered
the skill, agent, registry, and report-contract deliverables listed here, and
did not require E.1's runtime changes to be merged into `integrate/phase-e`
at the time E.2 itself closed. That exception is now resolved: E.1 merged
first (PR #159), and E.3 confirmed the merge before starting its campaign,
satisfying E.1's hard dependency for campaign execution.

## Deliverables

- `E2-D1` — a discoverable `adversarial-fuzzing` skill whose description names
  rendering-breakage, risky-change validation, regression-hunting, and test
  promotion triggers.
- `E2-D2` — a primary coordinator agent that validates campaign inputs, selects
  focused workers, spawns at most four background agents with deterministic
  correlation IDs and timeouts, aggregates partial failures, and returns a
  fenced JSON envelope.
- `E2-D3` — a single-responsibility probe agent contract covering value/ingress,
  template behavior, negative boundaries, and differential/metamorphic checks
  through coordinator-assigned focus.
- `E2-D4` — a promotion contract requiring reproduction, minimization, explicit
  expected oracles, and a durable test in the owning crate test suite before
  a failure is called a confirmed bug.
- `E2-D5` — quality-mgr routing and registry metadata that make the skill and
  agents versioned, discoverable, and reviewable without arbitrary agent paths.
- `E2-D6` — a first-campaign checklist and structured report contract that
  records seed, worker correlation, limits, findings, promoted tests,
  unresolved candidates, and next owners for unfixed confirmed bugs.

The deliverable list above is authoritative for E.2 closure. E.3 owns the
first real campaign and its promoted tests; E.2 does not claim that campaign
has already run.

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
      "all_successful": true,
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
  the `qa_routes.adversarial-fuzzing` entry lets quality-mgr route an
  adversarial pass without copying scope manually.
- A full campaign is bounded to four concurrent workers, deterministic seed and
  correlation ordering, explicit timeouts, partial-failure reporting, and no
  silent retries.
- Candidate failures are minimized and classified before promotion; confirmed
  bugs produce deterministic tests in the owning crate suite.
- Intentional boundaries and inconclusive results remain visible in the final
  report and cannot be reported as PASS.
- The report contract records campaign metadata, every worker correlation and
  limit, finding IDs, promoted test paths, failed-worker counts, unresolved
  candidates, and next owners.
- E.2 leaves product runtime code unchanged and passes the required structural
  and repository validation.
- The classify/minimize/promote pipeline is not considered proven until the
  first real campaign passes the E.3 exit gate.

## Required Validation

Run from the E.2 worktree:

```bash
python3 <skill-creator-root>/scripts/quick_validate.py .claude/skills/adversarial-fuzzing
git diff --check
```

The `skill-creator-root` placeholder is operator-specific and this structural
validator is non-gating when the skill-creator package is unavailable; report
that fact explicitly rather than using a hardcoded workstation path. The
gating checks are registry parsing, registered-path existence, version
compatibility, fenced-contract review, and `git diff --check`.

Run the full repository `cargo fmt --all --check`, `cargo test --workspace`,
and clippy gates in E.3 because E.3 owns promoted Rust/CLI regression tests;
E.2 has no Rust Exact Targets and does not claim to have executed a real
campaign. E.3's first campaign is the end-to-end proof of classify → minimize
→ promote → test.

## Closure Notes

- E2-D1 through E2-D4 remain based on the previously landed skill, coordinator,
  probe, and registry artifacts; this sprint did not rewrite their history.
- E2-D5 adds the versioned `qa_routes.adversarial-fuzzing` registry route and
  quality-mgr rules for safe coordinator dispatch and non-lossy QA reporting.
- E2-D6 adds the reusable first-campaign checklist and
  `adversarial-fuzzing/v1` durable evidence contract in the skill.
- The first real campaign, regression promotion, and end-to-end pipeline proof
  remain owned by E.3 and are intentionally not claimed here.
