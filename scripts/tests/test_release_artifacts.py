from __future__ import annotations

import json
import subprocess
import sys
import tomllib
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


def pypi_publish_workflow_text() -> str:
    return (repo_root() / ".github" / "workflows" / "pypi-publish.yml").read_text(encoding="utf-8")


def homebrew_publish_workflow_text() -> str:
    return (repo_root() / ".github" / "workflows" / "homebrew-publish.yml").read_text(encoding="utf-8")


def winget_publish_workflow_text() -> str:
    return (repo_root() / ".github" / "workflows" / "winget-publish.yml").read_text(encoding="utf-8")


def scoop_publish_workflow_text() -> str:
    return (repo_root() / ".github" / "workflows" / "scoop-publish.yml").read_text(encoding="utf-8")


def scoop_manifest_template_text() -> str:
    return (repo_root() / "release" / "scoop" / "sc-compose.json.j2").read_text(encoding="utf-8")


def published_release_guard_text() -> str:
    return (
        repo_root() / ".github" / "actions" / "verify-published-release" / "action.yml"
    ).read_text(encoding="utf-8")


def release_manifest() -> dict:
    return tomllib.loads(
        (repo_root() / "release" / "publish-artifacts.toml").read_text(encoding="utf-8")
    )


def python_pyproject_text() -> str:
    return (repo_root() / "bindings" / "python" / "pyproject.toml").read_text(encoding="utf-8")


def python_cargo_toml_text() -> str:
    return (repo_root() / "bindings" / "python" / "Cargo.toml").read_text(encoding="utf-8")


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


def test_release_manifest_publishes_sc_sha_before_its_consumers() -> None:
    """Keep the registry dependency order explicit and regression-tested."""
    manifest = release_manifest()

    assert [
        (entry["artifact"], entry["package"], entry["publish_order"], entry["wait_after_publish_seconds"])
        for entry in manifest["crates"]
    ] == [
        ("sc-sha", "sc-sha", 1, 30),
        ("sc-composer", "sc-composer", 2, 30),
        ("sc-compose", "sc-compose", 3, 0),
    ]
    assert {
        entry["name"]: entry["source"] for entry in manifest["python_distributions"]
    } == {
        "sc-sha": "bindings/sc-sha-python",
        "sc-compose": "bindings/python",
    }


def test_release_workflow_enforces_python_release_invariants() -> None:
    text = release_workflow_text()
    pypi_text = pypi_publish_workflow_text()
    action_text = (
        repo_root() / ".github" / "actions" / "setup-python-release-build" / "action.yml"
    ).read_text(encoding="utf-8")

    assert (
        "needs: [gate-and-tag, build, publish, build-python-wheels, build-python-sdist, build-sc-sha-python-wheels, build-sc-sha-python-sdist]"
        in text
    )
    assert "publish-testpypi:" in text
    assert "needs.gate-and-tag.outputs.release_target == 'testpypi'" in text
    assert "publish-pypi:" not in text
    assert "name: python-sdist" in text
    assert "default: testpypi" in text
    assert "type: choice" in text
    assert "expected exactly two sdists (sc-compose and sc-sha)" in text
    assert "TEST_PYPI_API_TOKEN" in text
    assert "--repository testpypi" in text
    assert (
        "maturin upload --repository testpypi --non-interactive --skip-existing "
        "dist/*.whl dist/*.tar.gz"
    ) in text
    assert "for pattern in *.tar.gz *.zip *.whl; do" in text
    assert "uses: ./.github/actions/setup-python-release-build" in text
    assert "update-homebrew:" not in text
    assert "publish-winget:" not in text
    assert "verify-python-version" in action_text
    assert "sync-python-version" in action_text
    assert "release_ref" in action_text
    assert "pyproject" in action_text

    assert "name: Publish PyPI" in pypi_text
    assert "release_tag: ${{ inputs.tag }}" in pypi_text
    assert "gh release download" in pypi_text
    assert "published GitHub Release Python assets mismatch" in pypi_text
    assert "verified six wheels and two sdists from the published GitHub Release" in pypi_text
    assert "maturin build" not in pypi_text
    assert "maturin sdist" not in pypi_text
    assert "MATURIN_PYPI_TOKEN: ${{ secrets.PYPI_API_TOKEN }}" in pypi_text
    assert "maturin upload --non-interactive --skip-existing dist/*.whl dist/*.tar.gz" in pypi_text
    assert (
        "maturin upload --repository testpypi --non-interactive --skip-existing "
        "dist/*.whl dist/*.tar.gz"
    ) in pypi_text


