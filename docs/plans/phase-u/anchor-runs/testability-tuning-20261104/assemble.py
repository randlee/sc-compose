import json, glob, hashlib, os, subprocess, sys
W, E = sys.argv[1], sys.argv[2]
c = lambda o: hashlib.sha256(json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()
ROLES = ("single_constraint", "observable", "faithful", "scoped", "self_contained")
oracle = json.load(open(E + "/oracle.json"))
s12 = json.load(open(E + "/score-1.json")); s2 = json.load(open(E + "/score-2.json")); s3 = json.load(open(E + "/score-3.json"))
cands = {**s12["candidates"], **s2["candidates"]}
b3 = json.load(open(E + "/bundles-r3.json")); Q = b3["bundles"]["S"]; prov = b3["provenance"]
inc = json.load(open(W + "/.claude/skills/plan-anchoring/assets/questions/testability.json"))
req = json.load(open(E + "/request.json"))
assert inc["questions"] == req["current_questions"]
dd = json.load(open(E + "/dedupe-score.json")); lint = json.load(open(E + "/deterministic-lint.json"))
def key(cid):
    v = cands[cid]
    return (not v["accepted"], -v["exact"], -v["stable"], v["below_ref_0_8"], -v["min_conf"], -v["mean_conf"], v["complexity_chars"], cid)
rankings = []
for role in ROLES:
    ids = sorted([k for k in cands if k.endswith(":" + role)], key=key)
    sel = "r1:b0" if prov[role].startswith("r1:b0") else prov[role]
    for rank, cid in enumerate(ids, 1):
        v = cands[cid]; label = cid.rsplit(":", 1)[0]
        if label == sel: why = "selected: accepted on all 17 cells; best confidence diagnostics among accepted candidates; reconfirmed in production-shape round 3"
        elif not v["accepted"]: why = "rejected: " + "; ".join(f"{m['case']} expected {m['expected']} got {m['calls']}" for m in v["mismatches"])
        else: why = "accepted but outranked on confidence diagnostics (below-0.8 count, min, mean) or complexity"
        rankings.append({"role": role, "rank": rank, "candidate_id": label + ("(incumbent)" if label == "r1:b0" else ""), "selected": label == sel,
                         "accepted": v["accepted"], "exact_cells": v["exact"], "confident_cells": v["confident"], "stable_cells": v["stable"], "total_cells": v["total"],
                         "third_calls": v["third"], "min_confidence": v["min_conf"], "mean_confidence": v["mean_conf"], "calls_below_0_8_reference": v["below_ref_0_8"],
                         "complexity_chars": v["complexity_chars"], "reason": why})
table = []
cells = {(x["case"], x["candidate"].split(":")[-1]): x for x in s3["cells"]}
for case in oracle["semantic_cases"]:
    row = {"case_id": case["case_id"], "requirement_form_lint": next(l["got"] for l in lint["semantic_cases"] if l["id"] == case["case_id"])}
    for role in ROLES:
        x = cells[(case["case_id"], role)]
        row[role] = {"expected": x["expected"], "calls": [[a, b] for a, b in x["calls"]], "cell": x["cell"]}
    table.append(row)
sem = {"exact_cells": sum(v["exact"] for k, v in s3["candidates"].items()), "confident_cells": sum(v["confident"] for v in s3["candidates"].values()),
       "stable_cells": sum(v["stable"] for v in s3["candidates"].values()), "total_cells": sum(v["total"] for v in s3["candidates"].values())}
third3 = sum(v["third"] for v in s3["candidates"].values())
inconc = sum(1 for x in s3["cells"] if x["cell"] == "inconclusive"); unst = sum(1 for x in s3["cells"] if x["cell"] == "unstable")
digest = c({"questions": Q})
runs = json.load(open(E + "/run-r1.json")), json.load(open(E + "/run-r2.json")), json.load(open(E + "/run-r3.json"))
ids = sorted(json.load(open(p))["run_id"] for p in glob.glob(E + "/responses/*.json"))
rel = lambda p: os.path.relpath(p, W)
data = {
 "operation": "testability", "gate": "testability", "accepted": True, "rubric_version": 2,
 "tournament_protocol_version": "plan-anchor/question-tournament/2.0.0", "production_confidence_threshold": 0.6, "confidence_reference_threshold": 0.8,
 "corpus_sha256": oracle["corpus_sha256"], "corpus_case_count": 17,
 "rubric_question_set": {"digest": digest, "questions": Q},
 "candidate_rankings": rankings, "acceptance_table": table,
 "confirmation_summary": {"two_call_cells": sem["total_cells"], "third_call_cells": third3, "inconclusive_cells": inconc, "unstable_cells": unst},
 "deterministic_lint": {"requirement_form_all_match": lint["all_match"], "semantic_cases_checked": len(lint["semantic_cases"]), "lint_cases_checked": len(lint["lint_cases"]),
     "note": "requirement-form lint matches every frozen expectation (a0-17 fails form as expected). a0-06 additionally trips the separate verbatim-quote check (statement equals quote); that is not a requirement-form rule and its corpus requirement_form_lint label is pass.",
     "evidence": rel(E + "/deterministic-lint.json")},
 "dedupe": {"question_set": "assets/questions/dedupe.json v2 unchanged (accepted; no tournament needed)", "compares": "statements only; quotes excluded",
     "rows": dd["rows"], "third_calls": dd["third_calls"], "evidence": rel(E + "/dedupe-score.json")},
 "evidence": {"directory": rel(E), "oracle": rel(E + "/oracle.json"), "oracle_sha256": open(E + "/oracle.sha256").read().strip(),
     "oracle_frozen_before_first_jev_call": True, "candidates": [rel(E + "/candidates-r1.json"), rel(E + "/candidates-r2.json")],
     "bundles": [rel(E + f"/bundles-r{n}.json") for n in (1, 2, 3)], "scores": [rel(E + "/score-1.json"), rel(E + "/score-2.json"), rel(E + "/score-3.json")],
     "raw_requests_dir": rel(E + "/requests"), "raw_responses_dir": rel(E + "/responses"), "harness": [rel(E + "/harness.py"), rel(E + "/score.py"), rel(E + "/dedupe.py"), rel(E + "/freeze.py")],
     "rounds": {"r1": runs[0], "r2": runs[1], "r3_seal_confirmation": runs[2]}, "jev_model_answered": "jev-1.14.0",
     "incumbent_rubric_sha256": c({"questions": inc["questions"]}),
     "incumbent_failures_on_frozen_corpus": {"observable": "a0-12-unrecorded-approval no@0.52/0.55 (below 0.6)", "faithful": "a0-02-valid-prohibition yes@0.50/0.68 (below 0.6)"},
     "selection_provenance": prov, "lowest_margin_seal_cells": [
        {"role": "faithful", "case": "a0-14-shared-quote-second-atomic", "calls": [0.66, 0.61]},
        {"role": "self_contained", "case": "a0-17-nondeclarative-form", "calls": [0.73, 0.68]},
        {"role": "self_contained", "case": "a0-12-unrecorded-approval", "calls": [0.74, 0.73]},
        {"role": "faithful", "case": "a0-01-valid-directive", "calls": [0.86, 0.76]}],
     "planner_observation": "untrusted; Phase U r-*/r2-* runs and cands*.json were not read or used as calibration truth"},
 "explanation": ("The incumbent rubric no longer passes the frozen calibration corpus on jev-1.14.0: observable (a0-12 at 0.52/0.55) and faithful (a0-02 at 0.50/0.68) fall below the 0.6 production floor, while the oracle labels are unchanged. "
     "That oracle mismatch, not the Phase U inconclusive count, justifies a new tournament. 24 role candidates (incumbent plus 3 new per role in round 1, 4 more faithful wordings in round 2) were scored on all 17 frozen cases with byte-identical paired calls. "
     "The top-ranked accepted candidate for each role was then reconfirmed as one production-shape five-question bundle (round 3), with 85/85 cells exact, confident and stable and no third calls; dedupe was 4/4 with the unchanged dedupe question; requirement-form lint matched every expectation. "
     "Sealed: single_constraint r1:b1, observable r1:b1, faithful r2:f6, scoped r1:b2, self_contained incumbent. The faithful and self_contained margins remain the thinnest (min 0.61 and 0.68). This calibration does not show that the 21 inconclusive Phase U anchors are rubric defects. Re-run them under the new seal; any that stay inconclusive go to evidence-bound testability triage, not further rubric tuning."),
 "authority_recommendations": [], "candidate_count": len(cands),
 "semantic_score": sem, "dedupe_case_count": 4,
 "dedupe_score": {"exact_cells": dd["score"]["exact"], "confident_cells": dd["score"]["confident"], "stable_cells": dd["score"]["stable"], "total_cells": 4},
 "repeat_requests_byte_identical": all(r["byte_identical_pairs"] for r in runs) and all(x["byte_identical"] for x in dd["rows"]),
}
git = subprocess.run(["git", "-C", W, "status", "--porcelain"], capture_output=True, text=True).stdout.splitlines()
unchanged = all("anchor-runs/testability-tuning-20261104" in l for l in git)
meta = {"duration_ms": int(sys.argv[3]), "tool_calls": int(sys.argv[4]), "retry_count": 0, "correlation_id": req["correlation_id"], "jev_run_ids": ids,
        "planner_observation_trust": "untrusted", "authoritative_sources_unchanged": unchanged}
vars_ = {"success": True, "canceled": False, "aborted_by": None, "data": data, "error": None, "metadata": meta}
json.dump(vars_, open(E + "/result-vars.json", "w"), indent=1, ensure_ascii=False)
print(digest, sem, len(cands), len(ids), unchanged)
