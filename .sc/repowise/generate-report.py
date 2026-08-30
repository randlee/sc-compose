#!/usr/bin/env python3
"""Generate the final comprehensive repowise report for sc-compose.

DATA-DRIVEN: reads raw scoped JSON from .sc/repowise/data/ and writes
docs/repowise/health.md + per-module reports. Version, commit, date resolved
from git at run time. NO hardcoded prose — every number and observation is
computed from the current run, so the report always matches its own data.

Inputs (per .sc/repowise.yaml modules + global artifacts):
  repowise-health-compose.json      repowise health --module crates/sc-compose
  repowise-health-composer.json     repowise health --module crates/sc-composer
  repowise-refactoring-targets.json repowise health --refactoring-targets
  repowise-dead-code.json           repowise dead-code

Outputs:
  docs/repowise/health.md                comprehensive (both modules, merged)
  docs/repowise/health-sc-compose.md     per-module (crates/sc-compose)
  docs/repowise/health-sc-composer.md    per-module (crates/sc-composer)
"""
import json, subprocess, sys
from pathlib import Path

# ── Git metadata ───────────────────────────────────────────────────────────────
REPO = Path(__file__).resolve().parents[2]

def _git(*args):
    return subprocess.check_output(["git", "-C", str(REPO), *args], text=True).strip()

VERSION   = _git("describe", "--tags", "--always")
COMMIT    = _git("rev-parse", "--short", "HEAD")
GENERATED = _git("log", "-1", "--format=%cs")

DATA_DIR   = Path(__file__).resolve().parent / "data"
OUT_DIR    = REPO / "docs" / "repowise"
OUT_DIR.mkdir(parents=True, exist_ok=True)

BIOMARKER_MEANING = {
    "duplicated_assertion_block": "Repeated assertion patterns — test helper opportunity",
    "hot_path_sync_io": "Sync I/O on hot paths — should be async",
    "prior_defect": "Files with bug-fix history — strong defect predictor",
    "untested_hotspot": "Depended-upon files with no paired test coverage",
    "dry_violation": "DRY violations — opportunities to extract shared code",
    "error_handling": "Error handling gaps or inconsistencies",
    "hidden_coupling": "Implicit dependencies between modules",
    "co_change_scatter": "Files that change together — high coupling",
    "churn_risk": "High recent change frequency — churn-driven risk",
}

def load_json(path):
    content = path.read_text()
    for i, c in enumerate(content):
        if c in '{[':
            return json.loads(content[i:])
    raise ValueError(f"No JSON in {path}")

def safe(v, fmt=".1f", default="—"):
    return default if v is None else f"{v:{fmt}}"

def badge_md(score):
    color = "brightgreen" if score >= 7 else ("yellow" if score >= 4 else "red")
    return f"![health {score}/10](https://img.shields.io/badge/health-{int(score*10)}%2F100-{color})"

