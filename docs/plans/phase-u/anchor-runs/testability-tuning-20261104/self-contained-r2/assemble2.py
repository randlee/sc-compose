import json, glob, os, subprocess, sys
W, E, E2 = sys.argv[1:4]
ROLES = ("single_constraint", "observable", "faithful", "scoped", "self_contained")
rel = lambda p: os.path.relpath(p, W)
o = json.load(open(E2 + "/oracle.json")); osha = open(E2 + "/oracle.sha256").read().strip()
s1 = json.load(open(E2 + "/score-sc1.json")); s2 = json.load(open(E2 + "/score-sc2.json")); r1 = json.load(open(E + "/score-1.json"))
dd = json.load(open(E + "/dedupe-score.json")); req = json.load(open(E + "/request.json"))
hist = {"s0_sealed": "r1:b0:self_contained", "s1_r1c1": "r1:b1:self_contained", "s2_r1c2": "r1:b2:self_contained", "s3_r1c3": "r1:b3:self_contained"}
rk = []
for cid, v in s1["candidates"].items():
    name = cid.split(":")[1]; prior = r1["candidates"].get(hist.get(name, ""), None)
    acc_all = v["accepted"] and (prior is None or prior["accepted"])
    rk.append({"role": "self_contained", "candidate_id": name, "accepted_v3_round": v["accepted"], "prior_v2_round1_accepted": None if prior is None else prior["accepted"],
               "accepted_across_all_evidence": acc_all, "exact_cells": v["exact"], "confident_cells": v["confident"], "stable_cells": v["stable"], "total_cells": v["total"],
               "third_calls": v["third"], "min_confidence": v["min_conf"], "mean_confidence": v["mean_conf"], "calls_below_0_8_reference": v["below_ref_0_8"], "complexity_chars": v["complexity_chars"],
               "reason": ("rejected: " + "; ".join(f"{m['case']} expected {m['expected']} got {m['calls']}" for m in v["mismatches"])) if not v["accepted"] else
                         ("accepted on v3 (26/26) and in the five-role confirmation; but r1 on v2 recorded a0-17-nondeclarative-form yes@0.52/0.67 below 0.6, so it is not accepted across all evidence" if not acc_all else "accepted")})
rk.sort(key=lambda r: (not r["accepted_across_all_evidence"], not r["accepted_v3_round"], -r["exact_cells"], r["calls_below_0_8_reference"], -r["min_confidence"], r["complexity_chars"]))
for i, r in enumerate(rk, 1): r["rank"] = i; r["selected"] = False
cells = {(x["case"], x["candidate"].split(":")[-1]): x for x in s2["cells"]}
table = []
for case in o["semantic_cases"]:
    row = {"case_id": case["case_id"], "origin": "extension-v3" if case["case_id"].startswith("a3-") else "base-v2"}
    for role in ROLES:
        x = cells[(case["case_id"], role)]; row[role] = {"expected": x["expected"], "calls": [[a, b] for a, b in x["calls"]], "cell": x["cell"]}
    table.append(row)
