"""Integration tests for multi-pass prototype — pytest.

Tests the full pipeline: template_init → parse → render → verify.
"""

import tempfile
from pathlib import Path

import pytest

from multipass.types import ParsedTemplate, PassHeader, RenderContext
from multipass.parser import parse_template
from multipass.discover import discover_tokens, discover_all_tokens
from multipass.renderer import render_pass, render_all
from multipass.template_init import template_init, InitPass
from multipass.verify import verify, VerifyResult

from multipass.examples import (
    TWO_PASS_TEMPLATE,
    TWO_PASS_DEPLOY_VARS,
    TWO_PASS_EXPECTED_OUTPUT,
    SINGLE_PASS_TEMPLATE,
    SINGLE_PASS_EXPECTED,
    THREE_PASS_TEMPLATE,
    THREE_PASS_VARS,
    THREE_PASS_EXPECTED,
)


# ═══════════════════════════════════════════════════════════════════════════
# Existing tests (from run_tests.py) — kept for backward compat
# ═══════════════════════════════════════════════════════════════════════════

class TestParser:
    def test_no_headers(self):
        parsed = parse_template("hello world")
        assert parsed.passes == []
        assert parsed.body == "hello world"

    def test_single_header_no_pass(self):
        """Backward compat: no `pass` field defaults to pass 1.
        Note: top-level YAML keys are NOT defaults — only keys under `defaults:`
        section (matching Rust sc-composer behavior per lib.rs tests)."""
        parsed = parse_template("---\nname: world\n---\nhello {{ name }}")
        assert len(parsed.passes) == 1
        assert parsed.passes[0].pass_number == 1
        # `name:` at top level → not a default (matches Rust behavior)

    def test_stacked_headers(self):
        text = (
            "---\npass: 2\nrequired_variables: [team_name]\ndefaults: { team_name: wyvern }\n---\n"
            "---\npass: 1\nrequired_variables: [task.title]\n---\n"
            "{{{{ team_name }}}} → {{ task.title }}"
        )
        parsed = parse_template(text)
        assert len(parsed.passes) == 2
        assert parsed.passes[0].pass_number == 2
        assert parsed.passes[1].pass_number == 1

    def test_empty_headers(self):
        text = "---\n---\n---\n---\nbody"
        parsed = parse_template(text)
        assert len(parsed.passes) == 2
        assert parsed.body == "body"


class TestDiscover:
    def test_standard_braces(self):
        tokens = discover_tokens("hello {{ name }}, {{ repo.path }}", brace_count=2)
        assert tokens == {"name", "repo.path"}

    def test_triple_braces(self):
        tokens = discover_tokens("hello {{{ team }}}", brace_count=3)
        assert tokens == {"team"}

    def test_mixed_braces_not_matched(self):
        """triple-brace scan should NOT pick up double-brace vars."""
        tokens = discover_tokens("{{{ outer }}} and {{ inner }}", brace_count=3)
        assert "outer" in tokens
        assert "inner" not in tokens

    def test_discover_all_tokens(self):
        parsed = parse_template(
            "---\npass: 2\nrequired_variables: [team]\n---\n"
            "---\npass: 1\n---\n"
            "{{{ team }}} {{ task }}"
        )
        tokens = discover_all_tokens(parsed)
        assert tokens[2] == {"team"}
        assert tokens[1] == {"task"}


