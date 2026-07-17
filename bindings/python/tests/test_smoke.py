from __future__ import annotations

import textwrap
from pathlib import Path

import pytest

import sc_compose


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(contents), encoding="utf-8")


def make_file_request(root: Path, name: str, **kwargs: object) -> sc_compose.ComposeRequest:
    return sc_compose.ComposeRequest(
        root=root,
        mode=sc_compose.ComposeMode.file(name),
        **kwargs,
    )


def test_import_surface_exposes_c2_api() -> None:
    required = [
        "BUILTIN_VARIABLE_NAMES",
        "ComposeMode",
        "ComposePolicy",
        "ComposeRequest",
        "ComposeResult",
        "ConfiningRoot",
        "Diagnostic",
        "DiagnosticCode",
        "DiagnosticSeverity",
        "ExpandedTemplate",
        "Frontmatter",
        "FrontmatterInitResult",
        "InitResult",
        "LoadedTemplateRequest",
        "NamedTemplateAsset",
        "ParsedTemplate",
        "ProfileKind",
        "ProfileName",
        "Renderer",
        "RenderedArtifact",
        "ResolveResult",
        "ResolverPolicy",
        "RuntimeKind",
        "ScComposeError",
        "ScConfigError",
        "ScIncludeError",
        "ScRenderError",
        "ScResolveError",
        "ScValidationError",
        "UnknownVariablePolicy",
        "ValidationReport",
        "VariableName",
        "VariableSource",
        "compose",
        "compose_file",
        "discover_tokens",
        "expand_includes",
        "frontmatter_init",
        "init_workspace",
        "input_value_from_yaml",
        "parse_template_document",
        "render_loaded_template",
        "render_template",
        "resolve_profile",
        "resolve_template_path",
        "to_forward_slash",
        "validate",
        "validate_input_value",
    ]

    for name in required:
        assert getattr(sc_compose, name) is not None


