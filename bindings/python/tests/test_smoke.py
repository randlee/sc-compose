from __future__ import annotations

import textwrap
from pathlib import Path

import pytest

import sc_compose


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(contents), encoding="utf-8", newline="\n")


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
        "ExtractionDiagnostic",
        "ExtractionOccurrence",
        "ExtractionReport",
        "ExtractionSource",
        "Frontmatter",
        "FrontmatterInitResult",
        "InitResult",
        "LoadedTemplateRequest",
        "NamedTemplateAsset",
        "PassConfig",
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
        "VerifyResult",
        "XmlPathSegment",
        "compose",
        "compose_file",
        "discover_all_pass_tokens",
        "discover_tokens",
        "discover_tokens_with_brace_count",
        "expand_includes",
        "extract_variables",
        "frontmatter_init",
        "init_workspace",
        "input_value_from_yaml",
        "parse_template_document",
        "render_all",
        "render_loaded_template",
        "render_template",
        "resolve_profile",
        "resolve_template_path",
        "to_forward_slash",
        "validate",
        "validate_input_value",
        "verify",
    ]

    for name in required:
        assert getattr(sc_compose, name) is not None


def test_extraction_report_preserves_values_provenance_and_filters() -> None:
    template = (
        '<root id="{{ id }}">'
        "<item>{{ first }}</item>"
        "<item>{{ second }}</item>"
        "<name>Hello {{ name }}!</name>"
        "</root>"
    )
    rendered = (
        '<root id="42">'
        "<item>one</item>"
        "<item>two</item>"
        "<name>Hello Ada!</name>"
        "</root>"
    )

    report = sc_compose.extract_variables(template, rendered)

    assert isinstance(report, sc_compose.ExtractionReport)
    assert report.values == {
        "first": "one",
        "id": "42",
        "name": "Ada",
        "second": "two",
    }
    assert report.confidence > 0.75
    assert report.diagnostics == []

    by_variable = {occurrence.variable: occurrence for occurrence in report.occurrences}
    assert by_variable["id"].source.kind == "attribute"
    assert by_variable["id"].source.name == "id"
    assert by_variable["id"].rendered_text == "42"
    assert by_variable["name"].source.kind == "text_node"
    assert by_variable["name"].path[-1].kind == "element"
    assert by_variable["name"].path[-1].name == "name"

    first_item = by_variable["first"].path[-1]
    second_item = by_variable["second"].path[-1]
    assert first_item.ordinal == 0
    assert second_item.ordinal == 1

    included = sc_compose.extract_variables(
        template, rendered, include=["name", "id"]
    )
    assert included.values == {"id": "42", "name": "Ada"}

    excluded = sc_compose.extract_variables(template, rendered, exclude=["id"])
    assert "id" not in excluded.values
    assert {occurrence.variable for occurrence in excluded.occurrences} == {
        "first",
        "name",
        "second",
    }


def test_extraction_fails_closed_for_unsupported_syntax() -> None:
    with pytest.raises(sc_compose.ScConfigError) as caught:
        sc_compose.extract_variables(
            "<root>{% if enabled %}{{ value }}{% endif %}</root>",
            "<root>yes</root>",
        )

    assert caught.value.code == "ERR_EXTRACT_UNSUPPORTED"
    assert caught.value.diagnostic_kind == "unsupported"
    assert caught.value.diagnostic_message
    assert caught.value.recovery_hints


FIXTURE_ROOT = (
    Path(__file__).resolve().parents[3]
    / "crates"
    / "sc-composer"
    / "tests"
    / "fixtures"
    / "reverse-extract"
)


def fixture_pair(name: str) -> tuple[str, str]:
    template = (FIXTURE_ROOT / f"{name}.xml.j2").read_text(encoding="utf-8")
    rendered = (FIXTURE_ROOT / f"{name}.xml").read_text(encoding="utf-8")
    return template, rendered


def test_extraction_matches_shared_rust_fixture_contract() -> None:
    template, rendered = fixture_pair("attributes")
    attributes = sc_compose.extract_variables(template, rendered)
    assert attributes.values == {"id": "42", "name": "Ada"}
    assert [occurrence.variable for occurrence in attributes.occurrences] == [
        "id",
        "name",
    ]
    assert attributes.occurrences[0].source.kind == "attribute"
    assert attributes.occurrences[1].source.kind == "text_node"
    assert attributes.confidence > 0.99

    template, rendered = fixture_pair("repeated-siblings")
    repeated = sc_compose.extract_variables(template, rendered)
    assert repeated.values == {"first": "A", "second": "B"}
    assert repeated.occurrences[1].path[1].kind == "element"
    assert repeated.occurrences[1].path[1].name == "item"
    assert repeated.occurrences[1].path[1].ordinal == 1
    assert repeated.confidence == pytest.approx(0.6)
    assert [diagnostic.code for diagnostic in repeated.diagnostics] == [
        "WARN_EXTRACT_LOW_CONFIDENCE"
    ]

    template, rendered = fixture_pair("entities-whitespace-empty")
    entities = sc_compose.extract_variables(template, rendered)
    assert entities.values == {"empty": "", "value": "A & B"}
    assert entities.confidence > 0.75


