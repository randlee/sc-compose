from __future__ import annotations

import io
import subprocess
import sys
import tarfile
import tomllib
import zipfile
from pathlib import Path


def write_repo_fixture(tmp_path: Path, *, manifest_wheels: list[str]) -> tuple[Path, Path]:
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
    (tmp_path / "bindings" / "python" / "Cargo.toml").write_text(
        "[package]\nname = \"sc-compose-python\"\nversion = \"1.1.0\"\n",
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
                "[project]",
                'name = "fixture"',
                'archive_prefix = "fixture"',
                'description = "Fixture release"',
                'homepage = "https://example.invalid/fixture"',
                'license = "MIT"',
                'readme_dependency_crate = "sc-composer"',
                "",
                "[[release_targets]]",
                'target = "x86_64-unknown-linux-gnu"',
                'os = "ubuntu-latest"',
                'archive = "tar.gz"',
                "",
                "[[release_binaries]]",
                'name = "fixture"',
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
                "[channels.pypi]",
                'test_repository = "testpypi"',
                'production_repository = "pypi"',
                "",
                "[channels.homebrew]",
                'tap_repository = "example/homebrew-tap"',
                'formula_path = "Formula/fixture.rb"',
                'formula_template = "release/homebrew/formula.rb.j2"',
                'formula_class = "Fixture"',
                'binary = "fixture"',
                'test_command = "--help"',
                'test_output = "fixture"',
                'renderer_target = "x86_64-unknown-linux-gnu"',
                "",
                "[[channels.homebrew.assets]]",
                'key = "linux"',
                'target = "x86_64-unknown-linux-gnu"',
                "",
                "[channels.winget]",
                'identifier = "example.fixture"',
                'installer_target = "x86_64-unknown-linux-gnu"',
                "",
                "[channels.scoop]",
                'bucket_repository = "example/scoop-bucket"',
                'manifest_path = "fixture.json"',
                'manifest_template = "release/scoop/manifest.json.j2"',
                'installer_target = "x86_64-unknown-linux-gnu"',
                'binary = "bin/fixture"',
                'renderer_target = "x86_64-unknown-linux-gnu"',
                "",
            ]
        ),
        encoding="utf-8",
    )

    return workspace, manifest


def run_validate_manifest(tmp_path: Path, *, manifest_wheels: list[str]) -> subprocess.CompletedProcess[str]:
    workspace, manifest = write_repo_fixture(
        tmp_path,
        manifest_wheels=manifest_wheels,
    )
    return subprocess.run(
        [
            sys.executable,
            str(repo_root() / "scripts" / "release_artifacts.py"),
            "validate-manifest",
            "--manifest",
            str(manifest),
            "--workspace-toml",
            str(workspace),
        ],
        cwd=tmp_path,
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


def release_preflight_workflow_text() -> str:
    return (repo_root() / ".github" / "workflows" / "release-preflight.yml").read_text(encoding="utf-8")


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
    )

    assert result.returncode == 0, result.stderr
    assert "manifest validation passed" in result.stdout


def test_validate_manifest_rejects_unknown_channel_target(tmp_path: Path) -> None:
    workspace, manifest = write_repo_fixture(tmp_path, manifest_wheels=["ubuntu-latest"])
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'installer_target = "x86_64-unknown-linux-gnu"',
            'installer_target = "unknown-target"',
            1,
        ),
        encoding="utf-8",
    )
    result = subprocess.run(
        [
            sys.executable,
            str(repo_root() / "scripts" / "release_artifacts.py"),
            "validate-manifest",
            "--manifest",
            str(manifest),
            "--workspace-toml",
            str(workspace),
        ],
        cwd=tmp_path,
        text=True,
        capture_output=True,
        check=False,
    )
    assert result.returncode != 0
    assert "references unknown release target" in result.stderr


