#!/usr/bin/env python3
"""Testability rubric tournament harness (anchor-tuning, operation testability).

Uses only the workspace jev_judge.py (remote_preflight, judge_with_policy,
encode_body, answer_choice). Request shape is identical to `jev_judge.py
rubric`: {model, questions, state: {anchor: {statement, quote, scope}}}.
Usage: harness.py <workspace> <evidence_dir> <round> <bundles.json>
bundles.json: {"bundles": {bid: {role: question}}}
"""
import hashlib
import json
import sys
import types
from concurrent.futures import ThreadPoolExecutor

W, E, ROUND, BUNDLES = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
sys.path.insert(0, W + "/.claude/skills/plan-anchoring/scripts")
import jev_judge as J  # noqa: E402

PREFIX = "phase-u:beads-anchors:testability"
THRESH = 0.6


def sha(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


oracle = json.load(open(E + "/oracle.json"))
assert sha(json.dumps(oracle, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()) == open(E + "/oracle.sha256").read().strip()
bundles = json.load(open(BUNDLES))["bundles"]
args = types.SimpleNamespace(repository_root=W, model=J.DEFAULT_MODEL, timeout=60.0)
key = J.remote_preflight(args, "The anchor statements, quotes and scopes")
if isinstance(key, J.Err):
    print(json.dumps(key.error.to_json()))
    sys.exit(2)
API = key.value

tasks = []  # (run_id, body)
for bid, qs in sorted(bundles.items()):
    for case in oracle["semantic_cases"]:
        a = case["anchor"]
        body = {"model": J.DEFAULT_MODEL, "questions": qs,
                "state": {"anchor": {"statement": a["statement"], "quote": a["quote"], "scope": a["scope"]}}}
        tasks.append((f"{PREFIX}:r{ROUND}:{bid}:{case['case_id']}", body))
if ROUND.endswith("dedupe") or "--dedupe" in sys.argv:
    pass


def call(run_id, body):
    raw = J.encode_body(body)
    open(f"{E}/requests/{run_id.replace(':', '_')}.json", "wb").write(raw)
    res = J.judge_with_policy(json.loads(raw), API, 60.0, J.DEFAULT_MAX_REQUEST_BYTES, J.DEFAULT_MIN_ITEM_CHARS, body["questions"])
    rec = {"run_id": run_id, "request_sha256": sha(raw)}
    if isinstance(res, J.Ok):
        resp, meta = res.value
        rec.update(response=resp, attempts=meta.get("attempts"))
    else:
        rec.update(error=res.error.to_json()["error"], attempts=(getattr(res.error, "meta", None) or {}).get("attempts"))
    json.dump(rec, open(f"{E}/responses/{run_id.replace(':', '_')}.json", "w"), indent=1, sort_keys=True)
    return rec


results = {}
with ThreadPoolExecutor(max_workers=4) as ex:
    futs = {}
    for rid, body in tasks:
        for n in (1, 2):
            futs[ex.submit(call, f"{rid}:call{n}", body)] = (rid, n)
    for f, (rid, n) in futs.items():
        results[(rid, n)] = f.result()
    third = []
    for rid, body in tasks:
        r1, r2 = results[(rid, 1)], results[(rid, 2)]
        if "response" not in r1 or "response" not in r2:
            continue
        if any(J.answer_choice(r1["response"]["answers"][q])[0] != J.answer_choice(r2["response"]["answers"][q])[0] for q in body["questions"]):
            third.append((rid, body))
    futs = {ex.submit(call, f"{rid}:call3", body): rid for rid, body in third}
    for f, rid in futs.items():
        results[(rid, 3)] = f.result()

failed = [r["run_id"] for r in results.values() if "error" in r]
summary = {"round": ROUND, "requests": len(results), "third_calls": len(third), "failed": failed,
           "byte_identical_pairs": all(results[(rid, 1)]["request_sha256"] == results[(rid, 2)]["request_sha256"] for rid, _ in tasks)}
json.dump(summary, open(f"{E}/run-r{ROUND}.json", "w"), indent=1)
print(json.dumps(summary))
