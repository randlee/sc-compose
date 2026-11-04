import json, glob, os, subprocess, sys
W, E, E4 = sys.argv[1:4]
ROLES = ("single_constraint", "observable", "faithful", "scoped", "self_contained")
rel = lambda p: os.path.relpath(p, W)
o = json.load(open(E4 + "/oracle.json")); osha = open(E4 + "/oracle.sha256").read().strip()
sc = {r: json.load(open(f"{E4}/score-{r}.json")) for r in ("r4a", "r4b", "r4conf")}
dd = json.load(open(E + "/dedupe-score.json")); req = json.load(open(E + "/request.json"))
rk = []
for rnd in ("r4a", "r4b"):
    for cid, v in sc[rnd]["candidates"].items():
        b, role = cid.split(":")[1], cid.split(":")[2]
        name = ("sealed_3f1c0f65" if b == "sealed" else b.split("-", 1)[1])
        lead = (role, name) in (("self_contained", "s19"), ("faithful", "f8"))
        reason = ("rejected: " + "; ".join(f"{m['case']} expected {m['expected']} got {m['calls']}" for m in v["mismatches"])) if not v["accepted"] else \
                 ("leading: accepted 32/32 and in five-question confirmation; not sealed because sealed observable fails a4-05" if lead else ("baseline sealed question, passes v4" if name == "sealed_3f1c0f65" else "accepted but outranked on confidence diagnostics"))
        rk.append({"role": role, "round": rnd, "candidate_id": name, "selected": False, "leading": lead, "accepted": v["accepted"], "exact_cells": v["exact"], "confident_cells": v["confident"],
                   "stable_cells": v["stable"], "total_cells": v["total"], "third_calls": v["third"], "min_confidence": v["min_conf"], "mean_confidence": v["mean_conf"],
                   "calls_below_0_8_reference": v["below_ref_0_8"], "complexity_chars": v["complexity_chars"], "reason": reason})
rk.sort(key=lambda r: (r["role"], not r["accepted"], -r["exact_cells"], r["calls_below_0_8_reference"], -r["min_confidence"], r["complexity_chars"]))
n = {}
for r in rk: n[r["role"]] = n.get(r["role"], 0) + 1; r["rank"] = n[r["role"]]
cells = {(x["case"], x["candidate"].split(":")[-1]): x for x in sc["r4conf"]["cells"]}
table = []
for case in o["semantic_cases"]:
    cid = case["case_id"]; row = {"case_id": cid, "origin": "a4-extension" if cid.startswith("a4-") else ("a3-extension" if cid.startswith("a3-") else "base-v2")}
    for role in ROLES:
        x = cells[(cid, role)]; row[role] = {"expected": x["expected"], "calls": [[a, b] for a, b in x["calls"]], "cell": x["cell"]}
    table.append(row)