def test_extraction_error_categories_preserve_exception_mapping_and_detail() -> None:
    malformed_template, malformed_rendered = fixture_pair("malformed")
    with pytest.raises(sc_compose.ScConfigError) as malformed:
        sc_compose.extract_variables(malformed_template, malformed_rendered)
    assert malformed.value.code == "ERR_EXTRACT_MALFORMED"
    assert malformed.value.diagnostic_kind == "malformed"
    assert malformed.value.recovery_hints

    with pytest.raises(sc_compose.ScConfigError) as ambiguous:
        sc_compose.extract_variables(
            "<x>{{ first }}{{ second }}</x>",
            "<x>AB</x>",
        )
    assert ambiguous.value.code == "ERR_EXTRACT_AMBIGUOUS"
    assert ambiguous.value.diagnostic_kind == "ambiguous"
    assert ambiguous.value.diagnostic_occurrence is None
    assert ambiguous.value.recovery_hints

    with pytest.raises(sc_compose.ScConfigError) as invalid:
        sc_compose.extract_variables("", "<x />")
    assert invalid.value.code == "ERR_EXTRACT_INVALID_REQUEST"
    assert invalid.value.diagnostic_kind is None
    assert invalid.value.recovery_hints


def test_extraction_reports_missing_occurrence_warning() -> None:
    template, rendered = fixture_pair("missing-occurrence")
    report = sc_compose.extract_variables(template, rendered)

    assert report.values == {}
    not_observed = [
        diagnostic
        for diagnostic in report.diagnostics
        if diagnostic.code == "WARN_EXTRACT_NOT_OBSERVED"
    ]
    assert len(not_observed) == 1
    assert not_observed[0].kind == "not_observed"
    assert "not observed" in not_observed[0].message


def test_repr_surface_is_informative(tmp_path: Path) -> None:
    mode = sc_compose.ComposeMode.profile(sc_compose.ProfileKind.AGENT, "reviewer")
    policy = sc_compose.ComposePolicy(
        strict_undeclared_variables=True,
        unknown_variable_policy=sc_compose.UnknownVariablePolicy.ERROR,
        max_include_depth=7,
        allowed_roots=[tmp_path],
    )
    request = sc_compose.ComposeRequest(
        root=tmp_path,
        mode=mode,
        vars_input={"name": "world"},
        guidance_block="use the guide",
        user_prompt="render this",
        policy=policy,
        runtime=sc_compose.RuntimeKind.CLAUDE,
    )

    mode_repr = repr(mode)
    policy_repr = repr(policy)
    request_repr = repr(request)

    assert mode_repr == "ComposeMode.profile(kind='agent', name='reviewer')"
    assert "ComposePolicy(" in policy_repr
    assert "unknown_variable_policy='error'" in policy_repr
    assert "allowed_roots=[" in policy_repr
    assert "resolver_policy=ResolverPolicy(" in policy_repr
    assert "passes=0" in policy_repr
    assert "ComposeRequest(" in request_repr
    assert "mode=ComposeMode.profile(kind='agent', name='reviewer')" in request_repr
    assert "runtime='claude'" in request_repr
    assert "vars_input=1" in request_repr
    assert "guidance_block=True" in request_repr
    assert "user_prompt=True" in request_repr
    assert "policy=ComposePolicy(" in request_repr


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

    recursive_template = (
        "{% for group in groups %}"
        "{{ group.name }}:"
        "{% for row in group.rows %}"
        "[{% for value in row %}{{ value }}{% if not loop.last %},{% endif %}{% endfor %}]"
        "{% endfor %};"
        "{% endfor %}"
    )
    recursive_context = {
        "groups": [
            {"name": "one", "rows": [["a", "b"], ["c"]]},
            {"name": "two", "rows": [[]]},
        ]
    }
    assert (
        sc_compose.render_template(recursive_template, recursive_context)
        == "one:[a,b][c];two:[];"
    )

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
    assert parsed.frontmatter.pass_number == 1
    assert len(parsed.passes) == 1
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

    with pytest.raises(sc_compose.ScRenderError):
        sc_compose.Renderer.with_delimiters("", "]]")


