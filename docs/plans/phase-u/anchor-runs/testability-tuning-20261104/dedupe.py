#!/usr/bin/env python3
"""Dedupe pairs x2 (third on disagreement) with the shipped dedupe question; request shape identical to `jev_judge.py dedupe --jev`."""
import json, sys, types, hashlib
from concurrent.futures import ThreadPoolExecutor
W, E = sys.argv[1], sys.argv[2]
sys.path.insert(0, W + "/.claude/skills/plan-anchoring/scripts")
import jev_judge as J
o = json.load(open(E + "/oracle.json"))
qs = J.load_simple_question_set("dedupe", (J.DEDUPE_QUESTION,)).value["questions"]
api = J.remote_preflight(types.SimpleNamespace(repository_root=W, model=J.DEFAULT_MODEL), "The anchor statements").value
def call(rid, body):
    raw = J.encode_body(body); open(f"{E}/requests/{rid.replace(':','_')}.json", "wb").write(raw)
    r = J.judge_with_policy(json.loads(raw), api, 60.0, J.DEFAULT_MAX_REQUEST_BYTES, J.DEFAULT_MIN_ITEM_CHARS, qs)
    rec = {"run_id": rid, "request_sha256": hashlib.sha256(raw).hexdigest()}
    rec.update(response=r.value[0], attempts=r.value[1]["attempts"]) if isinstance(r, J.Ok) else rec.update(error=r.error.to_json()["error"])
    json.dump(rec, open(f"{E}/responses/{rid.replace(':','_')}.json", "w"), indent=1, sort_keys=True); return rec
tasks = [(f"phase-u:beads-anchors:testability:dedupe:{p['pair_id']}", {"model": J.DEFAULT_MODEL, "questions": qs, "state": {"a": {"id": None, "statement": p["a"]["statement"]}, "b": {"id": None, "statement": p["b"]["statement"]}}}, p) for p in o["dedupe_pairs"]]
res = {}
with ThreadPoolExecutor(4) as ex:
    fs = {ex.submit(call, f"{rid}:call{n}", b): (rid, n) for rid, b, _ in tasks for n in (1, 2)}
    for f, k in fs.items(): res[k] = f.result()
    th = [(rid, b) for rid, b, _ in tasks if res[(rid, 1)]["response"]["answers"][J.DEDUPE_QUESTION]["choice"] != res[(rid, 2)]["response"]["answers"][J.DEDUPE_QUESTION]["choice"]]
    fs = {ex.submit(call, f"{rid}:call3", b): (rid, 3) for rid, b in th}
    for f, k in fs.items(): res[k] = f.result()
rows = []
for rid, b, p in tasks:
    calls = [(res[(rid, n)]["response"]["answers"][J.DEDUPE_QUESTION]["choice"], res[(rid, n)]["response"]["answers"][J.DEDUPE_QUESTION]["confidence"]) for n in (1, 2, 3) if (rid, n) in res]
    st = calls[0][0] == calls[1][0]; ex_ = st and calls[0][0] == p["expected"]; cf = min(calls[0][1], calls[1][1]) >= 0.6
    rows.append({"pair_id": p["pair_id"], "expected": p["expected"], "calls": calls, "stable": st, "exact": ex_, "confident": cf,
                 "byte_identical": res[(rid, 1)]["request_sha256"] == res[(rid, 2)]["request_sha256"]})
out = {"question_set": "assets/questions/dedupe.json v2 (unchanged)", "rows": rows, "third_calls": len(th),
       "score": {k: sum(r[k] for r in rows) for k in ("exact", "confident", "stable")} | {"total": len(rows)}}
json.dump(out, open(E + "/dedupe-score.json", "w"), indent=1); print(json.dumps(out["score"]), [ (r["pair_id"], r["calls"]) for r in rows])
