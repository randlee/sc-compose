# ADR-0026: Contract Decisions From the Phase V Post-Merge Fuzz Campaign

## Status

Accepted (2026-11-06). Implementation is pending for every decision. No fix
has landed, and the two new error codes are not emitted yet.

## Approval provenance

All references are user messages in the team-lead Claude Code session
`9ec5e645-3303-4b01-b48f-a3583fb75424`. The transcript is
`~/.claude/projects/-Users-randlee-github-sc-compose/9ec5e645-3303-4b01-b48f-a3583fb75424.jsonl`.
Each message is cited by its uuid and UTC time.

| Decision | Kind | Reference |
|---|---|---|
| b059 (part of D1): include escape is the correct error | explicit user ruling | `60cfdb6b`, 2026-11-06T22:02:39Z: "escape sounds like the correct error to return" |
| D1 rule: a boundary violation is reported in preference to not-found | explicit user ruling | `8251bf87`, 2026-11-06T22:08:22Z: "a boundary violation is a more specific error than what the agent expected" |
| D3, D7, D8 (parent line endings) | explicit user approval | `610cd975`, 2026-11-06T22:11:59Z: "agree" to each |
| D9 | explicit user approval | `610cd975`: "bug --> Multi-pass" |
| D2, D4, D5, D6, D10, D11, D12, and the D8 points other than line endings | delegated contract decision | `8251bf87`: "We need to make appropriate decisions, create req/adr to cover any ambiguity and fix any issues"; `7bf5ef58`, 2026-11-06T22:06:35Z: "There should be no inconclusive results" |

Approval of the FR-24.2 exceptions:

- **D7, D8 line endings, D9:** the user explicitly approved each behaviour
  change. That approval is recorded as the exception approval.
- **D4 (`--output` no longer echoes to stdout):** this is a delegated
  decision. The user has not explicitly approved its FR-24.2 exception, and
  FR-24.2 says adding an entry requires user approval. The D4 fix must not add
  its `approved-exceptions.json` entry until that explicit approval is
  recorded.
- **D8 trailing-line-break point:** it changes output, and its exception
  approval is likewise still pending.

## Context

The post-merge adversarial fuzz campaigns (`comp-phase-v.adversarial-fuzz-postmerge`
and `comp-phase-v.fuzz-c02-bounds-rework`) produced findings where the code
contradicts the written contract, or where the contract says nothing. Matching
v1.7.0 is not evidence of correctness. Every decision below is judged on the
contract text and on what callers need.

FR-24.2 still applies: a change to schema-less template output ships only with
an entry in `approved-exceptions.json` that links its GitHub issue. Decisions
marked **output change** need such an entry.

## Decisions

### D1. A boundary violation is reported in preference to not-found

When a requested path, or the target it resolves to (including through a
symlink, dangling or not), lies outside the allowed roots, the error is the
confinement code. It is not the not-found code. A boundary violation is a more
specific and more actionable error than not-found. This applies to:

- includes: `ERR_INCLUDE_ESCAPE` (unchanged code). A dangling symlink inside
  the root whose target is outside the root is an escape, not
  `ERR_INCLUDE_NOT_FOUND`. *(finding FUZZ-C02-001: bug)*
- templates named with `--file` or resolved by path: new code
  `ERR_RESOLVE_ESCAPE`, replacing `ERR_RESOLVE_NOT_FOUND` for a path outside
  the allowed roots. The text and JSON messages must name the same cause.
  *(finding FUZZ-003: bug)*
- the include fallback to the root (FR-3 resolution order) keeps this rule.
  A relative include that is missing next to its parent and resolves outside
  the root reports `ERR_INCLUDE_ESCAPE`. This is current behaviour. The fuzz
  oracle that expected not-found was wrong. The code comment in
  `path_containment.rs` that says missing candidates are checked "lexically"
  must be corrected to describe this rule. *(finding b059: documentation)*

### D2. Template engine errors get their own code

`ERR_RENDER_WRITE` means only an output write or output-target failure. A
template syntax or engine evaluation error reports the new code
`ERR_RENDER_TEMPLATE`, in both text and JSON output, with the template path
and line. *(finding FUZZ-002: bug)*

### D3. Duplicate frontmatter keys fail

Any duplicate key in a frontmatter mapping, at any level and whether the key
is recognized or not, fails with `ERR_CONFIG_PARSE`. This matches the FR-12
rule for var-files. *(finding FUZZ-006: bug; user approved)*

### D4. `--output` replaces stdout

When `render` writes to a file with `--output`, it writes nothing to stdout in
text mode. JSON mode already omits `body`. **Output change.** *(finding
FUZZ-010: bug)*

### D5. Output paths are not confined to the template root

FR-4 confinement covers reads. `render --output` and `--append` may write to
any path the caller names, because the caller chose that path. Beads request
outputs keep their own confinement under `working_directory` (ADR-0021).
*(finding FUZZ-004: documented)*

