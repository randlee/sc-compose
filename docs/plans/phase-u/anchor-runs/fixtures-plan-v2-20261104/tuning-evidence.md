# Lead plan-question tuning (Phase U)

Problem: default plan question `addressed` scored 0.18-0.55 confidence on every fail fixture (runs cf1, cf2). Its "no" criterion means "silent", but a fail fixture is not silent: it does the opposite. Non-orthogonal with `contradicted`.

Fix (plan.v2.json): `addressed` asks whether the item carries out the directive. "no" = silent OR does something different/opposite. relevant/contradicted unchanged; roles unchanged; threshold 0.6 unchanged.

Results, 10 locked anchors x section D draft fixtures (pass+fail), plan gate:
- v1 (cf1, cf2): 20/20 expected verdicts, min fail.addressed 0.18
- v2 (cf3, cf4): 20/20 expected verdicts both runs; min cell 0.88 (fail.contradicted); fail.addressed 1.0; pass.addressed >= 0.94
- Silence probe (cf5, cf6 v2 vs cf7 v1): identical verdicts and similar confidences on 3 relevant-but-silent probes. v2 does not change silence handling; the low cells there come from ambiguous probe text.

Mixed-item probes (comps' open review condition), 2 runs each: ANC-002 (padded rendered, tight left to bd), ANC-015 (docli in release workflow, bypassed by manual PyPI publisher), ANC-033 (Windows excluded, manylinux added to normal CI). Expected: contradicted.
- v1 (mxv11/mxv12): verdict contradicted 6/6; addressed=yes at 0.24-0.96 (ambiguous).
- v2 (mxv2b1/mxv2b2): verdict contradicted 6/6; addressed=no at 0.88-1.0; contradicted >= 0.89.
Mixed items are treated as not carried out (addressed=no) and contradicted, confidently.