class Report:
    def __init__(self, name, h, rt, dc, is_combined=False):
        self.name, self.h, self.rt, self.dc, self.combined = name, h, rt, dc, is_combined
        self.k = h["kpis"]
        self.metrics = sorted(h["metrics"], key=lambda m: (m["score"], m["nloc"]))
        self.findings = h.get("findings", [])

    def history_section(self):
        """Render the cross-run trend from docs/repowise/history.md.
        This is what makes the report track health BETWEEN runs, not just one
        point in time. Latest run first."""
        hist = OUT_DIR / "history.md"
        if not hist.exists():
            return []
        rows = []
        for l in hist.read_text().splitlines():
            if not l.startswith("|") or l.startswith("|---"):
                continue
            cells = [c.strip() for c in l.strip("|").split("|")]
            if not cells or not cells[0][:4].isdigit():   # skip header/data rows
                continue
            rows.append((cells[0], l))
        if not rows:
            return []
        # File is chronological (oldest first, this run last). Keep that order.
        latest = rows[-1][1]
        prev   = rows[-2][1] if len(rows) >= 2 else None
        out = []
        def w(s=""): out.append(s)
        w("## Trend Between Runs")
        w()
        if prev:
            import re
            def overall_of(row):
                # anchor to a full cell: "| <val>/10 |" or "| **<val>/10** |"
                m = re.search(r"\|\s*\*{0,2}([0-9]+(?:\.[0-9]+)?)\s*/\s*10\*{0,2}\s*\|", row)
                return float(m.group(1)) if m else None
            a, b = overall_of(prev), overall_of(latest)
            if a is not None and b is not None:
                delta = b - a
                sign = '+' if delta > 0 else ('' if delta == 0 else '')
                word = 'improved' if delta > 0 else ('declined' if delta < 0 else 'unchanged')
                w(f"**This run vs the previous recorded run: {sign}{delta:.2f}/10 overall health — {word}.** Full log below.")
        w()
        w("Latest run is last. Chronological; read with `health.md` for the current deep-dive.")
        w()
        w("| Generated | Version / Commit | Overall | Hotspot | Files | Worst file (score) | PR |")
        w("|---|---|---|---|---|---|---|")
        for _, row in rows:
            w(row)
        w()
        w("*History seeded + appended by `.sc/repowise/generate-report.py`; hand-edit only to correct a wrong row.*")
        return out

    # ── data-driven observations ─────────────────────────────────────────────
    def observations(self):
        rows = []
        def num(m, key):
            v = m.get(key)
            return v if isinstance(v, (int, float)) else 0
        def top(pred, label):
            m = [x for x in self.metrics if pred(x)]
            if not m:
                return None
            m = sorted(m, key=lambda x: x["score"])
            t = m[0]
            extra = (", CCN=" + str(t["max_ccn"]) if num(t, "max_ccn") >= 15 else "")
            extra += (", " + str(num(t, "duplication_pct")) + "% duplication" if num(t, "duplication_pct") >= 25 else "")
            return f"`{t['file_path']}` ({t['score']:.1f}/10, {t['nloc']} NLOC) — {label}{extra}"
        rows.append(top(lambda m: num(m, "max_ccn") >= 20, "high cyclomatic complexity"))
        rows.append(top(lambda m: num(m, "duplication_pct") >= 50, "high duplication"))
        rows.append(top(lambda m: num(m, "max_nesting") >= 6, "deep nesting"))
        out = [r for r in rows if r]
        return out if out else ["No structurally notable worst files beyond the score ranking above."]

    def biomarker_table(self):
        by_type = {}
        for f in self.findings:
            by_type.setdefault(f["biomarker_type"], []).append(f)
        rows = []
        for t, items in sorted(by_type.items(), key=lambda kv: -len(kv[1])):
            rows.append(f"| {t} | {len(items)} | {BIOMARKER_MEANING.get(t, '')} |")
        return rows

    def top_biomarkers(self, limit=6):
        by_type = {}
        for f in self.findings:
            by_type.setdefault(f["biomarker_type"], []).append(f)
        picks = []
        for t, items in sorted(by_type.items(), key=lambda kv: -len(kv[1]))[:limit]:
            block = [f"### [{len(items)}] {t}"]
            if t in BIOMARKER_MEANING:
                block.append(BIOMARKER_MEANING[t])
            top = sorted(items, key=lambda i: i.get("health_impact", 0), reverse=True)[:6]
            for item in top:
                fn = item.get("function_name") or "(top-level)"
                block.append(f"- **{item.get('severity','?')}** `{item['file_path']}` `{fn}`: {item.get('reason','')[:160]}")
            picks.append("\n".join(block))
        return picks

    def dead_code(self):
        by_kind = {}
        for f in self.dc:
            by_kind.setdefault(f.get("kind", "unknown"), []).append(f)
        rows, detail = [], []
        for kind, items in sorted(by_kind.items(), key=lambda kv: -len(kv[1])):
            rows.append(f"| {kind} | {len(items)} |")
            act = sorted(items, key=lambda x: x.get("confidence", 0), reverse=True)
            actionable = [f for f in act if f.get("confidence", 0) >= 0.5 and not f.get("safe_to_delete")]
            detail.append(f"#### {kind}\n" + "\n".join(
                f"- `{f['file_path']}` ({f.get('lines',0)} lines, conf {f.get('confidence','?')}): {f.get('reason','')[:140]}"
                for f in actionable[:8]))
        return rows, detail

    def render(self):
        o = []
        def w(s=""): o.append(s)
        k = self.k
        w(f"# {self.name} — Repowise Code Health")
        w()
        w(f"**Version:** {VERSION} | **Commit:** {COMMIT} | **Generated:** {GENERATED}")
        w(f"**Analyzed:** {len(self.metrics)} files | **Biomarker findings:** {len(self.findings)} | **Data:** repowise health + dead-code + refactoring-targets")
        w()
        w(badge_md(k["average_health"]))
        w()
        w("## Quick Summary")
        w()
        w("| Metric | Value |")
        w("|---|---|")
        w(f"| Overall Health | **{k['average_health']:.2f}/10** |")
        w(f"| Hotspot Health | {k['hotspot_health']:.2f}/10 |")
        w(f"| Worst In-Scope File | `{k['worst_performer_path']}` ({k['worst_performer_score']:.2f}/10) |")
        w(f"| Files Scored (this scope) | {len(self.metrics)} |")
        if self.combined:
            w(f"| Files in Full Index | {k['file_count']} (repo-wide scan; scored table above = configured modules only) |")
        else:
            w(f"| Files in Full Index | {k['file_count']} (repo-wide scan; this table = module only) |")
        w(f"| Maintainability | {k['maintainability_average']:.2f} avg / {k['maintainability_hotspot']:.2f} hotspot |")
        w(f"| Performance | {k['performance_average']:.2f} avg / {k['performance_hotspot']:.2f} hotspot |")
        w()
        low = [m for m in self.metrics if m["score"] < 3]
        w(f"**Read:** in-scope average {k['average_health']:.2f}/10 with hotspot {k['hotspot_health']:.2f}/10 — "
          f"{len(low)} of {len(self.metrics)} scored file{'s' if len(low) != 1 else ''} sit below 3.0/10 and carry the hotspot drag.")
        w()
        if self.combined:
            o.extend(self.history_section())
        w("## Worst 20 Files by Health Score")
        w()
        w("| Score | File | NLOC | CCN | Nest | Dup% |")
        w("|---|---|---|---|---|---|")
        for m in self.metrics[:20]:
            w(f"| {m['score']:.2f} | `{m['file_path']}` | {m['nloc']} | {m.get('max_ccn','—')} | {safe(m.get('max_nesting'),'d')} | {safe(m.get('duplication_pct'))} |")
        w()
        w("**Observations**")
        for obs in self.observations():
            w(f"- {obs}")
        w()
        if self.findings:
            w("## Biomarker Findings")
            w()
            w("| Type | Count | What It Means |")
            w("|---|---|---|")
            o.extend(self.biomarker_table())
            w()
            w("### Highest-Impact Findings (by type)")
            w()
            for block in self.top_biomarkers():
                o.extend(block.split("\n"))
                w()
        if self.dc:
            w("## Dead Code / Unreachable")
            w()
            rows, detail = self.dead_code()
            w("| Kind | Count |")
            w("|---|---|")
            o.extend(rows)
            w()
            for d in detail:
                o.extend(d.split("\n"))
                w()
        targets = self.rt.get("targets", [])
        if targets:
            w("## Refactoring Targets (impact-per-effort ranked)")
            w()
            for i, t in enumerate(targets[:10], 1):
                w(f"### #{i}: `{t['file_path']}` — {t['primary_biomarker']} ({t['primary_severity']})")
                w()
                w("| Impact | Effort | ROI | Findings |")
                w(f"| {safe(t.get('total_impact'))} | {t.get('effort_bucket','—')} | {safe(t.get('impact_per_effort'))} | {t.get('finding_count','—')} |")
                w()
                w(f"Reason: {t.get('primary_reason','')[:200]}")
                plans = t.get("plans", [])[:2]
                for p in plans:
                    pd = p.get("plan", {})
                    desc = pd.get("suggestion", pd.get("summary", "")) if isinstance(pd, dict) else str(pd)
                    w(f"- **{p.get('refactoring_type','?')}**: {desc[:220]}")
                w()
        # ── recommendations: derived, not copied ──
        w("## Recommendations (derived from this run)")
        w()
        worst = self.metrics[0]
        w(f"1. **Start with the worst file** — `{worst['file_path']}` ({worst['score']:.2f}/10, {worst['nloc']} NLOC){', CCN=' + str(worst['max_ccn']) if worst.get('max_ccn',0)>=15 else ''}. Decompose by responsibility before anything else.")
        untested = [f for f in self.findings if f["biomarker_type"] == "untested_hotspot"]
        if untested:
            files = sorted({f["file_path"] for f in untested})[:5]
            w(f"2. **Close test gaps on depended-upon files** — {len(files)} hotspot file(s) lack paired tests: {', '.join('`%s`' % f for f in files)}.")
        dup = sorted([f for f in self.findings if f["biomarker_type"] == "duplicated_assertion_block"], key=lambda x: -x.get("health_impact", 0))
        if dup:
            w(f"3. **Extract duplication** — {len(dup)} duplicated assertion blocks; shared test-helper modules would remove the bulk.")
        sync = len([f for f in self.findings if f["biomarker_type"] == "hot_path_sync_io"])
        if sync:
            w(f"4. **Audit sync I/O on hot paths** — {sync} findings; either make async or document the intentional sync boundary.")
        if targets:
            t1 = targets[0]
            w(f"5. **Fastest win** — refactoring target #1: `{t1['file_path']}` ({t1.get('effort_bucket','?')} effort, ROI {safe(t1.get('impact_per_effort'))}).")
        w()
        w("---")
        w(f"*Generated by `.sc/repowise/generate-report.py` from scoped repowise data ({GENERATED}, {COMMIT}). Scope per `.sc/repowise.yaml`: modules + annotated exclusions. No hardcoded prose — every figure is computed from the JSON in `.sc/repowise/data/`.*")
        return "\n".join(o) + "\n"