### D6. Unknown top-level bead request keys are accepted and ignored

The bead request schema `sc-compose/beads/v1` ignores unknown top-level keys in
every mode. Callers may use such keys for their own annotations. Rejecting
them would break FR-24.24 for `--render-by bd`. The template path suffix is
not enforced; only the `rendered_formula` extension is checked (ADR-0021).
*(finding FUZZ-005: documented)*

### D7. `--var` wins over `--var-file`

Within explicit input variables (FR-2, rank 1), an inline `--var` value
overrides the same key loaded from `--var-file`, regardless of argument order.
Among repeated `--var` arguments for one key, the last wins; among several
`--var-file` arguments, a later file wins. Both are current behaviour and are
kept. **Output change** for the `--var` versus `--var-file` order only.
*(user approved)*

### D8. Include expansion keeps the parent's layout

- The directive line `@<path>` is replaced by the included content, and the
  replacement always ends the line: if the included content has no final line
  break, the parent's line break follows it. Text after the directive in the
  parent is never joined onto the included content's last line.
- Line endings outside the included content are never changed. A CRLF parent
  stays CRLF after an include directive. The included content keeps its own
  line endings.
- Indentation in front of a directive is not applied to the included lines.
- Include directives expand before Jinja evaluation, so a directive inside
  `{% raw %}` or a false `{% if %}` is still expanded and validated. FR-3a and
  include-chain validation depend on this order.

**Output change** for the first two points. *(user approved)*

### D9. Every pass handles whitespace the same way

The pass-2 delimiters (`{{{ }}}`, ADR-0002) follow the same whitespace rules as
pass 1. An expression at the end of a line keeps its line break in every pass.
**Output change.** *(user confirmed as a bug)*

### D10. Errors inside included files name the included file

A render or validation error raised from included content reports the included
file and its own line number, plus the include chain (FR-3). It must not report
the line in the expanded parent.

### D11. Var-file scalar typing

- A YAML scalar of digits with a leading zero (`012`) stays the string `"012"`.
  This keeps identifiers and codes intact and avoids the YAML 1.1 octal versus
  YAML 1.2 decimal ambiguity. Callers who need a number write it without the
  leading zero.
- `--var-file` accepts a JSON or YAML object (FR-7a), detected by content. Text
  that is not valid JSON but is valid YAML is read as YAML, whatever the file
  extension. *(finding FUZZ-C03-002: documented)*
- Non-finite YAML numbers (`.inf`, `.nan`) are rejected because var-file values
  must be JSON-compatible. *(finding FUZZ-009: documented)*

### D12. Binary include targets

An include target that cannot be read as valid text fails with
`ERR_CONFIG_READ`, as the architecture taxonomy already defines.

## Exception receipt format

No `approved-exceptions.json` exists yet. The Phase V plan names its location
as `tests/fixtures/render-regressions/approved-exceptions.json`. Every fix
owner uses this single format, so entries compose: a JSON object holding one
`exceptions` array, sorted by `id`.

```json
{
  "exceptions": [
    {
      "id": "ADR-0026-D7",
      "decision": "inline --var overrides --var-file regardless of argument order",
      "issue": "https://github.com/randlee/sc-compose/issues/<n>",
      "approval": {
        "kind": "explicit-user",
        "reference": "session 9ec5e645 message 610cd975",
        "date": "2026-11-06"
      },
      "case": "tests/fixtures/render-regressions/cases/<case-dir>",
      "before": { "exit": 0, "stdout_sha256": "<v1.7.0 bytes>", "code": null },
      "after": { "exit": 0, "stdout_sha256": "<fixed bytes>", "code": null },
      "regression_test": "<crate>/tests/<file>.rs::<test name>"
    }
  ]
}
```

Rules for the format:

- `issue` names the linked GitHub issue. Reuse an existing issue when one
  matches; otherwise create one.
- `approval.kind` is `explicit-user`. A delegated decision cannot supply an
  entry.
- `before` is the v1.7.0 golden. It is never regenerated from candidate
  output.
- `after` is the fixed output.
- `regression_test` is deterministic and uses controlled inputs.

The first fix to land creates the file.

## Consequences

- D1, D2: two new registry codes, `ERR_RESOLVE_ESCAPE` and
  `ERR_RENDER_TEMPLATE`. Callers matching `ERR_RESOLVE_NOT_FOUND` or
  `ERR_RENDER_WRITE` for these causes must update. Release notes list both.
- D4, D7, D8 and D9 change schema-less output. Each fix PR adds an
  `approved-exceptions.json` entry in the format above, with its linked issue,
  and a regression test with controlled inputs. For D4 and the D8
  trailing-line-break point, the entry waits for explicit user approval (see
  Approval provenance).
- D5, D6, D11, D12 need no code change, only requirement text and pinning
  tests where none exist.
