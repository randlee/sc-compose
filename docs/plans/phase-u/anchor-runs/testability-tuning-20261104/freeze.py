import json,hashlib,sys,datetime
W=sys.argv[1]; E=sys.argv[2]
sys.path.insert(0,W+'/.claude/skills/plan-anchoring/scripts')
import jev_judge as J
c=lambda o:hashlib.sha256(json.dumps(o,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
corp=json.load(open(W+'/.claude/skills/plan-anchoring/assets/fixtures/testability-calibration.v2.json'))
assert c(corp)=='ce5f4efcfee74ecc767bb8c140bf749ef2e33c779f82397d5c42d96698a7b1bc'
assert len(corp['semantic_cases'])==17
cases=[]
for s in corp['semantic_cases']:
    exp={('single_constraint' if k=='atomicity' else k):v for k,v in s['expected'].items()}
    cases.append({"case_id":s['case_id'],"anchor":s['state']['anchor'],"expected":exp,"requirement_form_lint":s['requirement_form_lint'],"rationale":s['rationale']})
oracle={"schema":"plan-anchor/testability-oracle/1","corpus_sha256":c(corp),"corpus_schema":corp['schema'],
 "authority":"frozen plan-independent calibration corpus (descriptor plan-anchor/testability-calibration/2 v2); legacy role atomicity mapped to single_constraint",
 "semantic_cases":cases,"requirement_form_lint_cases":corp['requirement_form_lint_cases'],"dedupe_pairs":corp['dedupe_pairs'],
 "acceptance":"every cell: two byte-identical calls agree with the expected choice at confidence >= 0.6; third call only on first-pair choice disagreement and never votes a cell into acceptance"}
json.dump(oracle,open(E+'/oracle.json','w'),indent=1,sort_keys=True,ensure_ascii=False)
open(E+'/oracle.sha256','w').write(c(oracle)+'\n')
# deterministic lint
lint={"semantic_cases":[],"lint_cases":[]}
for s in cases:
    w=J.lint_statement(s['anchor']['statement'],s['anchor']['quote'],'ANC')
    errs=[x for x in w if x in J.REQUIREMENT_FORM_WARNINGS]
    got='fail' if errs else 'pass'
    lint['semantic_cases'].append({"id":s['case_id'],"expected":s['requirement_form_lint'],"got":got,"requirement_form_errors":errs,"verbatim_quote_error":J.LINT_VERBATIM in w,"warnings":w,"match":got==s['requirement_form_lint']})
for s in corp['requirement_form_lint_cases']:
    w=J.lint_statement(s['statement'],'(quote differs)','ANC')
    errs=[x for x in w if x in J.REQUIREMENT_FORM_WARNINGS]
    got='fail' if errs else 'pass'
    lint['lint_cases'].append({"id":s['id'],"expected":s['expected'],"got":got,"errors":errs,"match":got==s['expected']})
lint['all_match']=all(x['match'] for x in lint['semantic_cases']+lint['lint_cases'])
json.dump(lint,open(E+'/deterministic-lint.json','w'),indent=1)
print(c(oracle), lint['all_match'], [x['id'] for x in lint['semantic_cases']+lint['lint_cases'] if not x['match']])