def test_non_reporting_surface_smoke(tmp_path: Path) -> None:
    write(
        tmp_path / "template.md.j2",
        """
        ---
        required_variables:
          - name
        defaults:
          role: reviewer
        ---
        hello {{ name }} from {{ role }}
        """,
    )
    write(tmp_path / "partials" / "child.md", "child line\n")
    write(tmp_path / "include-root.md.j2", "top\n@<partials/child.md>\n")
    write(tmp_path / ".claude" / "agents" / "reviewer.md", "agent body\n")
    write(tmp_path / "raw.md.j2", "hello {{ name }}\n")

    request = make_file_request(
        tmp_path,
        "template.md.j2",
        vars_input={"name": "world"},
        policy=sc_compose.ComposePolicy(
            unknown_variable_policy=sc_compose.UnknownVariablePolicy.IGNORE
        ),
    )

    result = sc_compose.compose(request)
    assert isinstance(result, sc_compose.ComposeResult)
    assert "hello world" in result.rendered_text
    assert "reviewer" in result.rendered_text
    assert result.resolve_result.resolved_path.endswith("template.md.j2")
    assert isinstance(result.warnings, list)

    result_alias = sc_compose.compose_file(request)
    assert result_alias.rendered_text == result.rendered_text

    report = sc_compose.validate(request)
    assert isinstance(report, sc_compose.ValidationReport)
    assert report.ok is True

    resolved = sc_compose.resolve_template_path(request)
    assert isinstance(resolved, sc_compose.ResolveResult)
    assert resolved.resolved_path.endswith("template.md.j2")

    profile_request = sc_compose.ComposeRequest(
        root=tmp_path,
        mode=sc_compose.ComposeMode.profile(
            sc_compose.ProfileKind.AGENT, "reviewer"
        ),
        runtime=sc_compose.RuntimeKind.CLAUDE,
    )
    profile_resolved = sc_compose.resolve_profile(profile_request)
    assert profile_resolved.resolved_path.endswith("reviewer.md")

    assert sc_compose.render_template("hi {{ name }}", {"name": "dev"}) == "hi dev"

    loaded = sc_compose.LoadedTemplateRequest(
        template_name="page.j2",
        template_text='{% include "partials/greeting.j2" %}',
        context={"name": "wheel"},
        supporting_templates=[
            sc_compose.NamedTemplateAsset(
                "partials/greeting.j2", "hello {{ name }}"
            )
        ],
    )
    artifact = sc_compose.render_loaded_template(loaded)
    assert isinstance(artifact, sc_compose.RenderedArtifact)
    assert artifact.rendered == "hello wheel"

    parsed = sc_compose.parse_template_document(
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n"
    )
    assert isinstance(parsed, sc_compose.ParsedTemplate)
    assert parsed.frontmatter is not None
    assert [str(name) for name in parsed.frontmatter.required_variables] == ["name"]
    assert parsed.body == "hello {{ name }}\n"

    expanded = sc_compose.expand_includes(tmp_path / "include-root.md.j2", tmp_path)
    assert isinstance(expanded, sc_compose.ExpandedTemplate)
    assert expanded.text == "top\nchild line\n"

    init_result = sc_compose.frontmatter_init(tmp_path / "raw.md.j2", dry_run=True)
    assert isinstance(init_result, sc_compose.FrontmatterInitResult)
    assert [str(name) for name in init_result.discovered_variables] == ["name"]
    assert init_result.would_change is True

    workspace_result = sc_compose.init_workspace(tmp_path, dry_run=True)
    assert isinstance(workspace_result, sc_compose.InitResult)
    assert workspace_result.prompts_dir.endswith(".prompts")
    assert isinstance(workspace_result.recommendations, list)

    sc_compose.validate_input_value({"items": ["a", "b"]})
    assert sc_compose.input_value_from_yaml("name: world\ncount: 2\n") == {
        "name": "world",
        "count": 2,
    }
    assert sc_compose.to_forward_slash(tmp_path / "partials" / "child.md").endswith(
        "partials/child.md"
    )

    renderer = sc_compose.Renderer.with_delimiters("[[", "]]")
    assert renderer.render("hello [[ name ]]", {"name": "python"}) == "hello python"
    assert renderer.render_named("inline", "hey [[ name ]]", {"name": "api"}) == "hey api"

    tokens = sc_compose.discover_tokens("{{ name }} {{ report.title }}")
    assert [str(token) for token in tokens] == ["name", "report.title"]
    assert "TEMPLATE_NAME" in sc_compose.BUILTIN_VARIABLE_NAMES


@pytest.mark.parametrize(
    ("factory", "exc_type", "expected_code"),
    [
        (
            lambda root: sc_compose.render_template("{% if true %}", {}),
            sc_compose.ScRenderError,
            None,
        ),
        (
            lambda root: sc_compose.validate_input_value({"bad": [[1]]}),
            sc_compose.ScValidationError,
            sc_compose.DiagnosticCode.ERR_VAL_NESTED_ARRAY_UNSUPPORTED,
        ),
        (
            lambda root: sc_compose.resolve_template_path(
                make_file_request(root, "missing.md.j2")
            ),
            sc_compose.ScResolveError,
            sc_compose.DiagnosticCode.ERR_RESOLVE_NOT_FOUND,
        ),
        (
                lambda root: sc_compose.expand_includes(
                    root / "missing-include.md.j2", root
                ),
                sc_compose.ScIncludeError,
                sc_compose.DiagnosticCode.ERR_INCLUDE_NOT_FOUND,
            ),
        (
            lambda root: sc_compose.input_value_from_yaml("[1"),
            sc_compose.ScConfigError,
            sc_compose.DiagnosticCode.ERR_CONFIG_PARSE,
        ),
    ],
)
def test_exception_surface_exposes_message_and_code(
    tmp_path: Path,
    factory,
    exc_type: type[Exception],
    expected_code: str | None,
) -> None:
    write(tmp_path / "missing-include.md.j2", "@<missing.md>\n")

    with pytest.raises(exc_type) as exc_info:
        factory(tmp_path)

    error = exc_info.value
    assert isinstance(error.message, str)
    assert error.message
    assert error.code == expected_code
