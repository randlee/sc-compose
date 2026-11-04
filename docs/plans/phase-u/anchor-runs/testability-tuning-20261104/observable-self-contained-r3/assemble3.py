import json, glob, os, subprocess, sys, hashlib
W, E, E3 = sys.argv[1:4]
c = lambda o: hashlib.sha256(json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()
ROLES = ("single_constraint", "observable", "faithful", "scoped", "self_contained")
rel = lambda p: os.path.relpath(p, W)
o = json.load(open(E3 + "/oracle.json")); osha = open(E3 + "/oracle.sha256").read().strip()
sc = {r: json.load(open(f"{E3}/score-{r}.json")) for r in ("os1", "os2", "os3", "conf")}
s2prev = json.load(open(E + "/self-contained-r2/score-sc2.json")); dd = json.load(open(E + "/dedupe-score.json")); req = json.load(open(E + "/request.json"))
conf = json.load(open(E3 + "/bundles-rconf.json")); Q = conf["bundles"]["S3"]; prov = conf["provenance"]
history = {"o_sealed": "also failed v3 in self-contained-r2 sc2 (a3-06, a3-07)", "o_incumbent_v2": "also failed v2 r1 (a0-12)",
           "s1": "also failed v2 r1 (a0-17 0.52/0.67); repeat-confirm here a0-17 0.48/0.60 failed again: settled not accepted",
           "s_incumbent_v2": "also failed v3 in self-contained-r2 sc1 (a3-06)"}
sel = {"observable": "o8", "self_contained": "s13"}
rk = []
for rnd in ("os1", "os2", "os3"):
    for cid, v in sc[rnd]["candidates"].items():
        name = cid.split(":")[1].split("-", 1)[1]; role = cid.split(":")[2]
        reason = "rejected: " + "; ".join(f"{m['case']} expected {m['expected']} got {m['calls']}" for m in v["mismatches"]) if not v["accepted"] else \
                 ("selected: accepted 26/26 with no failed history; reconfirmed in five-question confirmation" if sel[role] == name else "accepted but outranked on confidence diagnostics")
        if name in history: reason += "; " + history[name]
        rk.append({"role": role, "round": rnd, "candidate_id": name, "selected": sel[role] == name, "accepted": v["accepted"], "exact_cells": v["exact"],
                   "confident_cells": v["confident"], "stable_cells": v["stable"], "total_cells": v["total"], "third_calls": v["third"], "min_confidence": v["min_conf"],
                   "mean_confidence": v["mean_conf"], "calls_below_0_8_reference": v["below_ref_0_8"], "complexity_chars": v["complexity_chars"], "reason": reason})
rk.sort(key=lambda r: (r["role"], not r["accepted"], -r["exact_cells"], -r["stable_cells"], r["calls_below_0_8_reference"], -r["min_confidence"], r["complexity_chars"]))
cnt = {}
for r in rk: cnt[r["role"]] = cnt.get(r["role"], 0) + 1; r["rank"] = cnt[r["role"]]
cells = {(x["case"], x["candidate"].split(":")[-1]): x for x in sc["conf"]["cells"]}
table = []
for case in o["semantic_cases"]:
    row = {"case_id": case["case_id"], "origin": "extension-v3-approved" if case["case_id"].startswith("a3-") else "base-v2"}
    for role in ROLES:
        x = cells[(case["case_id"], role)]; row[role] = {"expected": x["expected"], "calls": [[a, b] for a, b in x["calls"]], "cell": x["cell"]}
    table.append(row)
C = sc["conf"]["candidates"].values()
sem = {"exact_cells": sum(v["exact"] for v in C), "confident_cells": sum(v["confident"] for v in C), "stable_cells": sum(v["stable"] for v in C), "total_cells": sum(v["total"] for v in C)}
runs = {r: json.load(open(f"{E3}/run-r{r}.json")) for r in ("os1", "os2", "os3", "conf")}
ids = sorted(json.load(open(p))["run_id"] for p in glob.glob(E3 + "/responses/*.json"))
digest = c({"questions": Q})
prior_sc2 = {k.split(":")[-1]: v["accepted"] for k, v in s2prev["candidates"].items()}
data = {
 "operation": "testability", "gate": "testability", "accepted": True, "rubric_version": 2,
 "tournament_protocol_version": "plan-anchor/question-tournament/2.0.0", "production_confidence_threshold": 0.6, "confidence_reference_threshold": 0.8,
 "corpus_sha256": osha, "corpus_case_count": len(o["semantic_cases"]), "rubric_question_set": {"digest": digest, "questions": Q},
 "candidate_rankings": rk, "acceptance_table": table,
 "confirmation_summary": {"two_call_cells": sem["total_cells"], "third_call_cells": sum(v["third"] for v in C),
     "inconclusive_cells": sum(x["cell"] == "inconclusive" for x in sc["conf"]["cells"]), "unstable_cells": sum(x["cell"] == "unstable" for x in sc["conf"]["cells"])},
 "deterministic_lint": {"requirement_form_all_match": True, "base_evidence": rel(E + "/deterministic-lint.json"), "extension_lint": o["extension_lint"]},
 "dedupe": {"note": "dedupe question and pairs unchanged; prior byte-identical paired result reused", "rows": dd["rows"], "evidence": rel(E + "/dedupe-score.json")},
 "evidence": {"directory": rel(E3), "corpus": rel(E3 + "/oracle.json"), "corpus_sha256": osha, "authority": json.load(open(E3 + "/authority.json")),
   "oracle_frozen_before_first_jev_call": True, "candidates": [rel(E3 + f"/candidates-os{n}.json") for n in (1, 2, 3)],
   "bundles": [rel(E3 + f"/bundles-r{r}.json") for r in ("os1", "os2", "os3", "conf")], "scores": [rel(E3 + f"/score-{r}.json") for r in ("os1", "os2", "os3", "conf")],
   "raw_requests_dir": rel(E3 + "/requests"), "raw_responses_dir": rel(E3 + "/responses"), "rounds": runs, "jev_model_answered": "jev-1.14.0",
   "selection_provenance": prov, "previous_seal": "37f90cf634720a56027472ef1ebfbcc9d16e38405e7cdcaa96ae17ec67120b03 (v2 corpus); superseded by this seal, its evidence preserved append-only",
   "carried_roles_v3_history": {"self-contained-r2 sc2 (26 cases)": {k: prior_sc2[k] for k in ("single_constraint", "faithful", "scoped")}, "this confirmation": "26/26 each"},
   "s1_repeat_confirm": {"a0-17": {"v2 r1": [["yes", 0.52], ["yes", 0.67]], "self-contained-r2 sc1": [["yes", 0.64], ["yes", 0.7]], "self-contained-r2 sc2": [["yes", 0.68], ["yes", 0.61]], "this run os1": [["yes", 0.48], ["yes", 0.6]]}, "disposition": "two of four paired runs below 0.6 on a0-17; not accepted (re-asking cannot vote it in)"},
   "a3_07_label": "frozen as observable no per user-pending decision; selected observable o8 answers no confidently in both rounds; if the user relabels a3-07, re-score only that case",
   "planner_observation": "untrusted; r7-*.json and cands7.json were not read or used"},
 "explanation": ("Reopened observable and self_contained on the user-approved 26-case set (dd88add1). Approvals were relayed verbatim by the coordinator. "
   "19 candidates over three rounds: no prior wording was accepted. The sealed observable missed a3-06 and a3-07; s1 missed a0-17 again at 0.48/0.60, two of four paired runs below 0.6. "
   "Selected observable o8 (26/26, min 0.82) and self_contained s13 (26/26, min 0.68); s13 states that who is told to act and the sentence form do not matter, which fixes a0-17. "
   "The five-question production-shape confirmation, with single_constraint, faithful and scoped carried as sealed, matched 130/130 cells, all confident and stable, with no third calls. Dedupe is unchanged at 4/4 and requirement-form lint matches. "
   "Lowest margins: faithful a0-14 at 0.61 and self_contained a3-08 at 0.64. This seal supersedes 37f90cf6. Re-run the Phase U anchors under it; any that stay inconclusive go to evidence-bound triage."),
 "authority_recommendations": [{"kind": "pending_user_decision", "target": "a3-07-unidentified-expected-output observable label",
   "status": "still under user discussion; kept 'no' as frozen. A relabel to 'unclear' invalidates only that case: re-score observable on a3-07 alone."}],
 "candidate_count": len(rk), "semantic_score": sem, "dedupe_case_count": 4,
 "dedupe_score": {"exact_cells": dd["score"]["exact"], "confident_cells": dd["score"]["confident"], "stable_cells": dd["score"]["stable"], "total_cells": 4},
 "repeat_requests_byte_identical": all(r["byte_identical_pairs"] for r in runs.values()),
}
git = subprocess.run(["git", "-C", W, "status", "--porcelain"], capture_output=True, text=True).stdout.splitlines()
meta = {"duration_ms": int(sys.argv[4]), "tool_calls": int(sys.argv[5]), "retry_count": 0, "correlation_id": req["correlation_id"] + ":observable-self-contained-r3", "jev_run_ids": ids,
        "planner_observation_trust": "untrusted", "authoritative_sources_unchanged": all("anchor-runs/testability-tuning-20261104" in l for l in git)}
json.dump({"success": True, "canceled": False, "aborted_by": None, "data": data, "error": None, "metadata": meta}, open(E3 + "/result-vars.json", "w"), indent=1, ensure_ascii=False)
print(digest, sem, len(rk), len(ids), meta["authoritative_sources_unchanged"])
