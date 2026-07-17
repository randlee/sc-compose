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
