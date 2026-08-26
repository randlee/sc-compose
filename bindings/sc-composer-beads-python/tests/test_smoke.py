"""Installed-wheel import and typing-marker smoke coverage."""

import os
import tarfile
from importlib.resources import files
from pathlib import Path

import pytest

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


def test_sdist_includes_the_typing_marker_and_stub() -> None:
    artifact_dir = os.environ.get("SC_COMPOSER_BEADS_ARTIFACT_DIR")
    if artifact_dir is None:
        pytest.skip("the sdist artifact directory is configured by the wheel CI job")

    archives = sorted(Path(artifact_dir).glob("sc_composer_beads-*.tar.gz"))
    assert len(archives) == 1
    with tarfile.open(archives[0], "r:gz") as archive:
        names = set(archive.getnames())

    package_root = "bindings/sc-composer-beads-python/python/sc_composer_beads"
    assert any(name.endswith(f"{package_root}/py.typed") for name in names)
    assert any(name.endswith(f"{package_root}/_native.pyi") for name in names)
