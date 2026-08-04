---
id: sprint-I.6
title: YAML Merge-Key Var-File Safety
phase: I
status: in progress
branch: sprint/i-6-yaml-merge-key-safety
worktree: ../sc-compose-worktrees/sprint/i-6-yaml-merge-key-safety
target: integrate/phase-i
---

# Sprint I.6 — YAML Merge-Key Var-File Safety

## Purpose

Close GitHub issue #166. Eliminate the current silent-loss behavior when a
JSON/YAML var-file contains YAML merge keys such as `<<: *defaults`.

## Dependencies and exact targets

- I.1 accepted merge-key policy, diagnostic, and limits;
- `crates/sc-compose/src/var_file.rs` decode/validation boundary;
- `crates/sc-composer/src/types.rs` YAML-to-input conversion;
- CLI var-file diagnostics and existing var-file tests;
- `docs/error-code-registry.md` and requirement/architecture updates.

## Deliverables

- Detect merge-key syntax before generic `serde_yaml::Value::Tagged` unwrapping
  can erase its semantics.
- Implement the I.1-selected policy. The default fail-closed path returns
  `ERR_CONFIG_VARFILE` with the variable/object location and explains that
  merge keys are unsupported rather than silently dropping inherited fields.
- If I.1 instead approves expansion, implement bounded acyclic expansion with
  explicit precedence, duplicate-key behavior, nested aliases, depth/size
  limits, and identical CLI/library conversion semantics. Do not mix partial
  expansion with fallback unwrapping.
- Add exact #166 reproduction tests showing that `base`/`name` inherited
  fields cannot disappear while the command exits successfully.
- Add direct tests for nested merges, multiple merge sources, explicit-key
  precedence, aliases without merge keys, cycles/depth limits, malformed YAML,
  non-string keys, and JSON unaffected behavior.

## Acceptance criteria

- The #166 reproduction either produces the fully specified inherited object
  or fails with the documented stable diagnostic; it never returns exit 0
  with missing inherited fields.
- The chosen policy is the same for direct parsing and the CLI render path.
- Existing valid JSON/YAML objects, arrays, nested objects, duplicate-key
  rejection, and non-string-key diagnostics remain intact.
- Error output identifies the unsupported merge construct and gives an
  actionable recovery (expand the mapping explicitly or use the supported
  policy), without exposing internal serde details as the only guidance.
- No generic tagged-value path can reintroduce silent merge-key loss.

## Required validation

Use the [authoritative Phase I validation
checklist](phase-I-plan.md#authoritative-validation-checklist). Run the exact
issue reproduction and its control case through the CLI, then
retain the input, exit code, parsed variables or diagnostic, and requirement/
ADR trace in evidence.

## Removal path

If the selected merge-key policy fails QA, remove the detector/expander and
its fixtures as one boundary change; restore neither the old silent-loss path
nor generic tagged-value unwrapping for merge nodes. The issue reproduction
must remain a required regression before a replacement policy is accepted.

## Out of scope

- YAML merge semantics in extraction of rendered YAML documents (this sprint
  is var-file input only);
- changing frontmatter merge precedence;
- accepting arbitrary YAML tags or aliases that are not part of the selected
  merge policy;
- silently coercing unsupported YAML constructs into JSON.