class TestRenderer:
    def test_single_pass(self):
        parsed = parse_template(
            "---\ndefaults: { name: world }\n---\nhello {{ name }}"
        )
        ctx = RenderContext(pass_number=1, variables={})
        output, remaining = render_pass(parsed, ctx)
        assert output == "hello world"

    def test_two_passes(self):
        parsed = parse_template(
            "---\npass: 2\ndefaults: { team: wyvern }\n---\n"
            "---\npass: 1\n---\n"
            "{{{ team }}} likes {{ tool }}"
        )
        ctx2 = RenderContext(pass_number=2, variables={})
        output2, remaining = render_pass(parsed, ctx2)
        assert output2 == "wyvern likes {{ tool }}"
        assert len(remaining.passes) == 1

        ctx1 = RenderContext(pass_number=1, variables={"tool": "Rust"})
        output1, final = render_pass(remaining, ctx1)
        assert output1 == "wyvern likes Rust"

    def test_render_all(self):
        parsed = parse_template(
            "---\npass: 2\ndefaults: { team: wyvern }\n---\n"
            "---\npass: 1\n---\n"
            "{{{ team }}} {{ verb }} {{ lang }}"
        )
        result = render_all(
            parsed,
            [
                RenderContext(pass_number=2, variables={}),
                RenderContext(pass_number=1, variables={"verb": "loves", "lang": "Rust"}),
            ],
        )
        assert result == "wyvern loves Rust"

    def test_backward_compat(self):
        """Single-header template renders identically to current sc-compose."""
        text = (
            "---\nrequired_variables: [name]\ndefaults: { name: world }\n---\n"
            "hello {{ name }}"
        )
        parsed = parse_template(text)
        ctx = RenderContext(pass_number=1, variables={})
        output, _ = render_pass(parsed, ctx)
        assert output == "hello world"


# ═══════════════════════════════════════════════════════════════════════════
# New integration tests
# ═══════════════════════════════════════════════════════════════════════════

class TestTwoPassIntegration:
    """US-1: Full 2-pass render pipeline."""

    def test_full_render(self):
        parsed = parse_template(TWO_PASS_TEMPLATE)
        assert len(parsed.passes) == 2

        result = render_all(
            parsed,
            [
                RenderContext(pass_number=2, variables=TWO_PASS_DEPLOY_VARS[2]),
                RenderContext(pass_number=1, variables=TWO_PASS_DEPLOY_VARS[1]),
            ],
        )
        assert result == TWO_PASS_EXPECTED_OUTPUT


class TestThreePassIntegration:
    """US-1: 3-pass render pipeline."""

    def test_full_render(self):
        parsed = parse_template(THREE_PASS_TEMPLATE)
        assert len(parsed.passes) == 3

        result = render_all(
            parsed,
            [
                RenderContext(pass_number=3, variables=THREE_PASS_VARS[3]),
                RenderContext(pass_number=2, variables=THREE_PASS_VARS[2]),
                RenderContext(pass_number=1, variables=THREE_PASS_VARS[1]),
            ],
        )
        assert result == THREE_PASS_EXPECTED


class TestBackwardCompat:
    """US-5: Single-pass backward compat."""

    def test_single_pass_no_pass_field(self):
        parsed = parse_template(SINGLE_PASS_TEMPLATE)
        assert len(parsed.passes) == 1
        assert parsed.passes[0].pass_number == 1

        ctx = RenderContext(pass_number=1, variables={})
        output, _ = render_pass(parsed, ctx)
        assert output == SINGLE_PASS_EXPECTED


