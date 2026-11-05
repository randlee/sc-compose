#!/usr/bin/env python3
"""Score tournament rounds against the frozen oracle. Usage: score.py <evidence_dir> <round> [<round> ...]"""
import glob
import json
import os
import sys

E = sys.argv[1]
ROUNDS = sys.argv[2:]
ROLES = ("single_constraint", "observable", "faithful", "scoped", "self_contained")
oracle = json.load(open(E + "/oracle.json"))
out = {"rounds": ROUNDS, "candidates": {}, "cells": []}
for rnd in ROUNDS:
    bundles = json.load(open(f"{E}/bundles-r{rnd}.json"))["bundles"]
    for bid in sorted(bundles):
        for role in ROLES:
            if role not in bundles[bid]:
                continue
            cand = f"r{rnd}:{bid}:{role}"
            st = {"exact": 0, "confident": 0, "stable": 0, "total": 0, "third": 0, "confs": [], "mismatches": []}
            for case in oracle["semantic_cases"]:
                base = f"{E}/responses/phase-u_beads-anchors_testability_r{rnd}_{bid}_{case['case_id']}"
                calls = []
                for n in (1, 2, 3):
                    p = f"{base}_call{n}.json"
                    if os.path.exists(p):
                        a = json.load(open(p))["response"]["answers"][role]
                        calls.append((a["choice"], a["confidence"]))
                exp = case["expected"][role]
                c1, c2 = calls[0], calls[1]
                stable = c1[0] == c2[0]
                exact = stable and c1[0] == exp
                conf = min(c1[1], c2[1]) >= 0.6
                st["total"] += 1
                st["stable"] += stable
                st["exact"] += exact
                st["confident"] += conf
                st["third"] += len(calls) == 3
                st["confs"] += [c1[1], c2[1]]
                if not (exact and conf):
                    st["mismatches"].append({"case": case["case_id"], "expected": exp, "calls": calls})
                if exact and conf:
                    cell = "accepted"
                elif not stable:
                    cell = "unstable"
                elif c1[0] != exp and conf:
                    cell = "needs_work"
                else:
                    cell = "inconclusive" if not conf else "needs_work"
                out["cells"].append({"candidate": cand, "case": case["case_id"], "expected": exp, "calls": calls, "cell": cell})
            cs = st.pop("confs")
            st["min_conf"] = min(cs)
            st["mean_conf"] = round(sum(cs) / len(cs), 4)
            st["below_ref_0_8"] = sum(c < 0.8 for c in cs)
            st["accepted"] = st["exact"] == st["confident"] == st["stable"] == st["total"]
            st["complexity_chars"] = len(json.dumps(bundles[bid][role], sort_keys=True))
            out["candidates"][cand] = st
json.dump(out, open(f"{E}/score-{'-'.join(ROUNDS)}.json", "w"), indent=1)
for k, v in out["candidates"].items():
    print(k, {x: v[x] for x in ("accepted", "exact", "confident", "stable", "min_conf", "mean_conf", "below_ref_0_8", "complexity_chars")},
          [(m["case"], m["expected"], m["calls"]) for m in v["mismatches"]])
