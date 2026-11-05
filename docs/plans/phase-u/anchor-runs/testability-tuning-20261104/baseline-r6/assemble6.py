import json, glob, os, subprocess, sys, hashlib
W, E = sys.argv[1:3]
c = lambda o: hashlib.sha256(json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()
ROLES = ("single_constraint", "observable", "faithful", "scoped", "self_contained")
rel = lambda p: os.path.relpath(p, W)
o = json.load(open(E + "/oracle.json")); osha = open(E + "/oracle.sha256").read().strip()
assert c(o) == osha
req = json.load(open(E + "/request.json"))
R = ("6", "6b", "6c")
sc = {r: json.load(open(f"{E}/score-{r}.json")) for r in R}
B = {r: json.load(open(f"{E}/bundles-r{r}.json"))["bundles"] for r in R}
# distinct candidate texts: first occurrence defines the candidate id
seen = {}; rk = []
for r in R:
    for cid, v in sc[r]["candidates"].items():
        _, bid, role = cid.split(":")
        d = c(B[r][bid][role])
        if (role, d) in seen:
            seen[(role, d)]["repeat_evaluations"].append({"round": "r" + r, "bundle": bid, "accepted": v["accepted"], "exact": v["exact"], "confident": v["confident"], "stable": v["stable"], "min_confidence": v["min_conf"],
                "mismatches": [[m["case"], m["expected"], m["calls"]] for m in v["mismatches"]]})
            continue
        rec = {"role": role, "candidate_id": f"r{r}:{bid}:{role}", "question_sha256": d, "accepted": v["accepted"], "exact_cells": v["exact"], "confident_cells": v["confident"],
               "stable_cells": v["stable"], "total_cells": v["total"], "third_calls": v["third"], "min_confidence": v["min_conf"], "mean_confidence": v["mean_conf"],
               "calls_below_0_8_reference": v["below_ref_0_8"], "complexity_chars": v["complexity_chars"],
               "mismatches": [[m["case"], m["expected"], m["calls"]] for m in v["mismatches"]], "repeat_evaluations": [],
               "origin": "installed seal 28ce3aec v5 (current_questions)" if bid == "b0" else "new wording"}
        seen[(role, d)] = rec; rk.append(rec)
for x in rk:
    allacc = x["accepted"] and all(e["accepted"] for e in x["repeat_evaluations"])
    x["accepted_every_evaluation"] = allacc
rk.sort(key=lambda r: (r["role"], not r["accepted_every_evaluation"], -r["exact_cells"], -r["confident_cells"], -r["stable_cells"], r["calls_below_0_8_reference"], -r["min_confidence"], r["complexity_chars"], r["candidate_id"]))
best = {}
for x in rk:
    best.setdefault(x["role"], x)
for i, x in enumerate(rk, 1):
    x["rank"] = i
    x["selected"] = False
    if best[x["role"]] is x and x["accepted_every_evaluation"]:
        x["reason"] = "best-ranked accepted wording for its role; not sealed because the rubric as a whole is not accepted (faithful has no accepted candidate)"
    elif x["accepted_every_evaluation"]:
        x["reason"] = "accepted but outranked"
    else:
        bad = x["mismatches"] + [m for e in x["repeat_evaluations"] for m in e["mismatches"]]
        x["reason"] = "rejected: " + "; ".join(f"{m[0]} expected {m[1]} got {m[2]}" for m in bad)
# acceptance table: composite bundle r6c:d1 (best wording for every role, evaluated together)
cells = {(x["case"], x["candidate"].split(":")[-1]): x for x in sc["6c"]["cells"] if x["candidate"].startswith("r6c:d1:")}
base = {(x["case"], x["candidate"].split(":")[-1]): x for x in sc["6"]["cells"] if x["candidate"].startswith("r6:b0:")}
def row(cs, case):
    cid = case["case_id"]; r = {"case_id": cid, "origin": {"a0": "base-v2", "a3": "a3-extension", "a4": "a4-extension", "a5": "a5-extension"}[cid[:2]]}
    for role in ROLES:
        x = cs[(cid, role)]; r[role] = {"expected": x["expected"], "calls": [[a, b] for a, b in x["calls"]], "cell": x["cell"]}
    return r
table = [row(cells, k) for k in o["semantic_cases"]]
btable = [row(base, k) for k in o["semantic_cases"]]
D = [v for k, v in sc["6c"]["candidates"].items() if k.startswith("r6c:d1:")]
dc = [x for x in sc["6c"]["cells"] if x["candidate"].startswith("r6c:d1:")]
sem = {"exact_cells": sum(v["exact"] for v in D), "confident_cells": sum(v["confident"] for v in D), "stable_cells": sum(v["stable"] for v in D), "total_cells": sum(v["total"] for v in D)}
B0 = [v for k, v in sc["6"]["candidates"].items() if k.startswith("r6:b0:")]
bsem = {"exact_cells": sum(v["exact"] for v in B0), "confident_cells": sum(v["confident"] for v in B0), "stable_cells": sum(v["stable"] for v in B0), "total_cells": sum(v["total"] for v in B0)}
dd = json.load(open(E + "/../dedupe-score.json"))
runs = {r: json.load(open(f"{E}/run-r{r}.json")) for r in R}
ids = sorted(json.load(open(p))["run_id"] for p in glob.glob(E + "/responses/*.json"))
cur = req["current_questions"]
recs = [{"kind": "corpus_case_review", "applies_to": "a5-01-tagged-release-baseline", "authority_version": f"{rel(E + '/oracle.json')} sha256 {osha} (calibration corpus v2, a5 extension frozen 2026-11-04)",
  "ambiguity": "Expected faithful=yes, but the statement adds '(the formatter keeps the 2.4.0 line-wrapping rules)', which the quote never mentions, and narrows 'every library function and CLI command ... same output bytes, same exit codes' to formatted output only. Five distinct faithful wordings over seven evaluations failed this cell: the installed seal, c1 and c2 answered yes at 0.28-0.43; b1 split no/yes/no; b2 answered no twice (0.55/0.56). The case pairs an unsupported elaboration with the label it is meant to calibrate.",
  "proposed_text": "In a new corpus version, either drop the parenthetical from the a5-01 statement (keeping faithful=yes), or keep it and relabel faithful=no with a matching rationale. Do not relabel the frozen v2 case.",
  "meaning_unchanged_because": "The case targets self_contained/observable for a tagged-release baseline; removing the unsupported parenthetical leaves that intent unchanged.",
  "requires_user_approval": True}]
data = {
 "operation": "testability", "gate": "testability", "accepted": False, "rubric_version": 2,
 "tournament_protocol_version": req["tournament_protocol_version"], "production_confidence_threshold": req["production_confidence_threshold"], "confidence_reference_threshold": req["confidence_reference_threshold"],
 "corpus_sha256": osha, "corpus_case_count": len(o["semantic_cases"]), "rubric_question_set": None,
 "candidate_rankings": rk, "acceptance_table": table,
 "confirmation_summary": {"two_call_cells": sem["total_cells"], "third_call_cells": sum(v["third"] for v in D),
     "inconclusive_cells": sum(x["cell"] == "inconclusive" for x in dc), "unstable_cells": sum(x["cell"] == "unstable" for x in dc)},
 "deterministic_lint": {"requirement_form_all_match": all(x["got"] == x["expected"] for x in o["extension_lint"]), "extension_lint": o["extension_lint"], "base_evidence": rel(E + "/../deterministic-lint.json")},
 "dedupe": {"note": "dedupe question and pairs unchanged; prior byte-identical paired result reused", "rows": dd["rows"], "evidence": rel(E + "/../dedupe-score.json")},
 "evidence": {"directory": rel(E), "corpus": rel(E + "/oracle.json"), "corpus_sha256": osha, "oracle_frozen_before_first_jev_call": True,
   "request": rel(E + "/request.json"), "request_canonical_sha256": c(req),
   "installed_seal_current_questions_canonical_sha256": c(cur), "installed_seal_reported_by_planner": "28ce3aec (version 5)",
   "acceptance_table_source": "r6c:d1 composite bundle (best wording per role evaluated together)",
   "baseline_installed_seal": {"bundle": "r6:b0", "semantic_score": bsem, "acceptance_table": btable,
       "failing_cells": [[k, m] for k, v in sc["6"]["candidates"].items() if k.startswith("r6:b0:") for m in v["mismatches"]]},
   "best_composite_questions": {"digest_if_sealed": c({"questions": B["6c"]["d1"]}), "questions": B["6c"]["d1"], "status": "not sealed; faithful a5-01 inconclusive"},
   "bundles": [rel(f"{E}/bundles-r{r}.json") for r in R], "scores": [rel(f"{E}/score-{r}.json") for r in R],
   "scripts": [rel(E + "/harness.py"), rel(E + "/score.py"), rel(E + "/assemble6.py")],
   "raw_requests_dir": rel(E + "/requests"), "raw_responses_dir": rel(E + "/responses"), "rounds": runs,
   "planner_observation": "none supplied"},
 "explanation": ("Not accepted. The installed seal (b0) scored 179/180 exact, 176/180 confident, all stable. It went inconclusive on observable a5-03 (yes 0.52/0.48), faithful a5-01 (yes 0.40/0.28) and a0-14 (yes 0.66/0.58). Self_contained missed a5-03 (no 0.28/0.30, low confidence). "
   "Over three rounds (r6, r6b, r6c) I tested 23 distinct wordings for the five roles. Single_constraint and scoped b0 were accepted every time. Observable r6b:c1:observable was accepted in all three evaluations (min 0.66); it adds that named records and inline-listed approved or exception sets are identified. "
   "Self_contained r6c:d1:self_contained was accepted at min 0.60, exactly the threshold, so its margin is thin. Faithful has no accepted wording: a5-01 stayed at 0.28-0.43 in all seven faithful evaluations, b1 split, and b2 answered no twice. "
   "The a5-01 statement adds a parenthetical that is not in the quote ('keeps the 2.4.0 line-wrapping rules'). I recommend a corpus revision, not further faithful tuning. The frozen labels were not changed. Composite d1 scored 180/180 exact, 179/180 confident, all stable. No rubric is sealed."),
 "authority_recommendations": recs, "candidate_count": len(rk), "semantic_score": sem, "dedupe_case_count": len(dd["rows"]),
 "dedupe_score": {"exact_cells": dd["score"]["exact"], "confident_cells": dd["score"]["confident"], "stable_cells": dd["score"]["stable"], "total_cells": len(dd["rows"])},
 "repeat_requests_byte_identical": all(r["byte_identical_pairs"] for r in runs.values()),
}
git = subprocess.run(["git", "-C", W, "status", "--porcelain"], capture_output=True, text=True).stdout.splitlines()
meta = {"duration_ms": int(sys.argv[3]), "tool_calls": int(sys.argv[4]), "retry_count": 0, "correlation_id": req["correlation_id"], "jev_run_ids": ids,
        "planner_observation_trust": None, "authoritative_sources_unchanged": all("anchor-runs/testability-tuning-20261104" in l for l in git)}
json.dump({"success": True, "canceled": False, "aborted_by": None, "data": data, "error": None, "metadata": meta}, open(E + "/result-vars.json", "w"), indent=1, ensure_ascii=False)
print(len(rk), sem, bsem, len(ids), meta["authoritative_sources_unchanged"], data["confirmation_summary"], git[:8])