C = sc["r4conf"]["candidates"].values()
sem = {"exact_cells": sum(v["exact"] for v in C), "confident_cells": sum(v["confident"] for v in C), "stable_cells": sum(v["stable"] for v in C), "total_cells": sum(v["total"] for v in C)}
runs = {r: json.load(open(f"{E4}/run-r{r}.json")) for r in ("r4a", "r4b", "r4conf")}
ids = sorted(json.load(open(p))["run_id"] for p in glob.glob(E4 + "/responses/*.json"))
data = {
 "operation": "testability", "gate": "testability", "accepted": False, "rubric_version": 2,
 "tournament_protocol_version": "plan-anchor/question-tournament/2.0.0", "production_confidence_threshold": 0.6, "confidence_reference_threshold": 0.8,
 "corpus_sha256": osha, "corpus_case_count": len(o["semantic_cases"]), "rubric_question_set": None,
 "candidate_rankings": rk, "acceptance_table": table,
 "confirmation_summary": {"two_call_cells": sem["total_cells"], "third_call_cells": sum(v["third"] for v in C),
     "inconclusive_cells": sum(x["cell"] == "inconclusive" for x in sc["r4conf"]["cells"]), "unstable_cells": sum(x["cell"] == "unstable" for x in sc["r4conf"]["cells"])},
 "deterministic_lint": {"requirement_form_all_match": True, "extension_lint": o["extension_lint"], "base_evidence": rel(E + "/deterministic-lint.json")},
 "dedupe": {"note": "dedupe question and pairs unchanged; prior byte-identical paired result reused", "rows": dd["rows"], "evidence": rel(E + "/dedupe-score.json")},
 "evidence": {"directory": rel(E4), "corpus": rel(E4 + "/oracle.json"), "corpus_sha256": osha, "previous_corpus_sha256": o["previous_corpus_sha256"],
   "base_26_cases_unchanged": True, "approval": o["approval_status"], "oracle_frozen_before_first_jev_call": True,
   "candidates": [rel(E4 + "/candidates-r4a.json"), rel(E4 + "/candidates-r4b.json")], "bundles": [rel(E4 + f"/bundles-r{r}.json") for r in ("r4a", "r4b", "r4conf")],
   "scores": [rel(E4 + f"/score-{r}.json") for r in ("r4a", "r4b", "r4conf")], "raw_requests_dir": rel(E4 + "/requests"), "raw_responses_dir": rel(E4 + "/responses"),
   "rounds": runs, "jev_model_answered": "jev-1.14.0",
   "current_seal_retained": "3f1c0f657da6564e3fed36058107f9760206582e06f67107e9dea2b7f1fb2122 (26-case set dd88add1)",
   "sealed_on_v4_baseline": {"self_contained": "a4-05 yes@0.50/0.53 (expected no)", "faithful": "a4-02 yes@0.66/0.58 and a0-14 yes@0.55/0.73 below 0.6",
                             "observable": "a4-05 yes@0.98/0.98 (expected no)", "single_constraint": "32/32", "scoped": "32/32"},
   "leading_candidates": {"self_contained": "rr4b:s19", "faithful": "rr4b:f8"},
   "blocking_cell": {"role": "observable", "case": "a4-05-buried-undefined-label", "expected": "no", "calls": [["yes", 0.98], ["yes", 0.98]], "seen_in": ["r4a", "r4conf"]},
   "planner_observation": "untrusted; Phase U run files were not read"},
 "explanation": ("Reopened self_contained, and faithful because the same shape fails it, on seal 3f1c0f65. The multi-record gap is real. Six general a4 cases were frozen before any call (0daf11ab, 32 cases); the 26 prior cases are byte-identical. "
   "Baseline on the sealed rubric: self_contained fails a4-05, a 'Q-tier bucket' label buried among three paths (yes at 0.50/0.53, expected no). Faithful fails a4-02, three commit hashes, at 0.66/0.58, and a0-14 at 0.55. "
   "10 candidates: self_contained s19 (32/32, min 0.70) and faithful f8 (32/32, min 0.70) lead, and both held in the five-question confirmation. "
   "Not sealed: the sealed observable answers a4-05 yes at 0.98/0.98 against the frozen 'no' label (the decisive destination is an undefined label, same shape as a3-06), and observable was not authorized for reopening. Confirmation matched 159 of 160 cells. Seal 3f1c0f65 stays current. "
   "Authorize reopening observable, with s19 and f8 carried as leads, to reach a seal."),
 "authority_recommendations": [{"kind": "reopen_scope", "target": "observable role on 32-case set " + osha,
   "request": "a4-05 is a confident oracle mismatch for sealed observable o8; authorize reopening observable with self_contained s19 and faithful f8 as leading candidates."}],
 "candidate_count": len(rk), "semantic_score": sem, "dedupe_case_count": 4,
 "dedupe_score": {"exact_cells": dd["score"]["exact"], "confident_cells": dd["score"]["confident"], "stable_cells": dd["score"]["stable"], "total_cells": 4},
 "repeat_requests_byte_identical": all(r["byte_identical_pairs"] for r in runs.values()),
}
git = subprocess.run(["git", "-C", W, "status", "--porcelain"], capture_output=True, text=True).stdout.splitlines()
meta = {"duration_ms": int(sys.argv[4]), "tool_calls": int(sys.argv[5]), "retry_count": 0, "correlation_id": req["correlation_id"] + ":self-contained-r4", "jev_run_ids": ids,
        "planner_observation_trust": "untrusted", "authoritative_sources_unchanged": all("anchor-runs/testability-tuning-20261104" in l for l in git)}
json.dump({"success": True, "canceled": False, "aborted_by": None, "data": data, "error": None, "metadata": meta}, open(E4 + "/result-vars.json", "w"), indent=1, ensure_ascii=False)
print(sem, len(rk), len(ids), meta["authoritative_sources_unchanged"])