C = s2["candidates"].values()
sem = {"exact_cells": sum(v["exact"] for v in C), "confident_cells": sum(v["confident"] for v in C), "stable_cells": sum(v["stable"] for v in C), "total_cells": sum(v["total"] for v in C)}
inc = sum(x["cell"] == "inconclusive" for x in s2["cells"]); uns = sum(x["cell"] == "unstable" for x in s2["cells"]); nw = sum(x["cell"] == "needs_work" for x in s2["cells"])
runs = [json.load(open(E2 + f"/run-rsc{n}.json")) for n in (1, 2)]
ids = sorted(json.load(open(p))["run_id"] for p in glob.glob(E2 + "/responses/*.json"))
data = {
 "operation": "testability", "gate": "testability", "accepted": False, "rubric_version": 2,
 "tournament_protocol_version": "plan-anchor/question-tournament/2.0.0", "production_confidence_threshold": 0.6, "confidence_reference_threshold": 0.8,
 "corpus_sha256": osha, "corpus_case_count": len(o["semantic_cases"]), "rubric_question_set": None,
 "candidate_rankings": rk, "acceptance_table": table,
 "confirmation_summary": {"two_call_cells": sem["total_cells"], "third_call_cells": sum(v["third"] for v in C), "inconclusive_cells": inc, "unstable_cells": uns},
 "deterministic_lint": {"requirement_form_all_match": True, "extension_lint": o["extension_lint"], "base_evidence": rel(E + "/deterministic-lint.json")},
 "dedupe": {"note": "dedupe pairs and the dedupe question are unchanged by this reopen; prior byte-identical paired result reused", "rows": dd["rows"], "evidence": rel(E + "/dedupe-score.json")},
 "evidence": {"directory": rel(E2), "oracle_v3": rel(E2 + "/oracle.json"), "oracle_v3_sha256": osha, "oracle_frozen_before_first_jev_call": True,
   "base_corpus_sha256": o["base_corpus"]["sha256"], "base_17_cases_unchanged": True, "extension_case_count": 9, "extension_approval_status": o["approval_status"],
   "candidates": rel(E2 + "/candidates-sc1.json"), "bundles": [rel(E2 + "/bundles-rsc1.json"), rel(E2 + "/bundles-rsc2.json")],
   "scores": [rel(E2 + "/score-sc1.json"), rel(E2 + "/score-sc2.json")], "raw_requests_dir": rel(E2 + "/requests"), "raw_responses_dir": rel(E2 + "/responses"),
   "rounds": {"sc1_self_contained_tournament": runs[0], "sc2_five_role_confirmation": runs[1]}, "jev_model_answered": "jev-1.14.0",
   "current_seal_retained": "37f90cf634720a56027472ef1ebfbcc9d16e38405e7cdcaa96ae17ec67120b03 (v2 corpus); not revoked by this run",
   "sealed_self_contained_on_extension": "a3-06-undefined-mode-label no@0.60/0.49: fails the 0.6 floor; confirms the coverage gap is real",
   "blocking_cells_five_role_confirmation": [{"role": "observable", "case": "a3-06-undefined-mode-label", "expected": "no", "calls": [["yes", 0.14], ["yes", 0.3]]},
                                              {"role": "observable", "case": "a3-07-unidentified-expected-output", "expected": "no", "calls": [["unclear", 0.2], ["unclear", 0.2]]}],
   "planner_observation": "untrusted; r7-*.json and cands7.json were not read or used"},
 "explanation": ("Reopened self_contained only. The planner's gap claim was checked and holds: the v2 corpus has no inline-defined term, product command name, or unidentified reference artifact case. "
   "A 9-case plan-independent v3 extension was frozen before any call (base 17 unchanged; labels derived from the sealed criteria and the A0 cases each mirrors). On it the sealed self_contained fails (a3-06 at 0.60/0.49). "
   "Of 8 self_contained candidates, s1_r1c1 passed all 26 cases and the five-role confirmation, but its own round-1 v2 run had a0-17 at 0.52, so it is not accepted across all evidence. "
   "The decisive blocker is outside the reopened role: the sealed observable question fails extension cases a3-06 (yes at 0.14/0.30 for an undefined mode label) and a3-07 (unclear at 0.20). The frozen labels cannot be changed and observable was not authorized for reopening, so no five-role rubric can be sealed against v3. "
   "The existing seal 37f90cf6 remains the v2 seal. Next: get the v3 extension labels approved (see the a3-07 recommendation) and authorize reopening observable with self_contained; s1_r1c1 is the leading self_contained candidate."),
 "authority_recommendations": [
   {"kind": "calibration_extension_approval", "target": "testability calibration corpus v3 extension (" + osha + ")",
    "request": "Approve the 9 tuner-authored a3-* cases and their labels before they become normative; until approved the A0 v2 corpus remains the authority."},
   {"kind": "clarity", "target": "observable criteria (sealed 37f90cf6) vs a3-07-unidentified-expected-output",
    "ambiguity": "The observable 'unclear' criterion ('some evidence is named, but the observation needed to settle compliance remains ambiguous') can be read to cover 'the expected output' with no identified record, while the frozen label is 'no' by analogy to a0-12 (no observable record).",
    "proposed": "Confirm that a reference to an output, approval, or artifact that has no identified record is 'no' for observable, or relabel a3-07 observable to 'unclear' in an approved v3 revision.",
    "meaning_unchanged_because": "Either choice only states which existing criterion applies to an unrecorded reference; it adds no new testability requirement."},
   {"kind": "reopen_scope", "target": "observable role",
    "request": "a3-06 (undefined private mode label, same shape as approved a0-11 R7) is an oracle mismatch for the sealed observable question independent of a3-07; reopening observable is required before any v3 seal."}],
 "candidate_count": len(s1["candidates"]),
 "semantic_score": sem, "dedupe_case_count": 4,
 "dedupe_score": {"exact_cells": dd["score"]["exact"], "confident_cells": dd["score"]["confident"], "stable_cells": dd["score"]["stable"], "total_cells": 4},
 "repeat_requests_byte_identical": all(r["byte_identical_pairs"] for r in runs),
}
git = subprocess.run(["git", "-C", W, "status", "--porcelain"], capture_output=True, text=True).stdout.splitlines()
meta = {"duration_ms": int(sys.argv[4]), "tool_calls": int(sys.argv[5]), "retry_count": 0, "correlation_id": req["correlation_id"] + ":self-contained-r2", "jev_run_ids": ids,
        "planner_observation_trust": "untrusted", "authoritative_sources_unchanged": all("anchor-runs/testability-tuning-20261104" in l for l in git)}
json.dump({"success": True, "canceled": False, "aborted_by": None, "data": data, "error": None, "metadata": meta}, open(E2 + "/result-vars.json", "w"), indent=1, ensure_ascii=False)
print(sem, inc, uns, nw, len(ids), meta["authoritative_sources_unchanged"])
