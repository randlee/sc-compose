import json, glob, os, subprocess, sys, hashlib
W, E, E5 = sys.argv[1:4]
c = lambda o: hashlib.sha256(json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()
ROLES = ("single_constraint", "observable", "faithful", "scoped", "self_contained")
rel = lambda p: os.path.relpath(p, W)
o = json.load(open(E5 + "/oracle.json")); osha = open(E5 + "/oracle.sha256").read().strip()
R = ("r5a", "r5b", "r5c", "r5conf")
sc = {r: json.load(open(f"{E5}/score-{r}.json")) for r in R}
r4 = {r: json.load(open(f"{E}/self-contained-r4/score-{r}.json")) for r in ("r4a", "r4b", "r4conf")}
dd = json.load(open(E + "/dedupe-score.json")); req = json.load(open(E + "/request.json"))
conf = json.load(open(E5 + "/bundles-rr5conf.json")); Q = conf["bundles"]["S5"]; prov = conf["provenance"]
rk = []
for rnd in ("r5a", "r5b", "r5c"):
    for cid, v in sc[rnd]["candidates"].items():
        name = cid.split(":")[1].split("-", 1)[1]
        reason = ("rejected: " + "; ".join(f"{m['case']} expected {m['expected']} got {m['calls']}" for m in v["mismatches"])) if not v["accepted"] else \
                 ("selected: accepted 32/32 and in the five-question confirmation" if name == "o19" else "accepted but outranked")
        rk.append({"role": "observable", "round": rnd, "candidate_id": name, "selected": name == "o19", "accepted": v["accepted"], "exact_cells": v["exact"],
                   "confident_cells": v["confident"], "stable_cells": v["stable"], "total_cells": v["total"], "third_calls": v["third"], "min_confidence": v["min_conf"],
                   "mean_confidence": v["mean_conf"], "calls_below_0_8_reference": v["below_ref_0_8"], "complexity_chars": v["complexity_chars"], "reason": reason})
rk.append({"role": "observable", "round": "self-contained-r4 r4a/r4conf", "candidate_id": "o8_sealed_3f1c0f65", "selected": False, "accepted": False, "exact_cells": 31, "confident_cells": 32,
           "stable_cells": 32, "total_cells": 32, "reason": "rejected: a4-05-buried-undefined-label expected no got yes@0.98/0.98 (twice)"})
rk.sort(key=lambda r: (not r["accepted"], -r["exact_cells"], r.get("calls_below_0_8_reference", 99), -r.get("min_confidence", 0), r.get("complexity_chars", 0)))
for i, r in enumerate(rk, 1): r["rank"] = i
for role, name in (("self_contained", "s19"), ("faithful", "f8")):
    rk.append({"role": role, "candidate_id": name, "selected": True, "accepted": True, "carried_from": "self-contained-r4 (rr4b, confirmed rr4conf)",
               "reason": "carried forward per authorization; passed 32/32 in r4b, r4conf, and r5conf"})
cells = {(x["case"], x["candidate"].split(":")[-1]): x for x in sc["r5conf"]["cells"]}
table = []
for case in o["semantic_cases"]:
    cid = case["case_id"]; row = {"case_id": cid, "origin": "a4-extension" if cid.startswith("a4-") else ("a3-extension" if cid.startswith("a3-") else "base-v2")}
    for role in ROLES:
        x = cells[(cid, role)]; row[role] = {"expected": x["expected"], "calls": [[a, b] for a, b in x["calls"]], "cell": x["cell"]}
    table.append(row)
C = sc["r5conf"]["candidates"].values()
sem = {"exact_cells": sum(v["exact"] for v in C), "confident_cells": sum(v["confident"] for v in C), "stable_cells": sum(v["stable"] for v in C), "total_cells": sum(v["total"] for v in C)}
runs = {r: json.load(open(f"{E5}/run-r{r}.json")) for r in R}
ids = sorted(json.load(open(p))["run_id"] for p in glob.glob(E5 + "/responses/*.json"))
digest = c({"questions": Q})
hist = {"single_constraint": [r4["r4a"]["candidates"]["rr4a:sealed:single_constraint"]["accepted"], r4["r4conf"]["candidates"]["rr4conf:S4:single_constraint"]["accepted"]],
        "scoped": [r4["r4a"]["candidates"]["rr4a:sealed:scoped"]["accepted"], r4["r4conf"]["candidates"]["rr4conf:S4:scoped"]["accepted"]],
        "self_contained_s19": [r4["r4b"]["candidates"]["rr4b:s-s19:self_contained"]["accepted"], r4["r4conf"]["candidates"]["rr4conf:S4:self_contained"]["accepted"]],
        "faithful_f8": [r4["r4b"]["candidates"]["rr4b:f-f8:faithful"]["accepted"], r4["r4conf"]["candidates"]["rr4conf:S4:faithful"]["accepted"]]}
assert all(all(v) for v in hist.values())
data = {
 "operation": "testability", "gate": "testability", "accepted": True, "rubric_version": 2,
 "tournament_protocol_version": "plan-anchor/question-tournament/2.0.0", "production_confidence_threshold": 0.6, "confidence_reference_threshold": 0.8,
 "corpus_sha256": osha, "corpus_case_count": len(o["semantic_cases"]), "rubric_question_set": {"digest": digest, "questions": Q},
 "candidate_rankings": rk, "acceptance_table": table,
 "confirmation_summary": {"two_call_cells": sem["total_cells"], "third_call_cells": sum(v["third"] for v in C),
     "inconclusive_cells": sum(x["cell"] == "inconclusive" for x in sc["r5conf"]["cells"]), "unstable_cells": sum(x["cell"] == "unstable" for x in sc["r5conf"]["cells"])},
 "deterministic_lint": {"requirement_form_all_match": True, "extension_lint": o["extension_lint"], "base_evidence": rel(E + "/deterministic-lint.json")},
 "dedupe": {"note": "dedupe question and pairs unchanged; prior byte-identical paired result reused", "rows": dd["rows"], "evidence": rel(E + "/dedupe-score.json")},
 "evidence": {"directory": rel(E5), "corpus": rel(E5 + "/oracle.json"), "corpus_sha256": osha, "oracle_frozen_before_first_jev_call": True,
   "authorization": "observable reopen authorized by user standing direction relayed by the coordinator ('I don't need to \"approve\" or \"lock\" every time'; 'If they aren't [passing] DO NOT STOP')",
   "candidates": [rel(E5 + f"/candidates-r5{x}.json") for x in "abc"], "bundles": [rel(E5 + f"/bundles-r{r}.json") for r in R],
   "scores": [rel(E5 + f"/score-{r}.json") for r in R], "raw_requests_dir": rel(E5 + "/requests"), "raw_responses_dir": rel(E5 + "/responses"),
   "rounds": runs, "jev_model_answered": "jev-1.14.0", "selection_provenance": prov, "prior_runs_on_this_corpus_all_accepted": hist,
   "previous_seal": "3f1c0f657da6564e3fed36058107f9760206582e06f67107e9dea2b7f1fb2122 (26-case dd88add1); superseded, evidence preserved",
   "lowest_margin_cells": [{"role": "faithful", "case": "a4-02-multi-commit-hashes", "calls": [0.66, 0.70]}, {"role": "faithful", "case": "a4-03-report-id-record", "calls": [0.66, 0.72]},
                           {"role": "self_contained", "case": "a0-06-genuinely-bundled", "calls": [0.67, 0.74]}, {"role": "faithful", "case": "a0-14-shared-quote-second-atomic", "calls": [0.68, 0.73]}],
   "planner_observation": "untrusted; Phase U run files were not read"},
 "explanation": ("Reopened observable on the frozen 32-case set 0daf11ab, with self_contained s19 and faithful f8 carried forward and single_constraint and scoped as sealed. "
   "The sealed observable o8 answered a4-05 (an undefined tier label as the copy destination) yes at 0.98 twice. 10 observable candidates over three rounds: the general 'every part must be defined' wordings traded a4-05 against the bundled case a0-06. "
   "o19 keeps o8's structure and adds that an undefined label, tier, or mode that names where results must go leaves nothing to inspect. It was accepted 32/32 (min 0.72). "
   "The five-question production-shape confirmation matched 160/160 cells, all confident and stable, with no third calls. Every selected wording has a clean record on this set. Dedupe is unchanged at 4/4 and requirement-form lint matches. "
   "Supersedes seal 3f1c0f65. Lowest margins: faithful a4-02 and a4-03 at 0.66."),
 "authority_recommendations": [], "candidate_count": len(rk), "semantic_score": sem, "dedupe_case_count": 4,
 "dedupe_score": {"exact_cells": dd["score"]["exact"], "confident_cells": dd["score"]["confident"], "stable_cells": dd["score"]["stable"], "total_cells": 4},
 "repeat_requests_byte_identical": all(r["byte_identical_pairs"] for r in runs.values()),
}
git = subprocess.run(["git", "-C", W, "status", "--porcelain"], capture_output=True, text=True).stdout.splitlines()
meta = {"duration_ms": int(sys.argv[4]), "tool_calls": int(sys.argv[5]), "retry_count": 0, "correlation_id": req["correlation_id"] + ":observable-r5", "jev_run_ids": ids,
        "planner_observation_trust": "untrusted", "authoritative_sources_unchanged": all("anchor-runs/testability-tuning-20261104" in l for l in git)}
json.dump({"success": True, "canceled": False, "aborted_by": None, "data": data, "error": None, "metadata": meta}, open(E5 + "/result-vars.json", "w"), indent=1, ensure_ascii=False)
print(digest, sem, len(rk), len(ids), meta["authoritative_sources_unchanged"])