def test_channel_recovery_workflows_require_a_published_release() -> None:
    guard_text = published_release_guard_text()
    pypi_text = pypi_publish_workflow_text()
    homebrew_text = homebrew_publish_workflow_text()
    winget_text = winget_publish_workflow_text()
    scoop_text = scoop_publish_workflow_text()

    assert "No published GitHub Release found" in guard_text
    assert "is still a draft" in guard_text
    assert "^v[0-9]+\\.[0-9]+\\.[0-9]+$" in guard_text
    assert "REQUIRED_ASSET_PATTERNS" in guard_text

    for workflow_text in (pypi_text, homebrew_text, winget_text, scoop_text):
        assert "workflow_dispatch:" in workflow_text
        assert "uses: ./.github/actions/verify-published-release" in workflow_text
        assert "release_tag: ${{ inputs.tag }}" in workflow_text
        assert "gate-and-tag" not in workflow_text

    assert "WINGET_GITHUB_TOKEN" in winget_text
    assert "x86_64-pc-windows-msvc" in winget_text
    assert "HOMEBREW_TAP_TOKEN" in homebrew_text
    assert "ref: ${{ inputs.tag }}" in homebrew_text
    assert "aarch64-apple-darwin" in homebrew_text
    assert "x86_64-apple-darwin" in homebrew_text
    assert "x86_64-unknown-linux-gnu" in homebrew_text
    assert "SCOOP_BUCKET_TOKEN" in scoop_text
    assert "randlee/scoop-bucket" in scoop_text
    assert "x86_64-pc-windows-msvc" in scoop_text
    assert "json.tool scoop-bucket/sc-compose.json" in scoop_text
    assert "sc-compose/release/scoop/sc-compose.json.j2" in scoop_text


def test_release_root_keeps_crates_io_in_the_authoritative_chain() -> None:
    text = release_workflow_text()

    assert "  publish:\n    if: ${{ needs.gate-and-tag.outputs.release_target == 'production' }}" in text
    assert "needs: gate-and-tag" in text
    assert "  release:\n    if: ${{ needs.gate-and-tag.outputs.release_target == 'production' }}" in text
    assert "needs: [gate-and-tag, build, publish," in text


def test_scoop_manifest_template_renders_a_valid_windows_zip_manifest() -> None:
    rendered = (
        scoop_manifest_template_text()
        .replace("{{ version }}", "1.4.1")
        .replace(
            "{{ windows_url }}",
            "https://github.com/randlee/sc-compose/releases/download/v1.4.1/"
            "sc-compose_1.4.1_x86_64-pc-windows-msvc.zip",
        )
        .replace("{{ windows_sha256 }}", "a" * 64)
    )

    manifest = json.loads(rendered)

    assert manifest["version"] == "1.4.1"
    assert manifest["architecture"]["64bit"]["extract_dir"] == (
        "sc-compose_1.4.1_x86_64-pc-windows-msvc"
    )
    assert manifest["architecture"]["64bit"]["bin"] == "bin/sc-compose.exe"
    assert manifest["architecture"]["64bit"]["hash"] == "a" * 64


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


def test_release_workflow_checks_out_repo_before_local_python_setup_action() -> None:
    text = release_workflow_text()

    wheels_job = """  build-python-wheels:
    needs: gate-and-tag
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ needs.gate-and-tag.outputs.release_ref }}
      - uses: ./.github/actions/setup-python-release-build"""
    sdist_job = """  build-python-sdist:
    needs: gate-and-tag
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ needs.gate-and-tag.outputs.release_ref }}
      - uses: ./.github/actions/setup-python-release-build"""

    assert wheels_job in text
    assert sdist_job in text
    assert "build-sc-sha-python-wheels:" in text
    assert "build-sc-sha-python-sdist:" in text
    assert "bindings/sc-sha-python/Cargo.toml" in text


def test_python_package_metadata_uses_local_readme_for_sdist() -> None:
    pyproject_text = python_pyproject_text()
    cargo_toml_text = python_cargo_toml_text()

    assert 'readme = "README.md"' in pyproject_text
    assert 'readme = "README.md"' in cargo_toml_text
    assert "../../README.md" not in pyproject_text
    assert "../../README.md" not in cargo_toml_text


