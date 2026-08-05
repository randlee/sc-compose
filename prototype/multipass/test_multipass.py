"""Tests for multi-pass template system."""

from .types import ParsedTemplate, PassHeader, RenderContext
from .parser import parse_template
from .discover import discover_tokens, discover_all_tokens
from .renderer import render_pass, render_all


# ── Parser tests ──────────────────────────────────────────────────────────


def test_parse_no_headers():
    parsed = parse_template("hello world")
    assert parsed.passes == []
    assert parsed.body == "hello world"


def test_parse_single_header_no_pass_field():
    """Backward compat: no `pass` field defaults to pass 1."""
    parsed = parse_template("---\nname: world\n---\nhello {{ name }}")
    assert len(parsed.passes) == 1
    assert parsed.passes[0].pass_number == 1
    assert parsed.passes[0].defaults == {}
    assert parsed.body == "hello {{ name }}"


def test_parse_single_header_with_pass():
    parsed = parse_template("---\npass: 2\nrequired_variables: [team]\n---\n{{{ team }}}")
    assert parsed.passes[0].pass_number == 2
    assert parsed.passes[0].required_variables == ["team"]


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
    assert parsed.passes[0].defaults == {"team_name": "wyvern"}
    assert parsed.passes[1].pass_number == 1
    assert parsed.passes[1].required_variables == ["task.title"]
    assert "{{{{ team_name }}}}" in parsed.body
    assert "{{ task.title }}" in parsed.body


def test_parse_empty_headers():
    text = "---\n---\n---\n---\nbody"
    parsed = parse_template(text)
    assert len(parsed.passes) == 2
    assert parsed.passes[0].pass_number == 1  # default
    assert parsed.passes[1].pass_number == 1
    assert parsed.body == "body"


def test_parse_with_dots_delimiter():
    text = "---\nname: foo\n...\n---\n...\nbody"
    parsed = parse_template(text)
    assert len(parsed.passes) == 2
    assert parsed.passes[0].defaults == {}


def test_parse_body_preserves_late_markdown_rule():
    text = "---\ndefaults: { name: world }\n---\nhello {{ name }}\n---\nbody rule\n"
    parsed = parse_template(text)
    assert len(parsed.passes) == 1
    assert parsed.body == "hello {{ name }}\n---\nbody rule\n"


def test_parse_duplicate_pass_numbers_fail():
    text = "---\npass: 2\n---\n---\npass: 2\n---\nbody"
    try:
        parse_template(text)
    except ValueError as error:
        assert "duplicate pass number" in str(error)
    else:
        raise AssertionError("expected duplicate pass numbers to fail")


# ── Token discovery tests ──────────────────────────────────────────────────


def test_discover_standard_braces():
    tokens = discover_tokens("hello {{ name }}, {{ repo.path }}", brace_count=2)
    assert tokens == {"name", "repo.path"}


def test_discover_triple_braces():
    tokens = discover_tokens("hello {{{ team }}} and {{{ agent.name }}}", brace_count=3)
    assert tokens == {"team", "agent.name"}


def test_discover_quadruple_braces():
    tokens = discover_tokens("{{{{ a }}}} {{{{ b.c }}}}", brace_count=4)
    assert tokens == {"a", "b.c"}


def test_discover_skips_keywords():
    tokens = discover_tokens(
        "{% if true %}{{ name }}{% endif %}{% for x in items %}{{ x }}{% endfor %}",
        brace_count=2,
    )
    assert "name" in tokens
    assert "items" in tokens
    assert "x" not in tokens  # loop variable, bound
    assert "if" not in tokens
    assert "for" not in tokens


def test_discover_mixed_braces():
    """triple-brace scan should NOT pick up double-brace vars."""
    tokens = discover_tokens("{{{ outer }}} and {{ inner }}", brace_count=3)
    assert "outer" in tokens
    assert "{{ inner }}" not in tokens  # inner is literal text to pass 2
    # Actually, "inner" appears in `{{ inner }}` but the scan looks for {{{,
    # so `{{ inner }}` is just literal text with `inner` inside it.
    # Let's check: "{{ inner }}" — the discover scans for "{{{", finds the
    # triple-brace around "outer", then "{{ inner }}" is just text.
    # inner would appear in the text as literal but our identifier collector
    # only runs inside delimited expressions.
    # Correct: "inner" should NOT be in tokens for brace_count=3 scan
    assert "inner" not in tokens