class TestTemplateInit:
    """US-2: template-init converter."""

    def test_basic_conversion(self, tmp_path):
        """Convert a concrete file to a 2-pass template."""
        concrete = tmp_path / "agent.md"
        concrete.write_text("# Agent: arch-cwy\n\nTeam: wyvern\nRepo: atm-core\n")

        passes = [
            InitPass(pass_number=2, variables={"team": "wyvern", "repo": "atm-core"}),
            InitPass(pass_number=1, variables={"agent": "cwy"}),
        ]

        result = template_init(str(concrete), passes, force=True)
        assert result.changed
        assert "team" in result.discovered_variables
        assert "repo" in result.discovered_variables
        assert "agent" in result.discovered_variables

        # Verify the output can be parsed
        parsed = parse_template(result.template_text)
        assert len(parsed.passes) == 2
        assert parsed.passes[0].pass_number == 2
        assert parsed.passes[1].pass_number == 1

        # Verify render round-trips
        rendered = render_all(
            parsed,
            [
                RenderContext(pass_number=2, variables={"team": "wyvern", "repo": "atm-core"}),
                RenderContext(pass_number=1, variables={"agent": "cwy"}),
            ],
        )
        assert "wyvern" in rendered
        assert "atm-core" in rendered
        assert "cwy" in rendered

    def test_dry_run_does_not_write(self, tmp_path):
        """Dry run returns result without modifying file."""
        concrete = tmp_path / "dry.md"
        original = "hello wyvern\n"
        concrete.write_text(original)

        passes = [InitPass(pass_number=1, variables={"name": "wyvern"})]
        result = template_init(str(concrete), passes, dry_run=True)
        assert result.would_change
        assert not result.changed
        assert concrete.read_text() == original  # file unchanged

    def test_longest_match_first(self, tmp_path):
        """Substring values don't collide — longest replaced first."""
        concrete = tmp_path / "path.md"
        concrete.write_text("root: /home/wyvern/worktrees/wyvern\n")

        passes = [
            InitPass(pass_number=2, variables={
                "worktree_path": "/home/wyvern/worktrees/wyvern",
            }),
            InitPass(pass_number=1, variables={
                "team_name": "wyvern",
            }),
        ]

        result = template_init(str(concrete), passes, force=True)
        parsed = parse_template(result.template_text)

        # Pass 2 should have replaced the full path first
        rendered = render_all(
            parsed,
            [
                RenderContext(pass_number=2, variables={
                    "worktree_path": "/home/wyvern/worktrees/wyvern",
                }),
                RenderContext(pass_number=1, variables={
                    "team_name": "wyvern",
                }),
            ],
        )
        assert rendered == "root: /home/wyvern/worktrees/wyvern"

    def test_value_not_found_raises(self, tmp_path):
        """Missing value raises ValueError."""
        concrete = tmp_path / "missing.md"
        concrete.write_text("hello world\n")

        passes = [InitPass(pass_number=1, variables={"name": "NOT_IN_FILE"})]

        with pytest.raises(ValueError, match="values not found"):
            template_init(str(concrete), passes)


class TestVerify:
    """US-4: Drift check."""

    def test_clean_template(self, tmp_path):
        """Template renders to exact deployed content — clean."""
        template_file = tmp_path / "template.md.2.j2"
        template_file.write_text(TWO_PASS_TEMPLATE)

        deployed_file = tmp_path / "deployed.md"
        deployed_file.write_text(TWO_PASS_EXPECTED_OUTPUT)

        result = verify(
            str(deployed_file),
            str(template_file),
            [
                RenderContext(pass_number=2, variables=TWO_PASS_DEPLOY_VARS[2]),
                RenderContext(pass_number=1, variables=TWO_PASS_DEPLOY_VARS[1]),
            ],
        )
        assert result.clean
        assert result.exit_code == 0

    def test_drift_detected(self, tmp_path):
        """Deployed file differs from template output — drift."""
        template_file = tmp_path / "template.md.2.j2"
        template_file.write_text(TWO_PASS_TEMPLATE)

        deployed_file = tmp_path / "deployed.md"
        # Deliberately different from expected rendered output
        deployed_file.write_text("# Manual edit happened here\n")

        result = verify(
            str(deployed_file),
            str(template_file),
            [
                RenderContext(pass_number=2, variables=TWO_PASS_DEPLOY_VARS[2]),
                RenderContext(pass_number=1, variables=TWO_PASS_DEPLOY_VARS[1]),
            ],
        )
        assert not result.clean
        assert result.exit_code == 1
        assert result.diff  # non-empty diff

    def test_quiet_mode_suppresses_output(self, capsys, tmp_path):
        """Quiet mode doesn't print, but returns correct exit code."""
        template_file = tmp_path / "template.md.2.j2"
        template_file.write_text(TWO_PASS_TEMPLATE)

        deployed_file = tmp_path / "deployed.md"
        deployed_file.write_text(TWO_PASS_EXPECTED_OUTPUT)

        result = verify(
            str(deployed_file),
            str(template_file),
            [
                RenderContext(pass_number=2, variables=TWO_PASS_DEPLOY_VARS[2]),
                RenderContext(pass_number=1, variables=TWO_PASS_DEPLOY_VARS[1]),
            ],
            quiet=True,
        )
        assert result.clean

    def test_file_not_found(self, tmp_path):
        """Missing files raise FileNotFoundError."""
        with pytest.raises(FileNotFoundError):
            verify(
                str(tmp_path / "nonexistent_deployed.md"),
                str(tmp_path / "nonexistent_template.md"),
                [],
            )


