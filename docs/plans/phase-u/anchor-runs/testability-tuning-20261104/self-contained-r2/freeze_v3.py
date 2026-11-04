import json,hashlib,sys
W,E,E2=sys.argv[1:4]
sys.path.insert(0,W+'/.claude/skills/plan-anchoring/scripts'); import jev_judge as J
c=lambda o:hashlib.sha256(json.dumps(o,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
base_path=W+'/.claude/skills/plan-anchoring/assets/fixtures/testability-calibration.v2.json'
base=json.load(open(base_path)); assert c(base)=='ce5f4efcfee74ecc767bb8c140bf749ef2e33c779f82397d5c42d96698a7b1bc'
o1=json.load(open(E+'/oracle.json')); assert c(o1)==open(E+'/oracle.sha256').read().strip()
Y={"single_constraint":"yes","observable":"yes","faithful":"yes","scoped":"yes","self_contained":"yes"}
N=dict(Y,observable="no",self_contained="no")
def case(i,title,quote,scope,stmt,exp,why):
    return {"case_id":f"a3-{i:02d}-{title}","anchor":{"quote":quote,"scope":scope,"statement":stmt},"expected":exp,"requirement_form_lint":"pass","rationale":why,"title":title}
ext=[
 case(1,"inline-defined-mode","offline mode just means --offline is passed; in that mode the installer should only read packages from ./vendor","installer offline mode",
  "In offline mode (when the --offline flag is set), the installer must read packages only from the ./vendor directory.",Y,
  "The mode term is defined inline by an explicit flag condition; self_contained yes criterion: explicit input/action/outcome conditions suffice."),
 case(2,"inline-defined-meaning","strict mode is strict: true in the config, and then any lint warning should fail the run with exit 1","linter strict mode",
  "In strict mode, meaning the config file sets strict: true, the linter must exit with status 1 when it reports any warning.",Y,
  "Mode defined in the statement by a concrete config value; nothing decisive is left undefined."),
 case(3,"product-command-name","pkg sync-lock has to rewrite package.lock with entries sorted by package name","pkg sync-lock command",
  "The pkg sync-lock command must rewrite package.lock so that its entries are sorted by package name.",Y,
  "A named product command and file are ordinary technical terms, not private shorthand (yes criterion)."),
 case(4,"caller-role-flag","if the caller passes --dry-run, deploy must not modify anything under /srv","deploy command dry-run",
  "When the caller passes --dry-run to the deploy command, the deploy command must not modify any file under /srv.",Y,
  "'The caller' is an ordinary role term and the condition/prohibition is concrete."),
 case(5,"identified-expected-output","for every file in tests/fixtures the formatter output has to match the .expected file next to it byte for byte","formatter golden tests",
  "For each input file in tests/fixtures/, the formatter output must be byte-identical to the file with the same name and an .expected suffix in that directory.",Y,
  "The expected output is an identified recorded artifact; nothing outside the statement is needed."),
 case(6,"undefined-mode-label","run the importer in Mode K","importer",
  "The importer must run in Mode K.",N,
  "Mode K is an undefined private label, the same shape as a0-11 R7: self_contained no (private label) and observable no (undefined shorthand)."),
 case(7,"unidentified-expected-output","make the formatter produce the expected output","formatter output",
  "The formatter must produce the expected output.",N,
  "Compliance depends on an expected output that is never identified or recorded; decisive term is an undefined reference (self_contained no) and no observable record settles it (observable no), mirroring a0-12."),
 case(8,"unrecorded-signoff","migrations only run after the database owner signs off","schema migrations",
  "Schema migrations may run only after the database owner signs off.",N,
  "Human approval with no identified record, same shape as a0-12: self_contained no, observable no."),
 case(9,"recorded-approval","migrations only run once their ID is listed in docs/approvals/migrations.md","schema migrations",
  "A schema migration may run only when its migration ID is listed in docs/approvals/migrations.md.",Y,
  "Approval is an identified recorded artifact, the contrast to a3-08: self_contained yes and observable yes."),
]
lint=[]
for x in ext:
    w=J.lint_statement(x['anchor']['statement'],x['anchor']['quote'],'ANC')
    errs=[e for e in w if e in J.REQUIREMENT_FORM_WARNINGS]
    lint.append({"id":x['case_id'],"expected":"pass","got":"fail" if errs else "pass","errors":errs,"verbatim_quote_error":J.LINT_VERBATIM in w,"warnings":w})
corpus={"schema":"plan-anchor/testability-calibration-corpus/1","extension_schema":"plan-anchor/testability-calibration-extension/3",
 "approval_status":"tuner-authored extension pending user approval; base 17 cases unchanged",
 "base_corpus":{"path":".claude/skills/plan-anchoring/assets/fixtures/testability-calibration.v2.json","sha256":c(base),"case_count":17},
 "plan_independent":True,"phase_u_text_excluded":True,
 "label_authority":"sealed rubric criteria text (self_contained/observable digest 37f90cf6...) and the A0 cases each extension case mirrors",
 "semantic_cases":o1['semantic_cases']+ext,"requirement_form_lint_cases":o1['requirement_form_lint_cases'],"dedupe_pairs":o1['dedupe_pairs'],
 "extension_lint":lint}
json.dump(corpus,open(E2+'/oracle.json','w'),indent=1,sort_keys=True,ensure_ascii=False)
open(E2+'/oracle.sha256','w').write(c(corpus)+'\n')
assert corpus['semantic_cases'][:17]==o1['semantic_cases']
print(c(corpus),len(corpus['semantic_cases']),[l for l in lint if l['got']!='pass' or l['verbatim_quote_error']])