def write_readme_fixture(tmp_path: Path, *, dependency_version: str, status_version: str, stability_minor: str) -> tuple[Path, Path]:
    workspace = tmp_path / "Cargo.toml"
    workspace.write_text(
        "\n".join(["[workspace.package]", 'version = "1.2.0"', ""]),
        encoding="utf-8",
    )
    readme = tmp_path / "README.md"
    readme.write_text(
        "\n".join(
            [
                "## Library usage",
                "",
                "```toml",
                "[dependencies]",
                f'sc-composer = "{dependency_version}"',
                "```",
                "",
                "## Status",
                "",
                "| | |",
                "|-|-|",
                f"| Version | {status_version} |",
                "| MSRV | Rust 1.94.1 |",
                f"| Stability | stable {stability_minor} release line |",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return workspace, readme


def run_sync_readme_version(workspace: Path, readme: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "scripts/release_artifacts.py",
            "sync-readme-version",
            "--workspace-toml",
            str(workspace),
            "--readme",
            str(readme),
        ],
        cwd=Path(__file__).resolve().parents[2],
        text=True,
        capture_output=True,
        check=False,
    )


def run_verify_readme_version(workspace: Path, readme: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "scripts/release_artifacts.py",
            "verify-readme-version",
            "--workspace-toml",
            str(workspace),
            "--readme",
            str(readme),
        ],
        cwd=Path(__file__).resolve().parents[2],
        text=True,
        capture_output=True,
        check=False,
    )


def write_version_lockstep_fixture(
    tmp_path: Path,
    *,
    python_version: str = "1.4.0",
    dependency_version: str = "1.4.0",
) -> Path:
    workspace = tmp_path / "Cargo.toml"
    workspace.write_text(
        "[workspace.package]\nversion = \"1.4.0\"\n",
        encoding="utf-8",
    )
    for relative_path in (
        "crates/sc-sha/Cargo.toml",
        "crates/sc-composer/Cargo.toml",
        "crates/sc-compose/Cargo.toml",
        "bindings/python/Cargo.toml",
        "bindings/sc-sha-python/Cargo.toml",
    ):
        path = tmp_path / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        dependencies = ""
        if relative_path == "crates/sc-compose/Cargo.toml":
            dependencies = f'\n[dependencies]\nsc-composer = {{ version = "{dependency_version}" }}\n'
        elif relative_path == "bindings/python/Cargo.toml":
            dependencies = f'\n[dependencies]\nsc-composer = {{ version = "{dependency_version}" }}\n'
        elif relative_path == "bindings/sc-sha-python/Cargo.toml":
            dependencies = f'\n[dependencies]\nsc-sha = {{ version = "{dependency_version}" }}\n'
        path.write_text(
            "[package]\nname = \"fixture\"\nversion.workspace = true\n" + dependencies,
            encoding="utf-8",
        )
    for relative_path in (
        "bindings/python/pyproject.toml",
        "bindings/sc-sha-python/pyproject.toml",
    ):
        path = tmp_path / relative_path
        path.write_text(
            f'[project]\nname = "fixture"\nversion = "{python_version}"\n',
            encoding="utf-8",
        )
    return workspace


def run_verify_version_lockstep(workspace: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "scripts/release_artifacts.py",
            "verify-version-lockstep",
            "--workspace-toml",
            str(workspace),
        ],
        cwd=Path(__file__).resolve().parents[2],
        text=True,
        capture_output=True,
        check=False,
    )


def test_verify_readme_version_passes_when_readme_matches_workspace(tmp_path: Path) -> None:
    workspace, readme = write_readme_fixture(
        tmp_path, dependency_version="1.2.0", status_version="1.2.0", stability_minor="1.2"
    )

    result = run_verify_readme_version(workspace, readme)

    assert result.returncode == 0, result.stderr
    assert "readme version verification passed" in result.stdout


def test_verify_readme_version_rejects_stale_dependency_example(tmp_path: Path) -> None:
    workspace, readme = write_readme_fixture(
        tmp_path, dependency_version="1.1.0", status_version="1.2.0", stability_minor="1.2"
    )

    result = run_verify_readme_version(workspace, readme)

    assert result.returncode != 0
    assert "sc-composer dependency example" in result.stderr


def test_verify_readme_version_rejects_stale_status_table(tmp_path: Path) -> None:
    workspace, readme = write_readme_fixture(
        tmp_path, dependency_version="1.2.0", status_version="1.1.0", stability_minor="1.1"
    )

    result = run_verify_readme_version(workspace, readme)

    assert result.returncode != 0
    assert "Status table Version row" in result.stderr
    assert "Status table Stability row" in result.stderr


def test_sync_readme_version_rewrites_stale_references(tmp_path: Path) -> None:
    workspace, readme = write_readme_fixture(
        tmp_path, dependency_version="1.1.0", status_version="1.1.0", stability_minor="1.1"
    )

    sync_result = run_sync_readme_version(workspace, readme)

    assert sync_result.returncode == 0, sync_result.stderr
    assert "synced 3 readme version reference(s) to 1.2.0" in sync_result.stdout

    verify_result = run_verify_readme_version(workspace, readme)
    assert verify_result.returncode == 0, verify_result.stderr


def test_verify_version_lockstep_accepts_all_release_version_sources(tmp_path: Path) -> None:
    result = run_verify_version_lockstep(write_version_lockstep_fixture(tmp_path))

    assert result.returncode == 0, result.stderr
    assert "version lockstep verification passed" in result.stdout


def test_verify_version_lockstep_names_the_drifting_dependency_field(tmp_path: Path) -> None:
    result = run_verify_version_lockstep(
        write_version_lockstep_fixture(tmp_path, dependency_version="1.3.1")
    )

    assert result.returncode != 0
    assert "crates/sc-compose/Cargo.toml" in result.stderr
    assert "[dependencies].sc-composer.version mismatch" in result.stderr


def test_verify_version_lockstep_rejects_python_package_drift(tmp_path: Path) -> None:
    result = run_verify_version_lockstep(
        write_version_lockstep_fixture(tmp_path, python_version="1.3.1")
    )

    assert result.returncode != 0
    assert "bindings/python/pyproject.toml" in result.stderr
    assert "[project].version mismatch" in result.stderr
