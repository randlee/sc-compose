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
    assert raised.value.details == {"field": "ref", "value": reference}
    assert "[A-Za-z0-9_-]" in str(raised.value)


@pytest.mark.parametrize("endpoint", ["from", "to"])
def test_fuzz_021_invalid_relation_step_has_typed_validate_error(tmp_path, endpoint):
    relation = {"from": "step:build", "to": "bead:proj-1", "type": "blocks"}
    relation[endpoint] = "step:bad.step"
    with pytest.raises(beads.BeadComposeError) as raised:
        beads.BeadComposeRequest(tmp_path, tmp_path / "f.formula.toml.j2", tmp_path / "f.formula.toml", {}, operation="preview_attach", parent="proj-1", ref="valid", relations=[relation])
    assert raised.value.code == "BEADS_GRAPH_ID_INVALID"
    assert raised.value.stage == "validate"
    assert raised.value.details == {"field": "step", "value": "bad.step"}
