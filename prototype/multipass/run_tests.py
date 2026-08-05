import sys
import traceback
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from multipass.types import ParsedTemplate, PassHeader, RenderContext
from multipass.parser import parse_template
from multipass.discover import discover_tokens, discover_all_tokens
from multipass.renderer import render_pass, render_all


tests = []

def test(fn):
    tests.append(fn)
    return fn


@test
def test_parse_no_headers():
    parsed = parse_template("hello world")
    assert parsed.passes == []
    assert parsed.body == "hello world"

@test
def test_parse_single_header_no_pass_field():
    parsed = parse_template("---\ndefaults: {name: world}\n---\nhello {{ name }}")
    assert len(parsed.passes) == 1
    assert parsed.passes[0].pass_number == 1
    assert parsed.passes[0].defaults == {"name": "world"}
    assert "{{ name }}" in parsed.body

@test
def test_parse_single_header_with_pass():
    parsed = parse_template("---\npass: 2\nrequired_variables: [team]\n---\n{{{ team }}}")
    assert parsed.passes[0].pass_number == 2
    assert parsed.passes[0].required_variables == ["team"]

@test
def test_parse_stacked_headers():
    text = """---
pass: 2
required_variables: [team_name]
defaults: { team_name: wyvern }
---
---
pass: 1
required_variables: [task.title]
---
{{{{ team_name }}}} → {{ task.title }}"""
    parsed = parse_template(text)
    assert len(parsed.passes) == 2
    assert parsed.passes[0].pass_number == 2
    assert parsed.passes[0].required_variables == ["team_name"]
    assert parsed.passes[1].pass_number == 1
    assert parsed.passes[1].required_variables == ["task.title"]

@test
def test_parse_empty_headers():
    text = "---\n---\n---\n---\nbody"
    parsed = parse_template(text)
    assert len(parsed.passes) == 2, f"expected 2 passes, got {len(parsed.passes)}: headers={[(p.pass_number, p.required_variables) for p in parsed.passes]}"

@test
def test_parse_with_dots_delimiter():
    text = "---\nname: foo\n...\n---\n...\nbody"
    parsed = parse_template(text)
    assert len(parsed.passes) == 2, f"got {len(parsed.passes)}"

@test
def test_discover_standard_braces():
    tokens = discover_tokens("hello {{ name }}, {{ repo.path }}", brace_count=2)
    assert tokens == {"name", "repo.path"}

@test
def test_discover_triple_braces():
    tokens = discover_tokens("hello {{{ team }}} and {{{ agent.name }}}", brace_count=3)
    assert tokens == {"team", "agent.name"}

@test
def test_discover_quadruple_braces():
    tokens = discover_tokens("{{{{ a }}}} {{{{ b.c }}}}", brace_count=4)
    assert tokens == {"a", "b.c"}

@test
def test_discover_skips_keywords():
    tokens = discover_tokens("{% if true %}{{ name }}{% endif %}{% for x in items %}{{ x }}{% endfor %}", brace_count=2)
    assert "name" in tokens
    assert "items" in tokens
    assert "x" not in tokens

@test
def test_discover_mixed_braces():
    tokens = discover_tokens("{{{ outer }}} and {{ inner }}", brace_count=3)
    assert "outer" in tokens
    assert "inner" not in tokens

@test
def test_discover_all_tokens():
    text = "---\npass: 2\n---\n---\npass: 1\n---\n{{{ team }}} {{ task }}"
    parsed = parse_template(text)
    tokens = discover_all_tokens(parsed)
    assert tokens.get(2) == {"team"}
    assert tokens.get(1) == {"task"}

@test
def test_render_single_pass():
    parsed = parse_template("---\ndefaults: { name: world }\n---\nhello {{ name }}")
    ctx = RenderContext(pass_number=1, variables={})
    output, remaining = render_pass(parsed, ctx)
    assert output == "hello world"
    assert remaining.passes == []
    assert remaining.body == "hello world"

@test
def test_render_pass_custom_delimiters():
    text = "---\npass: 2\ndefaults: { team: wyvern }\n---\n{{{ team }}} rules"
    parsed = parse_template(text)
    ctx = RenderContext(pass_number=2, variables={})
    output, remaining = render_pass(parsed, ctx)
    assert output == "wyvern rules"
    assert remaining.passes == []

@test
def test_render_pass_overrides_defaults():
    text = "---\ndefaults: { name: default }\n---\n{{ name }}"
    parsed = parse_template(text)
    ctx = RenderContext(pass_number=1, variables={"name": "explicit"})
    output, _ = render_pass(parsed, ctx)
    assert output == "explicit"

@test
def test_render_two_passes():
    text = "---\npass: 2\ndefaults: { team: wyvern }\n---\n---\npass: 1\n---\n{{{ team }}} likes {{ tool }}"
    parsed = parse_template(text)
    ctx2 = RenderContext(pass_number=2, variables={})
    output2, remaining = render_pass(parsed, ctx2)
    assert output2 == "wyvern likes {{ tool }}"
    assert len(remaining.passes) == 1
    ctx1 = RenderContext(pass_number=1, variables={"tool": "Rust"})
    output1, final = render_pass(remaining, ctx1)
    assert output1 == "wyvern likes Rust"
    assert final.passes == []

@test
def test_render_all():
    text = "---\npass: 2\ndefaults: { team: wyvern }\n---\n---\npass: 1\n---\n{{{ team }}} {{ verb }} {{ lang }}"
    parsed = parse_template(text)
    result = render_all(parsed, [
        RenderContext(pass_number=2, variables={}),
        RenderContext(pass_number=1, variables={"verb": "loves", "lang": "Rust"}),
    ])
    assert result == "wyvern loves Rust"

@test
def test_current_sc_compose_template_renders_identically():
    text = "---\nrequired_variables: [name]\ndefaults: { name: world }\n---\nhello {{ name }}"
    parsed = parse_template(text)
    assert len(parsed.passes) == 1
    assert parsed.passes[0].pass_number == 1
    ctx = RenderContext(pass_number=1, variables={})
    output, _ = render_pass(parsed, ctx)
    assert output == "hello world"


if __name__ == "__main__":
    failed = 0
    for fn in tests:
        try:
            fn()
            print(f"  PASS {fn.__name__}")
        except Exception as e:
            print(f"  FAIL {fn.__name__}: {e}")
            traceback.print_exc()
            failed += 1
    print(f"\n{len(tests) - failed}/{len(tests)} passed")
