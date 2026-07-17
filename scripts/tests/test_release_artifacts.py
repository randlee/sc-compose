from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def write_repo_fixture(tmp_path: Path, *, manifest_wheels: list[str], workflow_oses: list[str]) -> tuple[Path, Path, Path]:
    workspace = tmp_path / "Cargo.toml"
    workspace.write_text(
        "\n".join(
            [
                "[workspace]",
                'members = ["crates/sc-composer", "crates/sc-compose"]',
                "",
                "[workspace.package]",
                'version = "1.1.0"',
                "",
            ]
        ),
        encoding="utf-8",
    )

    for crate_name in ("sc-composer", "sc-compose"):
        crate_dir = tmp_path / "crates" / crate_name
        crate_dir.mkdir(parents=True)
        (crate_dir / "Cargo.toml").write_text(
            "\n".join(
                [
                    "[package]",
                    f'name = "{crate_name}"',
                    'version = "1.1.0"',
                    "",
                ]
            ),
            encoding="utf-8",
        )

    bindings_dir = tmp_path / "bindings" / "python" / "python" / "sc_compose"
    bindings_dir.mkdir(parents=True)
    (bindings_dir / "__init__.py").write_text("", encoding="utf-8")
    (tmp_path / "bindings" / "python" / "pyproject.toml").write_text(
        "\n".join(
            [
                "[project]",
                'name = "sc-compose"',
                'version = "1.1.0"',
                "",
            ]
        ),
        encoding="utf-8",
    )

    manifest = tmp_path / "release" / "publish-artifacts.toml"
    manifest.parent.mkdir(parents=True)
    wheels = ", ".join(f'"{entry}"' for entry in manifest_wheels)
    manifest.write_text(
        "\n".join(
            [
                "schema_version = 1",
                "",
                "[[crates]]",
                'artifact = "sc-composer"',
                'package = "sc-composer"',
                'cargo_toml = "crates/sc-composer/Cargo.toml"',
                "publish_order = 1",
                "wait_after_publish_seconds = 0",
                "",
                "[[crates]]",
                'artifact = "sc-compose"',
                'package = "sc-compose"',
                'cargo_toml = "crates/sc-compose/Cargo.toml"',
                "publish_order = 2",
                "wait_after_publish_seconds = 0",
                "",
                "[[python_packages]]",
                'artifact = "sc-compose-python"',
                'package = "sc-compose"',
                'manifest = "bindings/python/pyproject.toml"',
                'module = "sc_compose"',
                'publish = "pypi"',
                "",
                "[[python_distributions]]",
                'name = "sc-compose"',
                'source = "bindings/python"',
                "sdist = true",
                f"wheels = [{wheels}]",
                "",
            ]
        ),
        encoding="utf-8",
    )

    workflow = tmp_path / ".github" / "workflows" / "release.yml"
    workflow.parent.mkdir(parents=True)
    oses = ", ".join(workflow_oses)
    workflow.write_text(
        "\n".join(
            [
                "jobs:",
                "  build-python-wheels:",
                "    strategy:",
                "      matrix:",
                f"        os: [{oses}]",
                "  build-python-sdist:",
                "    runs-on: ubuntu-latest",
                "",
            ]
        ),
        encoding="utf-8",
    )

    return workspace, manifest, workflow


def run_validate_manifest(tmp_path: Path, *, manifest_wheels: list[str], workflow_oses: list[str]) -> subprocess.CompletedProcess[str]:
    workspace, manifest, workflow = write_repo_fixture(
        tmp_path,
        manifest_wheels=manifest_wheels,
        workflow_oses=workflow_oses,
    )
    return subprocess.run(
        [
            sys.executable,
            "scripts/release_artifacts.py",
            "validate-manifest",
            "--manifest",
            str(manifest),
            "--workspace-toml",
            str(workspace),
            "--release-workflow",
            str(workflow),
        ],
        cwd=Path(__file__).resolve().parents[2],
        text=True,
        capture_output=True,
        check=False,
    )


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def release_workflow_text() -> str:
    return (repo_root() / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")


def test_validate_manifest_accepts_matching_python_release_shape(tmp_path: Path) -> None:
    result = run_validate_manifest(
        tmp_path,
        manifest_wheels=["ubuntu-latest", "macos-latest", "windows-latest"],
        workflow_oses=["ubuntu-latest", "macos-latest", "windows-latest"],
    )

    assert result.returncode == 0, result.stderr
    assert "manifest validation passed" in result.stdout


def test_validate_manifest_rejects_wheel_matrix_drift(tmp_path: Path) -> None:
    result = run_validate_manifest(
        tmp_path,
        manifest_wheels=["ubuntu-latest", "macos-latest", "windows-latest"],
        workflow_oses=["ubuntu-latest", "windows-latest"],
    )

    assert result.returncode != 0
    assert "wheels mismatch" in result.stderr


def test_release_workflow_enforces_python_release_invariants() -> None:
    text = release_workflow_text()
    action_text = (
        repo_root() / ".github" / "actions" / "setup-python-release-build" / "action.yml"
    ).read_text(encoding="utf-8")

    assert (
        "needs: [gate-and-tag, build, publish, build-python-wheels, build-python-sdist, publish-pypi]"
        in text
    )
    assert "name: python-sdist" in text
    assert "default: testpypi" in text
    assert "type: choice" in text
    assert "release_target == 'production' && 'pypi' || 'testpypi'" in text
    assert "pattern: python-wheels-*" in text
    assert "expected exactly one sdist" in text
    assert "TEST_PYPI_API_TOKEN" in text
    assert "--repository testpypi" in text
    assert "maturin upload --non-interactive dist/*.whl dist/*.tar.gz" in text
    assert "if: ${{ needs.gate-and-tag.outputs.release_target == 'production' }}" in text
    assert "for pattern in *.tar.gz *.zip *.whl; do" in text
    assert "uses: ./.github/actions/setup-python-release-build" in text
    assert "verify-python-version" in action_text
    assert "sync-python-version" in action_text
    assert "release_ref" in action_text


def test_release_workflow_collects_wheels_without_redundant_zip_sweep() -> None:
    text = release_workflow_text()

    assert (
        "find artifacts -type f \\( -name '*.tar.gz' -o -name '*.zip' \\) -exec mv {} release/ \\;"
        in text
    )
    assert "find artifacts -type f -name '*.whl' -exec mv {} release/ \\;" in text
    assert "find artifacts -type f \\( -name '*.zip' -o -name '*.whl' \\)" not in text


def test_release_workflow_rehearsal_mode_avoids_production_side_effects() -> None:
    text = release_workflow_text()

    assert 'echo "Rehearsal mode: validating release tag ${tag} locally only; not pushing any tag to origin"' in text
    assert "echo \"release_ref=$main_sha\" >> \"$GITHUB_OUTPUT\"" in text
    assert "if: ${{ needs.gate-and-tag.outputs.release_target == 'production' }}" in text