def test_validate_manifest_rejects_unknown_renderer_target(tmp_path: Path) -> None:
    workspace, manifest = write_repo_fixture(tmp_path, manifest_wheels=["ubuntu-latest"])
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'renderer_target = "x86_64-unknown-linux-gnu"',
            'renderer_target = "unknown-renderer"',
            1,
        ),
        encoding="utf-8",
    )
    result = subprocess.run(
        [
            sys.executable,
            str(repo_root() / "scripts" / "release_artifacts.py"),
            "validate-manifest",
            "--manifest",
            str(manifest),
            "--workspace-toml",
            str(workspace),
        ],
        cwd=tmp_path,
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode != 0
    assert "renderer_target references unknown release target" in result.stderr


def test_verify_python_release_assets_accepts_manifest_declared_wheels_and_sdist(tmp_path: Path) -> None:
    _, manifest = write_repo_fixture(tmp_path, manifest_wheels=["ubuntu-latest", "windows-latest"])
    assets = tmp_path / "assets"
    assets.mkdir()
    for suffix in ("linux", "windows"):
        with zipfile.ZipFile(assets / f"fixture-{suffix}.whl", "w") as wheel:
            wheel.writestr("fixture-1.1.0.dist-info/METADATA", "Name: sc-compose\nVersion: 1.1.0\n")
    with tarfile.open(assets / "fixture-1.1.0.tar.gz", "w:gz") as sdist:
        metadata = b"Name: sc-compose\nVersion: 1.1.0\n"
        info = tarfile.TarInfo("fixture-1.1.0/PKG-INFO")
        info.size = len(metadata)
        sdist.addfile(info, io.BytesIO(metadata))

    result = subprocess.run(
        [
            sys.executable,
            str(repo_root() / "scripts" / "release_artifacts.py"),
            "verify-python-release-assets",
            "--manifest",
            str(manifest),
            "--asset-dir",
            str(assets),
        ],
        cwd=tmp_path,
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    assert "'sc-compose': {'wheel': 2, 'sdist': 1}" in result.stdout


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
    assert manifest["channels"]["homebrew"]["renderer_target"] == "x86_64-unknown-linux-gnu"
    assert manifest["channels"]["scoop"]["renderer_target"] == "x86_64-unknown-linux-gnu"


def test_release_workflow_enforces_python_release_invariants() -> None:
    text = release_workflow_text()
    pypi_text = pypi_publish_workflow_text()
    action_text = (
        repo_root() / ".github" / "actions" / "setup-python-release-build" / "action.yml"
    ).read_text(encoding="utf-8")

    assert "release-plan:" in text
    assert "release-target-matrix" in text
    assert "python-wheel-matrix" in text
    assert "python-sdist-matrix" in text
    assert "matrix: ${{ fromJSON(needs.release-plan.outputs.python_wheel_matrix) }}" in text
    assert "publish-testpypi:" in text
    assert "needs.gate-and-tag.outputs.release_target == 'testpypi'" in text
    assert "publish-pypi:" not in text
    assert "name: python-sdist-${{ matrix.artifact }}" in text
    assert "TEST_PYPI_API_TOKEN" in text
    assert "MATURIN_PYPI_TOKEN: ${{ secrets.TEST_PYPI_API_TOKEN }}" in text
    assert "secrets.TEST_PYPI_TOKEN" not in text
    assert "--repository testpypi" in text
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
    assert "environment: ${{ inputs.target == 'production' && 'pypi' || 'testpypi' }}" in pypi_text
    assert "gh release download" in pypi_text
    assert "verify-python-release-assets" in pypi_text
    assert "maturin build" not in pypi_text
    assert "maturin sdist" not in pypi_text
    assert "name: Publish manifest-declared wheels and sdists to TestPyPI" in pypi_text
    assert "if: ${{ inputs.target == 'testpypi' }}" in pypi_text
    assert "MATURIN_PYPI_TOKEN: ${{ secrets.TEST_PYPI_API_TOKEN }}" in pypi_text
    assert "name: Publish manifest-declared wheels and sdists to PyPI" in pypi_text
    assert "if: ${{ inputs.target == 'production' }}" in pypi_text
    assert "MATURIN_PYPI_TOKEN: ${{ secrets.PYPI_API_TOKEN }}" in pypi_text
    assert "inputs.target == 'testpypi' && secrets." not in pypi_text
    assert "secrets.TEST_PYPI_TOKEN" not in pypi_text
    assert "secrets.PYPI_TOKEN" not in pypi_text
    assert "maturin upload --repository \"${PYPI_REPOSITORY}\" --non-interactive --skip-existing dist/*.whl dist/*.tar.gz" in pypi_text


def test_release_preflight_requires_each_standardized_secret() -> None:
    text = release_preflight_workflow_text()

    assert "Missing required GitHub Actions release secret(s):" in text
    for secret_name in (
        "CARGO_REGISTRY_TOKEN",
        "HOMEBREW_TAP_TOKEN",
        "PYPI_API_TOKEN",
        "SCOOP_BUCKET_TOKEN",
        "TEST_PYPI_API_TOKEN",
        "WINGET_GITHUB_TOKEN",
    ):
        assert secret_name in text
    assert "All standardized release secrets are available." in text
    assert "actions: read" in text
    assert "environment:" not in text
    assert "Verify Python environment secret metadata" in text
    assert "environments/${environment_name}/secrets" in text
    assert "Required Python environment secrets are present." in text
    assert "Verify GitHub channel token liveness" in text
    assert "https://api.github.com/user" in text
    assert "rotate or replace it" in text


def test_active_release_docs_use_the_standard_python_secret_names() -> None:
    for relative_path in (
        "RELEASING.md",
        "docs/publishing.md",
        "docs/publishing-agent.md",
    ):
        text = (repo_root() / relative_path).read_text(encoding="utf-8")
        assert "PYPI_API_TOKEN" in text
        assert "TEST_PYPI_API_TOKEN" in text
        assert "PYPI_TOKEN" not in text
        assert "TEST_PYPI_TOKEN" not in text


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
    assert "channel-config" in winget_text
    assert "HOMEBREW_TAP_TOKEN" in homebrew_text
    assert "ref: ${{ inputs.tag }}" in homebrew_text
    assert "channel-config" in homebrew_text
    assert "SCOOP_BUCKET_TOKEN" in scoop_text
    assert "channel-config" in scoop_text
    assert "Render Scoop manifest with sc-compose" in scoop_text
    assert 'MANIFEST_TEMPLATE: ${{ fromJSON(needs.verify-release.outputs.channel_config).channel.manifest_template }}' in scoop_text
    assert ".replace(placeholder, value)" not in scoop_text
    assert "cargo run --quiet --manifest-path release-source/Cargo.toml" not in scoop_text
    assert "SC_COMPOSE_RENDERER" in scoop_text
    assert "Checkout workflow support" in scoop_text
    assert "uses: ./.github/actions/extract-published-sc-compose" in scoop_text

    assert "Render formula from the manifest-declared template" in homebrew_text
    assert 'FORMULA_TEMPLATE: ${{ fromJSON(needs.verify-release.outputs.channel_config).channel.formula_template }}' in homebrew_text
    assert ".replace(placeholder, value)" not in homebrew_text
    assert "SC_COMPOSE_RENDERER" in homebrew_text
    assert "Checkout workflow support" in homebrew_text
    assert "uses: ./.github/actions/extract-published-sc-compose" in homebrew_text
    assert "install_block" not in homebrew_text
    assert "bundled_paths" in homebrew_text

    renderer_action = (
        repo_root()
        / ".github"
        / "actions"
        / "extract-published-sc-compose"
        / "action.yml"
    ).read_text(encoding="utf-8")
    assert "Published renderer archive is missing bin/sc-compose" in renderer_action
    assert "renderer-path=${renderer}" in renderer_action


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
    needs: [gate-and-tag, release-plan]
    strategy:
      fail-fast: false
      matrix: ${{ fromJSON(needs.release-plan.outputs.python_wheel_matrix) }}
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ needs.gate-and-tag.outputs.release_ref }}
      - uses: ./.github/actions/setup-python-release-build"""
    sdist_job = """  build-python-sdists:
    needs: [gate-and-tag, release-plan]
    strategy:
      fail-fast: false
      matrix: ${{ fromJSON(needs.release-plan.outputs.python_sdist_matrix) }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ needs.gate-and-tag.outputs.release_ref }}
      - uses: ./.github/actions/setup-python-release-build"""

    assert wheels_job in text
    assert sdist_job in text
    assert "matrix.cargo_manifest" in text
    assert "matrix.pyproject" in text


def test_python_package_metadata_uses_local_readme_for_sdist() -> None:
    pyproject_text = python_pyproject_text()
    cargo_toml_text = python_cargo_toml_text()

    assert 'readme = "README.md"' in pyproject_text
    assert 'readme = "README.md"' in cargo_toml_text
    assert "../../README.md" not in pyproject_text
    assert "../../README.md" not in cargo_toml_text


def write_readme_fixture(
    tmp_path: Path,
    *,
    dependency_version: str,
    status_version: str,
    stability_minor: str,
    dependency_crate: str = "sc-composer",
) -> tuple[Path, Path, Path]:
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
                f'{dependency_crate} = "{dependency_version}"',
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
    manifest = tmp_path / "release" / "publish-artifacts.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(
        "\n".join(
            [
                "[project]",
                f'readme_dependency_crate = "{dependency_crate}"',
                "",
                "[[crates]]",
                'artifact = "readme-dependency"',
                f'package = "{dependency_crate}"',
                'cargo_toml = "crates/readme-dependency/Cargo.toml"',
                "publish_order = 1",
                "wait_after_publish_seconds = 0",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return workspace, readme, manifest


def run_sync_readme_version(
    workspace: Path, readme: Path, manifest: Path
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "scripts/release_artifacts.py",
            "sync-readme-version",
            "--manifest",
            str(manifest),
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


def run_verify_readme_version(
    workspace: Path, readme: Path, manifest: Path
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "scripts/release_artifacts.py",
            "verify-readme-version",
            "--manifest",
            str(manifest),
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
    crate_inherits_workspace_version: bool = True,
) -> tuple[Path, Path]:
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
        version = "version.workspace = true" if crate_inherits_workspace_version else 'version = "1.3.1"'
        path.write_text(
            "[package]\nname = \"fixture\"\n" + version + "\n",
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
    manifest = tmp_path / "release" / "publish-artifacts.toml"
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(
        "\n".join(
            [
                "[[crates]]",
                'artifact = "sc-sha"',
                'package = "fixture"',
                'cargo_toml = "crates/sc-sha/Cargo.toml"',
                "publish_order = 1",
                "wait_after_publish_seconds = 0",
                "",
                "[[python_packages]]",
                'artifact = "sc-compose-python"',
                'package = "fixture"',
                'manifest = "bindings/python/pyproject.toml"',
                'module = "fixture"',
                'publish = "pypi"',
                "",
                "[[python_packages]]",
                'artifact = "sc-sha-python"',
                'package = "fixture-sha"',
                'manifest = "bindings/sc-sha-python/pyproject.toml"',
                'module = "fixture_sha"',
                'publish = "pypi"',
                "",
                "[[python_distributions]]",
                'name = "fixture"',
                'source = "bindings/python"',
                'cargo_manifest = "bindings/python/Cargo.toml"',
                "sdist = true",
                'wheels = ["ubuntu-latest"]',
                "",
                "[[python_distributions]]",
                'name = "fixture-sha"',
                'source = "bindings/sc-sha-python"',
                'cargo_manifest = "bindings/sc-sha-python/Cargo.toml"',
                "sdist = true",
                'wheels = ["ubuntu-latest"]',
                "",
            ]
        ),
        encoding="utf-8",
    )
    return workspace, manifest


def run_verify_version_lockstep(workspace: Path, manifest: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "scripts/release_artifacts.py",
            "verify-version-lockstep",
            "--manifest",
            str(manifest),
            "--workspace-toml",
            str(workspace),
        ],
        cwd=Path(__file__).resolve().parents[2],
        text=True,
        capture_output=True,
        check=False,
    )


def test_verify_readme_version_passes_when_readme_matches_workspace(tmp_path: Path) -> None:
    workspace, readme, manifest = write_readme_fixture(
        tmp_path, dependency_version="1.2.0", status_version="1.2.0", stability_minor="1.2"
    )

    result = run_verify_readme_version(workspace, readme, manifest)

    assert result.returncode == 0, result.stderr
    assert "readme version verification passed" in result.stdout


def test_verify_readme_version_rejects_stale_dependency_example(tmp_path: Path) -> None:
    workspace, readme, manifest = write_readme_fixture(
        tmp_path, dependency_version="1.1.0", status_version="1.2.0", stability_minor="1.2"
    )

    result = run_verify_readme_version(workspace, readme, manifest)

    assert result.returncode != 0
    assert "sc-composer dependency example" in result.stderr


def test_verify_readme_version_rejects_stale_status_table(tmp_path: Path) -> None:
    workspace, readme, manifest = write_readme_fixture(
        tmp_path, dependency_version="1.2.0", status_version="1.1.0", stability_minor="1.1"
    )

    result = run_verify_readme_version(workspace, readme, manifest)

    assert result.returncode != 0
    assert "Status table Version row" in result.stderr
    assert "Status table Stability row" in result.stderr


def test_sync_readme_version_rewrites_stale_references(tmp_path: Path) -> None:
    workspace, readme, manifest = write_readme_fixture(
        tmp_path, dependency_version="1.1.0", status_version="1.1.0", stability_minor="1.1"
    )

    sync_result = run_sync_readme_version(workspace, readme, manifest)

    assert sync_result.returncode == 0, sync_result.stderr
    assert "synced 3 readme version reference(s) to 1.2.0" in sync_result.stdout

    verify_result = run_verify_readme_version(workspace, readme, manifest)
    assert verify_result.returncode == 0, verify_result.stderr


def test_readme_version_commands_use_the_manifest_declared_dependency_crate(
    tmp_path: Path,
) -> None:
    workspace, readme, manifest = write_readme_fixture(
        tmp_path,
        dependency_crate="fixture-composer",
        dependency_version="1.1.0",
        status_version="1.2.0",
        stability_minor="1.2",
    )

    sync_result = run_sync_readme_version(workspace, readme, manifest)

    assert sync_result.returncode == 0, sync_result.stderr
    assert 'fixture-composer = "1.2.0"' in readme.read_text(encoding="utf-8")
    verify_result = run_verify_readme_version(workspace, readme, manifest)
    assert verify_result.returncode == 0, verify_result.stderr


def test_verify_version_lockstep_accepts_all_release_version_sources(tmp_path: Path) -> None:
    result = run_verify_version_lockstep(*write_version_lockstep_fixture(tmp_path))

    assert result.returncode == 0, result.stderr
    assert "version lockstep verification passed" in result.stdout


def test_verify_version_lockstep_rejects_non_inherited_crate_version(tmp_path: Path) -> None:
    result = run_verify_version_lockstep(
        *write_version_lockstep_fixture(tmp_path, crate_inherits_workspace_version=False)
    )

    assert result.returncode != 0
    assert "crates/sc-sha/Cargo.toml" in result.stderr
    assert "must inherit workspace.package.version" in result.stderr


def test_verify_version_lockstep_rejects_python_package_drift(tmp_path: Path) -> None:
    result = run_verify_version_lockstep(
        *write_version_lockstep_fixture(tmp_path, python_version="1.3.1")
    )

    assert result.returncode != 0
    assert "bindings/python/pyproject.toml" in result.stderr
    assert "[project].version mismatch" in result.stderr
