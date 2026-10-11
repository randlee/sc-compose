"""Cross-surface regressions from the Phase T campaign."""
import pytest
import sc_composer_beads as beads


@pytest.mark.parametrize("reference", ["a.b", "", "x" * 33])
@pytest.mark.parametrize("operation", ["attach", "preview_attach"])
def test_fuzz_021_invalid_ref_has_typed_validate_error(tmp_path, reference, operation):
    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(tmp_path, tmp_path / "f.formula.toml.j2", tmp_path / "f.formula.toml", {}, operation=operation, parent="proj-1", ref=reference)
    assert raised.value.code == "BEADS_GRAPH_ID_INVALID"
    assert raised.value.stage == "validate"
    assert raised.value.details == {"field": "ref", "value": reference, "rule": "ref is [A-Za-z0-9_-]{1,32}"}
    assert "[A-Za-z0-9_-]" in str(raised.value)


@pytest.mark.parametrize("endpoint", ["from", "to"])
def test_fuzz_021_invalid_relation_step_has_typed_validate_error(tmp_path, endpoint):
    relation = {"from": "step:build", "to": "bead:proj-1", "type": "blocks"}
    relation[endpoint] = "step:bad.step"
    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(tmp_path, tmp_path / "f.formula.toml.j2", tmp_path / "f.formula.toml", {}, operation="preview_attach", parent="proj-1", ref="valid", relations=[relation])
    assert raised.value.code == "BEADS_GRAPH_ID_INVALID"
    assert raised.value.stage == "validate"
    assert raised.value.details == {"field": "step", "value": "bad.step", "rule": "step is [A-Za-z0-9_]{1,64}; hyphens are forbidden"}


@pytest.mark.parametrize("invalid", ["", " ", "invalid parent", "bad\tparent", "bad\nparent", "bad\rparent"])
@pytest.mark.parametrize("endpoint", ["from", "to"])
@pytest.mark.parametrize("operation", ["attach", "preview_attach"])
def test_invalid_relation_bead_preserves_native_typed_error(tmp_path, invalid, endpoint, operation):
    relation = {"from": "step:build", "to": "bead:proj-1", "type": "blocks"}
    relation[endpoint] = "bead:" + invalid
    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(
            tmp_path, tmp_path / "template.j2", tmp_path / "output.toml", {},
            operation=operation, parent="proj-1", ref="valid", relations=[relation],
        )
    assert raised.value.code == "BEADS_GRAPH_ID_INVALID"
    assert raised.value.stage == "validate"
    assert raised.value.details == {"field": "bead", "value": invalid, "rule": "bead ids are non-empty without whitespace"}
    assert "bead ids are non-empty without whitespace" in str(raised.value)