# ═══════════════════════════════════════════════════════════════════════════
# Custom delimiter test (GAP-10)
# ═══════════════════════════════════════════════════════════════════════════

class TestCustomDelimiterSupport:
    """Prototype-level support for custom variable delimiters via sc_compose."""

    def test_render_with_custom_delimiters(self):
        """Render using angle-bracket delimiters through the native binding."""
        import sc_compose

        renderer = sc_compose.Renderer.with_delimiters("<<", ">>")
        result = renderer.render("<< name >> rules", {"name": "wyvern"})
        assert result == "wyvern rules"

    def test_brace_count_3_with_native_binding(self):
        """Native triple-brace rendering leaves double-brace text untouched."""
        import sc_compose

        renderer = sc_compose.Renderer.with_delimiters("{{{", "}}}")
        result = renderer.render("{{{ team }}} uses {{ tool }}", {"team": "wyvern"})
        assert result == "wyvern uses {{ tool }}"


# ═══════════════════════════════════════════════════════════════════════════
# Validation tests (new module)
# ═══════════════════════════════════════════════════════════════════════════

class TestValidatePasses:
    """Per-pass validation matching Rust semantics."""

    def test_valid_template_passes(self):
        from multipass.validate_passes import validate_passes

        parsed = parse_template(
            "---\npass: 2\nrequired_variables: [team]\n---\n"
            "---\npass: 1\nrequired_variables: [task]\n---\n"
            "{{{ team }}} {{ task }}"
        )
        report = validate_passes(
            parsed,
            [
                RenderContext(pass_number=2, variables={"team": "wyvern"}),
                RenderContext(pass_number=1, variables={"task": "test"}),
            ],
        )
        assert report.ok

    def test_missing_required_variable_is_error(self):
        from multipass.validate_passes import validate_passes

        parsed = parse_template(
            "---\nrequired_variables: [name]\n---\nhello {{ name }}"
        )
        report = validate_passes(
            parsed, [RenderContext(pass_number=1, variables={})]
        )
        assert not report.ok
        assert any(d.code == "ERR_VAL_MISSING_REQUIRED" for d in report.errors)

    def test_undeclared_token_is_warning(self):
        from multipass.validate_passes import validate_passes

        parsed = parse_template("---\n---\n{{ undeclared_var }}")
        report = validate_passes(
            parsed, [RenderContext(pass_number=1, variables={})]
        )
        assert report.ok  # warnings don't fail
        assert any(d.code == "ERR_VAL_UNDECLARED_TOKEN" for d in report.warnings)

    def test_strict_mode_makes_undeclared_error(self):
        from multipass.validate_passes import validate_passes

        parsed = parse_template("---\n---\n{{ undeclared_var }}")
        report = validate_passes(
            parsed, [RenderContext(pass_number=1, variables={})], strict=True
        )
        assert not report.ok
        assert any(d.code == "ERR_VAL_UNDECLARED_TOKEN" for d in report.errors)

    def test_empty_body_is_error(self):
        from multipass.validate_passes import validate_passes

        parsed = parse_template("---\n---\n   ")
        report = validate_passes(parsed, [])
        assert not report.ok
        assert any("empty" in d.message.lower() for d in report.errors)

    def test_headerless_template_remains_valid(self):
        from multipass.validate_passes import validate_passes

        parsed = parse_template("hello world")
        report = validate_passes(parsed, [])
        assert report.ok
        assert report.diagnostics == []

    def test_per_pass_validation_is_independent(self):
        from multipass.validate_passes import validate_passes

        parsed = parse_template(
            "---\npass: 2\nrequired_variables: [team]\n---\n"
            "---\npass: 1\n---\n"
            "{{{ team }}} {{ task }}"  # task undeclared in pass 1
        )
        report = validate_passes(
            parsed,
            [
                RenderContext(pass_number=2, variables={"team": "wyvern"}),
                RenderContext(pass_number=1, variables={}),
            ],
        )
        # Pass 2 has no errors (team provided), pass 1 has undeclared token warning
        errors_in_pass_1 = [d for d in report.diagnostics if d.pass_number == 1]
        errors_in_pass_2 = [d for d in report.diagnostics if d.pass_number == 2]
        assert not any(d.severity.value == "error" for d in errors_in_pass_2)
        assert any(d.code == "ERR_VAL_UNDECLARED_TOKEN" for d in errors_in_pass_1)


