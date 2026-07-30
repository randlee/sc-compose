---
name: html-a-b-comparison
description: Build a local, self-contained HTML A/B comparison viewer for two or more generated HTML variants (e.g. different template revisions, different data sets, before/after fixes), so a human can pick between panes side-by-side without any external hosting.
---

# HTML A/B Comparison

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
4. **Write the viewer file** to the caller's scratchpad directory (or
   wherever the caller specifies) using the bundled template
   (`compare-viewer.html.j2`) or by adapting it inline.
5. **Open it locally**: `open <path>` on macOS. Do not upload it anywhere,
   do not use the `Artifact` tool for this — this pattern exists specifically
   because hosted-artifact URLs are not always usable for the intended
   viewer, and the whole point is a file that opens with zero network
   dependency.
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

- `variants` is a flat, top-level array of objects — this renders cleanly
  through `sc-compose render --var-file` (top-level array-of-objects has
  been supported since FR-13 / PR #50). Do not nest `variants` under another
  key.
- `b64` values can be large; that's expected and fine — `sc-compose render`
  has no meaningful size limit on string variable values.
- Keep `variants` to a number a human can actually compare — 2 is the normal
  case, up to 4-5 is reasonable for a revision sweep. Beyond that, consider
  narrowing the field first (e.g. drop candidates that are obviously out)
  rather than dumping everything into one viewer.

## Producing the base64 payload

From a shell, after each variant's HTML file exists on disk:

```bash
python3 -c "import base64,sys; print(base64.b64encode(open(sys.argv[1],'rb').read()).decode())" variant.html
```

Build the JSON var-file with each variant's `label` and `b64`, then:

```bash
sc-compose render --file .claude/skills/html-a-b-comparison/compare-viewer.html.j2 \
  --var-file /tmp/compare-vars.json --output /tmp/compare.html
open /tmp/compare.html
```

## Non-goals

- This is a comparison *tool*, not a report artifact — don't write comparison
  viewers into `site/reports/`; they belong in a scratchpad and are
  disposable once the decision is made.
- Not a substitute for real regression testing — it's for human visual
  judgment calls, not automated diffing.