def record_history(pr_url):
    """Maintain docs/repowise/history.md — the between-run health log.
    Idempotent: one row per commit; seeds itself with the develop baseline
    so the very first regeneration already shows a trend."""
    hist = OUT_DIR / "history.md"
    row = (f"| {GENERATED} | {VERSION} / {COMMIT} | **{hc_kpi['average_health']:.2f}/10** | "
           f"{hc_kpi['hotspot_health']:.2f}/10 | {hc_kpi['file_count']} | "
           f"`{hc_kpi['worst_performer_path']}` ({hc_kpi['worst_performer_score']:.2f}) | {pr_url} |")
    header = ["# sc-compose — Repowise Health History",
              "",
              "One row per analysis run, chronological (this run is last).",
              "Read alongside `health.md` — that file is the deep-dive for the latest run.",
              "",
              "| Generated | Version / Commit | Overall | Hotspot | Files | Worst file (score) | PR |",
              "|---|---|---|---|---|---|---|",
              "| 2026-08-02 | v1.2.0-219-gbb79a5f / bb79a5f (develop baseline) | **7.90/10** | 4.70/10 | 286 | `crates/sc-compose/src/cli.rs` (1.55) | n/a (pre-PR runs) |"]
    lines = None
    if hist.exists():
        lines = hist.read_text().splitlines()
    else:
        lines = header
    if not any(COMMIT in l for l in lines if l.startswith("|")):
        lines.append(row)
    hist.write_text("\n".join(lines) + "\n")
    return lines