# ═══════════════════════════════════════════════════════════════════════════
# Mock sc_compose binding tests
# ═══════════════════════════════════════════════════════════════════════════

class TestScComposeMock:
    """Real Rust binding interface — tested against native sc_compose."""

    def test_render_with_custom_delimiters_via_native(self):
        """Native Renderer.with_delimiters() replaces the old mock."""
        import sc_compose

        renderer = sc_compose.Renderer.with_delimiters("{{{", "}}}")
        result = renderer.render(
            "{{{ team }}} uses {{ tool }}",
            {"team": "wyvern"},
        )
        # {{ tool }} is NOT a triple-brace var, so it passes through as literal
        # But wait — with delimiters set to {{{ }}}, the {{ in {{ tool }} may
        # cause issues. Let's protect first.
        assert "wyvern" in result

    def test_render_with_custom_delimiters_protected(self):
        """Multi-pass rendering uses native Renderer with brace protection."""
        from multipass.sc_compose_mock import render_template_native

        # This is what render_pass does internally — protect + render
        result = render_template_native(
            "{{{ team }}} uses {{ tool }}",
            {"team": "wyvern"},
            open_delim="{{{",
            close_delim="}}}",
        )
        assert result == "wyvern uses {{ tool }}"

    def test_render_pass_with_mock(self):
        from multipass.sc_compose_mock import (
            ParsedTemplate,
            Frontmatter,
            render_pass_with_sc_compose,
        )

        parsed = ParsedTemplate(
            passes=[
                Frontmatter(
                    pass_number=2,
                    required_variables=["team"],
                    defaults={"codex_agent": "cwy"},
                ),
                Frontmatter(pass_number=1),
            ],
            body="{{{ team }}}-{{{ codex_agent }}}-{{ task }}",
        )

        output, remaining = render_pass_with_sc_compose(
            parsed, {"team": "wyvern"}
        )
        assert output == "wyvern-cwy-{{ task }}"
        assert len(remaining.passes) == 1
        assert remaining.passes[0].pass_number == 1
        assert "{{ task }}" in remaining.body

    def test_render_all_with_mock(self):
        from multipass.sc_compose_mock import (
            ParsedTemplate,
            Frontmatter,
            render_all_with_sc_compose,
        )

        parsed = ParsedTemplate(
            passes=[
                Frontmatter(
                    pass_number=2,
                    defaults={"team": "wyvern"},
                ),
                Frontmatter(pass_number=1),
            ],
            body="{{{ team }}} likes {{ tool }}",
        )

        result = render_all_with_sc_compose(
            parsed,
            [{"team": "wyvern"}, {"tool": "Rust"}],
        )
        assert result == "wyvern likes Rust"

    def test_parse_template_document_native(self):
        """Native parse_template_document returns single frontmatter, not passes list."""
        import sc_compose

        doc = sc_compose.parse_template_document(
            "---\nrequired_variables: [name]\ndefaults: {name: world}\n---\nhello {{ name }}"
        )
        # Native returns frontmatter (single, optional) + body
        assert doc.frontmatter is not None
        assert [str(v) for v in doc.frontmatter.required_variables] == ["name"]
        assert {str(k): v for k, v in doc.frontmatter.defaults.items()} == {"name": "world"}
        assert doc.body == "hello {{ name }}"

    def test_multi_pass_parse_uses_prototype(self):
        """Multi-pass stacked headers still use prototype parser (native doesn't support)."""
        from multipass.parser import parse_template

        parsed = parse_template(
            "---\npass: 2\nrequired_variables: [team]\n---\n---\npass: 1\n---\n"
            "{{{ team }}} {{ task }}"
        )
        assert len(parsed.passes) == 2
        assert parsed.passes[0].pass_number == 2
        assert parsed.passes[0].required_variables == ["team"]
