---
name: html-a-b-comparison
description: Build a local, self-contained HTML A/B comparison viewer for two or more generated HTML variants (e.g. different template revisions, different data sets, before/after fixes), so a human can pick between panes side-by-side without any external hosting.
---

# HTML A/B Comparison

## Step 1 -- Verify Installation

Run this before deciding the comparison axis or generating any variants. The
skill requires both `sc-compose` and `python3`:

```bash
set -eu

resolve_cli() {
  cli_name="$1"
  cli_path="$(command -v "$cli_name" 2>/dev/null || true)"
  if [ -z "$cli_path" ]; then
    for candidate in \
      "$HOME/.local/bin/$cli_name" \
      "$HOME/.cargo/bin/$cli_name" \
      "$PWD/target/release/$cli_name" \
      "$PWD/target/debug/$cli_name" \
      "/opt/homebrew/bin/$cli_name" \
      "/usr/local/bin/$cli_name" \
      "/usr/bin/$cli_name"; do
      if [ -x "$candidate" ]; then
        cli_path="$candidate"
        break
      fi
    done
  fi
  [ -n "$cli_path" ] || return 1
  printf '%s\n' "$cli_path"
}

export SC_COMPOSE_BIN="$(resolve_cli sc-compose)"
export PYTHON3_BIN="$(resolve_cli python3)"
"$SC_COMPOSE_BIN" --version
"$PYTHON3_BIN" --version
```

If either lookup or version check fails, stop and read
`references/installation-and-troubleshooting.md` before proceeding. Do not
silently substitute a different renderer or skip base64 encoding.

## Purpose

When a design decision needs a human to look at two or more full renders of
generated HTML side by side, build one local file, not a hosted page. Each
variant's full HTML is embedded as a base64 data URI inside `<iframe>`
elements; dropdowns pick which variant loads into the left/right pane. The
whole thing works fully offline — open it with the OS file opener (macOS:
`open <path>`) — no claude.ai Artifact, no dev server, no network calls.

Use this when comparing:

- the same data rendered through different template revisions (isolate the
  template as the only variable)
- different data through the same template (isolate the data)
- before/after a fix, same inputs

Never mix both axes in one comparison (different template AND different
data) — the viewer can't tell you which change caused what you're seeing.
Hold everything constant except the one thing under review.

## Workflow

1. **Decide the one axis you're comparing.** State it explicitly before
   building — e.g. "same `differential-probe` session data, five worker-panel
   template revisions." If you can't state a single axis, stop and split the
   comparison into more than one viewer instead of conflating variables.
2. **Produce each variant's full, standalone HTML.** Render through the real
   pipeline (`sc-compose render`, a background report-generator agent,
   whatever normally produces this artifact) — never hand-author the
   variant's markup. Each variant must be a complete HTML document
   (or fragment wrapped into one) that can render on its own inside an
   `<iframe>`.
3. **Base64-encode each variant** and inject it into the viewer template's
   data object (see Template below). Label each entry with something a human
   can tell apart at a glance (template SHA, revision name, dataset id) —
   not "variant 1" / "variant 2".
4. **Write the viewer file** under the caller's approved scratchpad or
   repository output root using the bundled template
   (`compare-viewer.html.j2`) or by adapting it inline. Validate the resolved
   output path before writing by running
   `references/validate-output-path.py` with the approved root and every
   output path. The validator rejects `..` traversal and symlink escapes and
   requires each resolved path to remain beneath the approved root. Create the
   parent only after validation; never write to an unvalidated path.
5. **Open it locally** with the platform's file opener. On macOS use
   `open <path>`, on Linux use `xdg-open <path>`, and on Windows use
   `start "" <path>` from `cmd.exe` (or `Start-Process <path>` in
   PowerShell). Do not upload it anywhere or use the `Artifact` tool — this
   viewer is intentionally a zero-network local file.
6. **Report the axis and the file path** back to the user; let them tell you
   which pane/variant they prefer rather than guessing from the diff
   yourself.

## Template

`compare-viewer.html.j2` in this skill directory is a generic two-pane
comparison shell. Render it with `sc-compose render` (or adapt by hand for a
one-off) using a JSON var-file shaped like:

```json
{
  "title": "Worker-panel template comparison (same data, 5 revisions)",
  "variants": [
    { "label": "51b37c5 — bundle worker panels", "b64": "<base64 of variant 1's full HTML>" },
    { "label": "d2acdb1 — HEAD (committed)", "b64": "<base64 of variant 2's full HTML>" }
  ],
  "default_left": 0,
  "default_right": 1
}
```

Notes on the shape:

- The var-file root must be an object. Its `variants` property is a flat array
  of objects, which renders cleanly through `sc-compose render --var-file`
  (top-level array values have been supported since FR-13 / PR #50). Do not
  make the var-file itself an array or nest `variants` under another key.
- `b64` values can be large; that's expected and fine — `sc-compose render`
  has no meaningful size limit on string variable values.
- The bundled viewer has exactly two panes, so provide exactly two variants
  per viewer. For a larger revision sweep, narrow the candidates first or
  create multiple viewers; do not pass 4-5 variants to this two-pane template.

## Producing the base64 payload

From a shell, after each variant's HTML file exists on disk:

```bash
"$PYTHON3_BIN" -c "import base64,sys; print(base64.b64encode(open(sys.argv[1],'rb').read()).decode())" variant.html
```

Build the JSON var-file with each variant's `label` and `b64`, then:

```bash
compare_dir="$("$PYTHON3_BIN" -c 'import tempfile; print(tempfile.mkdtemp(prefix="sc-compose-compare-"))')"
vars_path="$compare_dir/compare-vars.json"
viewer_path="$compare_dir/compare.html"
"$PYTHON3_BIN" .claude/skills/html-a-b-comparison/references/validate-output-path.py \
  "$compare_dir" "$vars_path" "$viewer_path"
"$SC_COMPOSE_BIN" render --file .claude/skills/html-a-b-comparison/compare-viewer.html.j2 \
  --var-file "$vars_path" --output "$viewer_path"
open "$viewer_path"  # use xdg-open or Start-Process on other platforms
```

The temporary directory above is created by Python's platform-appropriate
temporary-directory API. The same
[`validate-output-path.py`](references/validate-output-path.py) command is
required when the caller supplies an output root.

## Non-goals

- This is a comparison *tool*, not a report artifact — don't write comparison
  viewers into `site/reports/`; they belong in a scratchpad and are
  disposable once the decision is made.
- Not a substitute for real regression testing — it's for human visual
  judgment calls, not automated diffing.
