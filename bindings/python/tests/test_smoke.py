from __future__ import annotations

import textwrap

import pytest

import sc_compose


def test_import_surface_exposes_c1_api() -> None:
    assert sc_compose.ComposeMode is not None
    assert sc_compose.ComposePolicy is not None
    assert sc_compose.ComposeRequest is not None
    assert sc_compose.ComposeResult is not None
    assert sc_compose.ScComposeError is not None
    assert sc_compose.compose_file is not None


def test_compose_file_renders_from_installed_wheel(tmp_path) -> None:
    template = tmp_path / "template.md.j2"
    template.write_text(
        textwrap.dedent(
            """\
            ---
            required_variables:
              - name
            ---
            hello {{ name }}
            """
        ),
        encoding="utf-8",
    )

    request = sc_compose.ComposeRequest(
        root=tmp_path,
        mode=sc_compose.ComposeMode.file("template.md.j2"),
        vars_input={"name": "world"},
        policy=sc_compose.ComposePolicy(),
    )

    result = sc_compose.compose_file(request)

    assert isinstance(result, sc_compose.ComposeResult)
    assert result.rendered_text == "hello world"
    assert result.resolved_files


def test_compose_file_raises_sc_compose_error_for_invalid_input(tmp_path) -> None:
    template = tmp_path / "template.md.j2"
    template.write_text(
        textwrap.dedent(
            """\
            ---
            required_variables:
              - name
            ---
            hello {{ name }}
            """
        ),
        encoding="utf-8",
    )

    request = sc_compose.ComposeRequest(
        root=tmp_path,
        mode=sc_compose.ComposeMode.file("template.md.j2"),
        vars_input={},
    )

    with pytest.raises(sc_compose.ScComposeError, match="missing required variable: name"):
        sc_compose.compose_file(request)