def test_discover_all_tokens():
    text = """---
pass: 2
required_variables: [team]
---
---
pass: 1
---
{{{ team }}} {{ task }}"""
    parsed = parse_template(text)
    tokens = discover_all_tokens(parsed)
    assert tokens[2] == {"team"}     # triple-brace scan
    assert tokens[1] == {"task"}     # double-brace scan


# ── Render tests ───────────────────────────────────────────────────────────


def test_render_single_pass():
    parsed = parse_template("---\ndefaults: { name: world }\n---\nhello {{ name }}")
    ctx = RenderContext(pass_number=1, variables={})
    output, remaining = render_pass(parsed, ctx)
    assert output == "hello world"
    assert remaining.passes == []
    assert remaining.body == "hello world"


def test_render_pass_custom_delimiters():
    text = "---\npass: 2\ndefaults: { team: wyvern }\n---\n{{{ team }}} rules"
    parsed = parse_template(text)
    ctx = RenderContext(pass_number=2, variables={})
    output, remaining = render_pass(parsed, ctx)
    assert output == "wyvern rules"
    assert remaining.passes == []
    assert remaining.body == "wyvern rules"


def test_render_pass_overrides_defaults():
    text = "---\ndefaults: { name: default }\n---\n{{ name }}"
    parsed = parse_template(text)
    ctx = RenderContext(pass_number=1, variables={"name": "explicit"})
    output, _ = render_pass(parsed, ctx)
    assert output == "explicit"


def test_render_two_passes():
    text = """---
pass: 2
defaults: { team: wyvern }
---
---
pass: 1
---
{{{ team }}} likes {{ tool }}"""

    parsed = parse_template(text)

    # Pass 2
    ctx2 = RenderContext(pass_number=2, variables={})
    output2, remaining = render_pass(parsed, ctx2)
    assert output2 == "wyvern likes {{ tool }}"
    assert len(remaining.passes) == 1
    assert remaining.passes[0].pass_number == 1

    # Pass 1
    ctx1 = RenderContext(pass_number=1, variables={"tool": "Rust"})
    output1, final = render_pass(remaining, ctx1)
    assert output1 == "wyvern likes Rust"
    assert final.passes == []


def test_render_all():
    text = """---
pass: 2
defaults: { team: wyvern }
---
---
pass: 1
---
{{{ team }}} {{ verb }} {{ lang }}"""

    parsed = parse_template(text)
    result = render_all(parsed, [
        RenderContext(pass_number=2, variables={}),
        RenderContext(pass_number=1, variables={"verb": "loves", "lang": "Rust"}),
    ])
    assert result == "wyvern loves Rust"


# ── Backward compat tests ──────────────────────────────────────────────────


def test_current_sc_compose_template_renders_identically():
    """A file identical to current sc-compose single-header templates."""
    text = "---\nrequired_variables: [name]\ndefaults: { name: world }\n---\nhello {{ name }}"
    parsed = parse_template(text)
    assert len(parsed.passes) == 1
    assert parsed.passes[0].pass_number == 1

    ctx = RenderContext(pass_number=1, variables={})
    output, _ = render_pass(parsed, ctx)
    assert output == "hello world"


if __name__ == "__main__":
    import sys

    # Run all tests
    tests = [
        (k, v) for k, v in globals().items()
        if k.startswith("test_") and callable(v)
    ]
    failed = 0
    for name, fn in tests:
        try:
            fn()
            print(f"  PASS {name}")
        except Exception as e:
            print(f"  FAIL {name}: {e}")
            failed += 1

    print(f"\n{len(tests) - failed}/{len(tests)} passed")
    sys.exit(failed)