def test_expanded_template_exposes_full_frontmatter_passes(tmp_path: Path) -> None:
    write(
        tmp_path / "stacked.md.j2",
        "---\n"
        "pass: 1\n"
        "metadata:\n"
        "  stage: outer\n"
        "---\n"
        "---\n"
        "pass: 2\n"
        "metadata:\n"
        "  stage: inner\n"
        "---\n"
        "body\n",
    )

    expanded = sc_compose.expand_includes(tmp_path / "stacked.md.j2", tmp_path)

    assert expanded.frontmatters[0][1] is not None
    assert expanded.frontmatters[0][1].metadata["stage"] == "outer"
    assert len(expanded.frontmatter_passes[0][1]) == 2
    assert expanded.frontmatter_passes[0][1][0].metadata["stage"] == "outer"
    assert expanded.frontmatter_passes[0][1][1].metadata["stage"] == "inner"

    tokens = sc_compose.discover_tokens("{{ name }} {{ report.title }}")
    assert [str(token) for token in tokens] == ["name", "report.title"]
    assert "TEMPLATE_NAME" in sc_compose.BUILTIN_VARIABLE_NAMES


def test_multi_pass_bindings_expose_d1_py_surface() -> None:
    parsed = sc_compose.parse_template_document(
        "---\n"
        "pass: 2\n"
        "required_variables:\n"
        "  - team\n"
        "---\n"
        "---\n"
        "metadata:\n"
        "  stage: final\n"
        "---\n"
        "{{ second }} {{{ first }}}\n"
    )

    assert parsed.frontmatter is not None
    assert parsed.frontmatter.pass_number == 2
    assert len(parsed.passes) == 2
    assert parsed.passes[0].pass_number == 2
    assert parsed.passes[1].pass_number == 1
    assert [str(name) for name in parsed.passes[0].required_variables] == ["team"]
    assert parsed.passes[1].metadata["stage"] == "final"

    config = sc_compose.PassConfig(
        0,
        required_variables=["team", sc_compose.VariableName("role")],
        defaults={"region": "west"},
        metadata={"labels": ["fast", "safe"], "rank": 2},
    )
    assert config.pass_number == 1
    assert [str(name) for name in config.required_variables] == ["team", "role"]
    assert config.defaults == {"region": "west"}
    assert config.metadata == {"labels": ["fast", "safe"], "rank": 2}

    triple = sc_compose.discover_tokens_with_brace_count(
        "{{{ outer }}} {{ inner }}", 3
    )
    assert [str(token) for token in triple] == ["outer"]

    per_pass = sc_compose.discover_all_pass_tokens(parsed)
    assert {pass_number: [str(token) for token in tokens] for pass_number, tokens in per_pass.items()} == {
        1: ["second"],
        2: ["first"],
    }


def test_d2_py_render_all_and_policy_passes_round_trip(tmp_path: Path) -> None:
    parsed = sc_compose.parse_template_document(
        "---\n"
        "pass: 2\n"
        "---\n"
        "---\n"
        "pass: 1\n"
        "---\n"
        "{{{ team }}} {{ task }}\n"
    )
    passes = [
        sc_compose.PassConfig(2, defaults={"team": "wyvern"}),
        sc_compose.PassConfig(1, defaults={"task": "test"}),
    ]
    policy = sc_compose.ComposePolicy(passes=passes, allowed_roots=[tmp_path])

    assert [pass_config.pass_number for pass_config in policy.passes] == [2, 1]
    assert "passes=2" in repr(policy)

    rendered = sc_compose.render_all(
        parsed,
        [
            (2, {"team": "wyvern"}),
            (1, {"task": "test"}),
        ],
    )
    assert rendered == "wyvern test"


def test_d2_py_render_all_applies_frontmatter_defaults_for_direct_callers() -> None:
    parsed = sc_compose.parse_template_document(
        "---\n"
        "pass: 2\n"
        "defaults:\n"
        "  team: wyvern\n"
        "---\n"
        "---\n"
        "pass: 1\n"
        "defaults:\n"
        "  task: test\n"
        "---\n"
        "{{{ team }}} {{ task }}\n"
    )

    rendered = sc_compose.render_all(parsed, [(2, {}), (1, {})])

    assert rendered == "wyvern test"


def test_d2_py_render_all_maps_context_shape_failures() -> None:
    parsed = sc_compose.parse_template_document(
        "---\npass: 2\n---\n---\npass: 1\n---\n{{{ team }}} {{ task }}\n"
    )

    with pytest.raises(sc_compose.ScConfigError) as count_error:
        sc_compose.render_all(parsed, [(2, {"team": "wyvern"})])
    assert count_error.value.code == sc_compose.DiagnosticCode.ERR_CONFIG_PARSE

    with pytest.raises(sc_compose.ScConfigError) as pass_error:
        sc_compose.render_all(parsed, [(1, {"team": "wyvern"}), (2, {"task": "test"})])
    assert pass_error.value.code == sc_compose.DiagnosticCode.ERR_CONFIG_PARSE


