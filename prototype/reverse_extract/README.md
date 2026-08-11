# Reverse Template Variable Extractor

Extract the JSON variable bindings that produced a rendered Jinja2 template —
the reverse of `sc-compose render`.

**Given:** a `.j2` template + its rendered XML output  
**Returns:** `{var_name: value, ...}` with a confidence score (0.0–1.0)

## Quick Start

```python
from prototype.reverse_extract import extract_variables

result = extract_variables(
    "qa-template.xml.j2",
    "rendered-payload.xml",
    include_vars=["task_id", "sprint", "branch"],
)
# → {"task_id": "AI2-QA-2", "sprint": "AI.2", "branch": "feature/pAI-s2-storage-topology",
#    "_confidence": 0.885}
```

### Filtering

```python
# Only extract specific vars
extract_variables(tmpl, out, include_vars=["task_id", "branch"])

# Skip noisy fields
extract_variables(tmpl, out, exclude_vars=["description", "references"])

# Disable metadata
extract_variables(tmpl, out, include_metadata=False)
```

## Confidence Score

`_confidence` measures what fraction of static (non-variable) template text
appears in-order in the rendered output. This is the primary signal for
template identification:

| Template | Typical confidence |
|---|---|
| Correct template | 0.08–0.89 |
| Wrong template | ~0.005 |

A threshold of **0.05** cleanly separates correct from wrong templates.
Lower confidence on a correct template indicates the rendered output
contains substantial custom content not in the template (e.g., custom
workflow steps, abbreviated QA rounds).

## Bulk Testing

The `bulk_test.py` script validates the extractor against hundreds of
rendered ATM payloads.

### Prerequisites

You need:
1. This repo checked out
2. The `atm-core` repo with `.claude/skills/codex-orchestration/` templates
3. Rendered payloads in `~/.config/atm/share/atm-dev/`

```bash
# Clone if needed
git clone https://github.com/randlee/sc-compose.git
cd sc-compose
git checkout prototype/reverse-extract
```

### Run

```bash
cd prototype/reverse_extract
python3 bulk_test.py
```

Output:
```
Found 500 XML files in ~/.config/atm/share/atm-dev

============================================================
BULK TEST RESULTS
============================================================
  Pass:  194
  Fail:  3
  Skip:  288
  Error: 15
  Total: 500

Skip reasons:
  root=atm-task: 272
  root=repo-fix-task: 9
  root=atm-review: 2

Sample passing extractions (first 5):
  ac0-plan-qa2.xml
    task_id=AC0-PLAN-QA-2, sprint=plan/phase-AC, branch=plan/phase-AC
  ac8-impl-qa-payload.xml
    task_id=AC8-IMPL-QA, sprint=AC.8, branch=feature/pAC-s8-...
```

### Duplicating the Test Corpus

If you don't have the ATM share directory, generate test payloads:

```bash
# 1. Render from templates using known variables
for vars_file in vars/*.json; do
    sc-compose render qa-template.xml.j2 \
        --var-file "$vars_file" \
        -o "test-output/$(basename $vars_file .json).xml"
done

# 2. Run the extractor on generated output and verify round-trip
for f in test-output/*.xml; do
    python3 -c "
from prototype.reverse_extract import extract_variables
result = extract_variables('qa-template.xml.j2', '$f',
    include_vars=['task_id', 'sprint', 'branch'])
print(f'{result[\"task_id\"]} | {result[\"_confidence\"]:.3f}')
"
done
```

## Architecture

```
prototype/reverse_extract/
├── __init__.py      # Public API: extract_variables
├── extractor.py     # Core: parser, discover, extract, confidence
└── bulk_test.py     # Bulk validation against ATM payloads
```

### Extraction Pipeline

1. **Parse template frontmatter** → `required_variables` + `optional_variables`
2. **Discover variable bindings** — find all `{{ var }}` in body, classify each:
   - `attribute`: `<tag attr="{{ var }}">`
   - `element_text`: `<tag>{{ var }}</tag>`
   - `block_text`: standalone `{{ var }}` between enclosing tags
3. **Parse rendered XML** with ElementTree
4. **Extract values** via XPath derived from template structure
5. **Compute confidence** — % of static template text matching rendered output

### Supported Formats

| Format | Extraction | Confidence | Status |
|---|---|---|---|
| XML | ✓ | ✓ | Production-ready |
| JSON | — | ✓ | Planned |
| Markdown | — | ✓ | Planned |

## Integration with sc-compose (Planned)

```bash
# Extract vars from known template
sc-compose extract qa-template.xml.j2 rendered.xml --vars task_id,sprint,branch

# Identify which template produced an output
sc-compose identify rendered.xml --templates-dir .claude/skills/codex-orchestration/
# → qa-template.xml.j2 (confidence: 0.885)
#    dev-template.xml.j2 (confidence: 0.005)
```

## Related

- `prototype/multipass/` — multi-pass template rendering (forward direction)
- `sc-compose template-init` — convert concrete files to templates
- `sc-compose verify` — drift check between template output and deployed file
