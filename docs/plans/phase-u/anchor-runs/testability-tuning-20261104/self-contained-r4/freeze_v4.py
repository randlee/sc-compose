import json,hashlib,sys
E3,E4=sys.argv[1:3]
sys.path.insert(0,sys.argv[3]+'/.claude/skills/plan-anchoring/scripts'); import jev_judge as J
c=lambda o:hashlib.sha256(json.dumps(o,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
o3=json.load(open(E3+'/oracle.json')); assert c(o3)=='dd88add1126dcde62d26d1fd2052987dcdeb035a6325fecb1d42acb5886c63d2'
Y={"single_constraint":"yes","observable":"yes","faithful":"yes","scoped":"yes","self_contained":"yes"}; N=dict(Y,observable="no",self_contained="no")
def case(i,t,q,s,st,e,r): return {"case_id":f"a4-{i:02d}-{t}","title":t,"anchor":{"quote":q,"scope":s,"statement":st},"expected":e,"requirement_form_lint":"pass","rationale":r}
ext=[
 case(1,"multi-diagnostic-codes","out/lint.json has to list W001-unused-import, W014-shadowed-name and E203-missing-return by their codes","lint report output",
  "The lint report written to out/lint.json must list each of the diagnostics W001-unused-import, W014-shadowed-name, and E203-missing-return by its code.",Y,
  "Several named diagnostic codes and one path are concrete records of one listing duty; nothing decisive is undefined."),
 case(2,"multi-commit-hashes","the 1.4.0 release notes need to cite commits 3f2a9c1, 8b7d0e4 and c41f5a2 by short hash","release notes",
  "The release notes in docs/release-notes/1.4.0.md must cite commits 3f2a9c1, 8b7d0e4, and c41f5a2 by their short hashes.",Y,
  "Commit hashes and a path are identified records; one citation duty."),
 case(3,"report-id-record","only close the parser fuzz campaign once report FZ-2026-031 in docs/reports shows zero open crashes for fuzz/corpus/parser","fuzz campaign closure",
  "The parser fuzz campaign may be closed only when report FZ-2026-031 in docs/reports/ records zero open crashes for the corpus fuzz/corpus/parser/.",Y,
  "The report id is the identified record of the corpus outcome (contrast a3-08 unrecorded sign-off)."),
 case(4,"exact-file-list","dist/tool-1.4.0.tar.gz should contain exactly bin/tool, lib/libtool.so, share/man/man1/tool.1 and LICENSE","release archive contents",
  "The archive dist/tool-1.4.0.tar.gz must contain exactly the files bin/tool, lib/libtool.so, share/man/man1/tool.1, and LICENSE.",Y,
  "Four concrete paths enumerate one exact-contents duty."),
 case(5,"buried-undefined-label","the release job copies build/out/manifest.json, dist/checksums.txt and docs/CHANGELOG.md into the Q-tier bucket","release artifact upload",
  "The release job must copy build/out/manifest.json, dist/checksums.txt, and docs/CHANGELOG.md into the Q-tier bucket.",N,
  "Q-tier bucket is an undefined private label and is the decisive destination; same shape as a3-06 Mode K: self_contained no, observable no."),
 case(6,"buried-subjective-quality","scripts/migrate_v4.py should turn db/schema_v3.sql, db/seed_v3.sql and db/views_v3.sql into clean v4 equivalents","schema migration script",
  "The script scripts/migrate_v4.py must convert db/schema_v3.sql, db/seed_v3.sql, and db/views_v3.sql into clean v4 equivalents.",N,
  "Clean is a decisive subjective quality, same shape as a0-08 robust: self_contained no, observable no."),
]
lint=[]
for x in ext:
    w=J.lint_statement(x['anchor']['statement'],x['anchor']['quote'],'ANC'); errs=[e for e in w if e in J.REQUIREMENT_FORM_WARNINGS]
    lint.append({"id":x['case_id'],"expected":"pass","got":"fail" if errs else "pass","errors":errs,"verbatim_quote_error":J.LINT_VERBATIM in w})
o=dict(o3); o['semantic_cases']=o3['semantic_cases']+ext; o['extension_lint']=o3['extension_lint']+lint
o['extension_schema']="plan-anchor/testability-calibration-extension/4"
o['approval_status']="a3 approved 2026-11-04; a4 derived from sealed criteria under the user's standing approval relayed by the coordinator ('I don't need to \"approve\" or \"lock\" every time')"
o['previous_corpus_sha256']=c(o3)
assert o['semantic_cases'][:26]==o3['semantic_cases']
json.dump(o,open(E4+'/oracle.json','w'),indent=1,sort_keys=True,ensure_ascii=False); open(E4+'/oracle.sha256','w').write(c(o)+'\n')
print(c(o),len(o['semantic_cases']),[l for l in lint if l['got']!='pass' or l['verbatim_quote_error']])