def test_d2_py_compose_renders_stacked_headers_with_policy_passes(tmp_path: Path) -> None:
    write(
        tmp_path / "stacked.md.j2",
        "---\n"
        "pass: 2\n"
        "---\n"
        "---\n"
        "pass: 1\n"
        "---\n"
        "{{{ team }}} {{ task }}\n",
    )

    request = make_file_request(
        tmp_path,
        "stacked.md.j2",
        policy=sc_compose.ComposePolicy(
            passes=[
                sc_compose.PassConfig(2, defaults={"team": "wyvern"}),
                sc_compose.PassConfig(1, defaults={"task": "test"}),
            ]
        ),
    )

    result = sc_compose.compose(request)
    assert result.rendered_text == "wyvern test"


def test_d2_py_validate_catches_higher_pass_undeclared_tokens(tmp_path: Path) -> None:
    write(
        tmp_path / "stacked.md.j2",
        "---\n"
        "pass: 2\n"
        "---\n"
        "---\n"
        "pass: 1\n"
        "defaults:\n"
        "  task: test\n"
        "---\n"
        "{{{ missing_team }}} {{ task }}\n",
    )

    request = make_file_request(
        tmp_path,
        "stacked.md.j2",
        policy=sc_compose.ComposePolicy(strict_undeclared_variables=True),
    )

    report = sc_compose.validate(request)

    assert not report.ok
    assert any(
        diagnostic.code == sc_compose.DiagnosticCode.ERR_VAL_UNDECLARED_TOKEN
        and "missing_team" in diagnostic.message
        for diagnostic in report.errors
    )


def test_d3_py_python_surface_remains_library_only() -> None:
    assert not hasattr(sc_compose, "parse_pass_inputs")
    assert not hasattr(sc_compose, "filtered_args_for_clap")
    assert not hasattr(sc_compose, "run_template_init")


def test_d4_py_verify_reports_clean_and_builtin_override(tmp_path: Path) -> None:
    write(tmp_path / "verify.md.j2", "Date={{ RENDER_DATE }}\nStamp={{ RENDER_TIMESTAMP }}\n")
    write(tmp_path / "deployed.md", "Date=2026-01-01\nStamp=2026-01-01T00:00:00Z")

    request = make_file_request(
        tmp_path,
        "verify.md.j2",
        vars_input={
            "RENDER_DATE": "2026-01-01",
            "RENDER_TIMESTAMP": "2026-01-01T00:00:00Z",
        },
    )

    result = sc_compose.verify(request, tmp_path / "deployed.md")
    assert isinstance(result, sc_compose.VerifyResult)
    assert result.clean is True
    assert result.diff is None
    assert result.resolved_template_path.endswith("verify.md.j2")
    assert result.deployed_path.endswith("deployed.md")
    assert result.rendered_text == "Date=2026-01-01\nStamp=2026-01-01T00:00:00Z"
    assert result.deployed_text == "Date=2026-01-01\nStamp=2026-01-01T00:00:00Z"
    assert "clean=True" in repr(result)


def test_d4_py_verify_reports_drift_and_diff(tmp_path: Path) -> None:
    write(tmp_path / "verify.md.j2", "hello {{ name }}\n")
    write(tmp_path / "deployed.md", "hello drifted")

    request = make_file_request(tmp_path, "verify.md.j2", vars_input={"name": "world"})

    result = sc_compose.verify(request, tmp_path / "deployed.md")
    assert result.clean is False
    assert result.diff is not None
    assert "--- " in result.diff
    assert "+++ " in result.diff
    assert "-hello drifted" in result.diff
    assert "+hello world" in result.diff


def test_d4_py_verify_missing_deployed_file_maps_existing_error(tmp_path: Path) -> None:
    write(tmp_path / "verify.md.j2", "hello {{ name }}\n")

    request = make_file_request(tmp_path, "verify.md.j2", vars_input={"name": "world"})

    with pytest.raises(sc_compose.ScConfigError) as exc_info:
        sc_compose.verify(request, tmp_path / "missing.md")

    assert exc_info.value.code == sc_compose.DiagnosticCode.ERR_RESOLVE_NOT_FOUND


def test_headerless_templates_keep_phase_c_behavior() -> None:
    parsed = sc_compose.parse_template_document("hello")

    assert parsed.frontmatter is None
    assert parsed.passes == []


@pytest.mark.parametrize(
    ("factory", "exc_type", "expected_code"),
    [
        (
            lambda root: sc_compose.render_template("{% if true %}", {}),
            sc_compose.ScRenderError,
            None,
        ),
        (
            lambda root: sc_compose.resolve_template_path(
                make_file_request(root, "missing.md.j2")
            ),
            sc_compose.ScResolveError,
            sc_compose.DiagnosticCode.ERR_RESOLVE_NOT_FOUND,
        ),
        (
            lambda root: sc_compose.expand_includes(root / "missing-include.md.j2", root),
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
