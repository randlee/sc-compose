# Graph contract fixtures

`captures.json` indexes production bd v1.3.1 `cook <path> --json` captures
for every ADR-0023 Formula grammar row, including inheritance, loop and
expansion. Each `.formula.json` is the exact input; `.capture.json` stores
exit status, stdout and stderr; `.cooked.json` is successful stdout for a
fake process runner. Only the absolute capture workspace is replaced with
`<workspace>`. The production archive checksum is in the index.

Reproduce in an isolated initialized bd workspace: put `base.formula.json`
and `exp.formula.json` in `.beads/formulas/`, then run `bd cook <input> --json`
for each indexed input. Do not supply --var or --persist. No graph writes
are needed to obtain these parser fixtures.

The capture evidence distinguishes parser rejection from engine validation:
- Duplicate ids and unknown dependencies fail `bd cook` with no JSON output;
  use their capture status/stderr for cook-failure tests.
- Unknown top-level and step fields are discarded by bd v1.3.1. Engine tests
  for future unknown parsed keys must explicitly add such keys to a copy of
  the successful capture; that mutation is synthetic, not captured output.
- `bead_variables_set` is a request precondition; its cooked formula is
  ordinary accepted content. Supply non-empty request bead_variables in the
  engine test. The captures do not pretend this request field is formula JSON.
- Extends merges steps; loop and expansion materialize their resulting ids.
  The resolved output, not the original construct, is checked by the engine.

The four `receipt-*.json` files are hand-authored wire-contract specimens,
not captured execution results. They cover graph pour, registry pour and the
two refusal codes for downstream conversion tests. Refusal fixtures use the
existing preview_pour operation because t-1 intentionally does not publish
attach operation/stage variants (t-2 adds them with execution). They test the
transport of a refusal code, not which operation can produce it at runtime.
