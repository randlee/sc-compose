"""Installed-wheel import and typing-marker smoke coverage."""

from importlib.resources import files

import sc_composer_beads as beads


def test_installed_wheel_exposes_the_public_binding_api() -> None:
    for name in (
        "BeadComposeError",
        "BeadComposeReceipt",
        "BeadComposeRequest",
        "BeadOperation",
        "BeadOutcome",
        "BeadStage",
        "BeadStageOutcome",
        "BeadStageReceipt",
        "PourAuthorization",
        "execute",
        "render",
        "validate",
        "preview_pour",
        "pour",
    ):
        assert getattr(beads, name) is not None


def test_wheel_includes_the_typing_marker_and_stub() -> None:
    package_files = files("sc_composer_beads")
    assert package_files.joinpath("py.typed").is_file()
    assert package_files.joinpath("_native.pyi").is_file()