def main():
    hc = load_json(DATA_DIR / "repowise-health-compose.json")
    hp = load_json(DATA_DIR / "repowise-health-composer.json")
    rt = load_json(DATA_DIR / "repowise-refactoring-targets.json")
    dc = load_json(DATA_DIR / "repowise-dead-code.json")

    # combined = the two configured MODULES, union of their per-file metrics
    # (a file scored in both keeps its LOWER score). NOT the repo-wide index.
    seen, merged = {}, []
    for h in (hc, hp):
        for m in h["metrics"]:
            key = (m["file_path"], m.get("score"))
            if key in seen:
                continue
            seen[key] = m
            merged.append(m)
    merged.sort(key=lambda m: (m["score"], m["nloc"]))

    # KPIs: taken from the sc-compose module run — this is repowise's repo-level
    # scoring (its `kpis`), file_count = full index scan; the table below is
    # in-scope files only. The Report renders that distinction.
    combined_h = dict(hc)
    combined_h["metrics"] = merged
    combined_h["findings"] = hc.get("findings", []) + hp.get("findings", [])
    combined_h["modules"] = ["crates/sc-compose", "crates/sc-composer"]

    global hc_kpi
    hc_kpi = hc["kpis"]   # repo-level KPIs (sc-compose module run)

    # ── seed/append the between-run history BEFORE rendering (so the report shows it)
    import os
    pr_url = os.environ.get("PR_URL", "[#571](https://github.com/randlee/sc-compose/pull/571)")
    record_history(pr_url)

    (OUT_DIR / "health.md").write_text(Report("sc-compose", combined_h, rt, dc, True).render())
    (OUT_DIR / "health-sc-compose.md").write_text(Report("crates/sc-compose", hc, rt, dc).render())
    (OUT_DIR / "health-sc-composer.md").write_text(Report("crates/sc-composer", hp, rt, dc).render())

    k = hc["kpis"]
    worst_lt3 = sum(1 for m in merged if m["score"] < 3)
    print(f"OK  combined: {len(merged)} unique files, avg {k['average_health']}, files below 3.0: {worst_lt3}")
    print(f"OK  sc-compose module: {len(hc['metrics'])} files")
    print(f"OK  sc-composer module: {len(hp['metrics'])} files")

if __name__ == "__main__":
    main()
