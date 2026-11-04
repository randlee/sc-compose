"""A crates-only manifest (no binaries, wheels or npm packages) completes the GitHub Release.

actions/download-artifact creates its path only when at least one artifact
exists, so a crates-only release job starts with no ``artifacts/`` directory.
These tests extract the real bash from release.yml and run the asset steps in
order against an empty download, then check that the published-release probe
treats the resulting Release (only ``checksums.txt``) as complete on a re-run.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from test_fail_closed_probes import REPO_ROOT, extract_run_block, run_release_probe


RELEASE_YML = REPO_ROOT / ".github" / "workflows" / "release.yml"
SCRIPTS = REPO_ROOT / ".github" / "scripts"

CRATES_ONLY_MANIFEST = """\
schema_version = 1

[project]
name = "fixture"
archive_prefix = "fixture"
description = "Fixture release"
homepage = "https://example.invalid/fixture"
license = "MIT"
readme_dependency_crate = "fixture-core"

[[release_targets]]
target = "x86_64-unknown-linux-gnu"
os = "ubuntu-latest"
archive = "tar.gz"

[[crates]]
artifact = "fixture-core"
package = "fixture-core"
cargo_toml = "crates/fixture-core/Cargo.toml"
publish = true
publish_order = 1
wait_after_publish_seconds = 0
"""


def release_step(name: str) -> str:
    """Return the bash of the named release.yml step (block or one-line `run:`)."""
    text = RELEASE_YML.read_text(encoding="utf-8")
    anchor = f"name: {name}\n"
    start = text.index(anchor)
    end = text.find("\n      - ", start + len(anchor))
    step = text[start : end if end != -1 else len(text)]
    if "run: |" in step:
        return extract_run_block(text, anchor)
    line = next(line for line in step.splitlines() if line.strip().startswith("run: "))
    return line.split("run: ", 1)[1] + "\n"


def run_step(
    script: str, cwd: Path, env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    # GitHub runs an unset-shell `run:` as `bash -e {0}`; stdin is not a file
    # list, so a bare `sha256sum` with no arguments must never be reached.
    return subprocess.run(
        ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", script],
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        text=True,
        capture_output=True,
        check=False,
        timeout=30,
        env={**os.environ, **(env or {})},
    )


def write_crates_only_manifest(tmp_path: Path) -> Path:
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/fixture-core"]\n\n[workspace.package]\nversion = "1.0.0"\n',
        encoding="utf-8",
    )
    crate = tmp_path / "crates" / "fixture-core"
    crate.mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "fixture-core"\nversion = "1.0.0"\n', encoding="utf-8"
    )
    # Kept out of release/ so the staging assertions below see only what the
    # asset steps produce.
    manifest = tmp_path / "manifest" / "publish-artifacts.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(CRATES_ONLY_MANIFEST, encoding="utf-8")
    contracts = REPO_ROOT / "release" / "publish-channel-contracts.toml"
    if not contracts.is_file():
        contracts = contracts.with_name(contracts.name + ".j2")
    (manifest.parent / "publish-channel-contracts.toml").write_text(
        contracts.read_text(encoding="utf-8"), encoding="utf-8"
    )
    return manifest


def test_collect_archives_tolerates_a_missing_artifacts_directory(tmp_path: Path) -> None:
    assert not (tmp_path / "artifacts").exists()
    result = run_step(release_step("Collect archives"), tmp_path)
    assert result.returncode == 0, result.stdout + result.stderr
    assert (tmp_path / "release").is_dir()
    assert list((tmp_path / "release").iterdir()) == []


def test_generate_checksums_writes_a_digestless_nonempty_file_for_an_empty_set(
    tmp_path: Path,
) -> None:
    (tmp_path / "release").mkdir()
    result = run_step(release_step("Generate checksums"), tmp_path / "release")
    assert result.returncode == 0, result.stdout + result.stderr
    checksums = (tmp_path / "release" / "checksums.txt").read_text(encoding="utf-8")
    # Never zero bytes (an empty upload) and never a digest of stdin ("-").
    assert checksums.strip()
    assert all(line.startswith("#") for line in checksums.splitlines()), checksums


def test_generate_checksums_still_digests_every_asset(tmp_path: Path) -> None:
    release = tmp_path / "release"
    release.mkdir()
    (release / "fixture_1.0.0_x86_64-unknown-linux-gnu.tar.gz").write_bytes(b"archive")
    (release / "fixture-1.0.0-py3-none-any.whl").write_bytes(b"wheel")
    result = run_step(release_step("Generate checksums"), release)
    assert result.returncode == 0, result.stdout + result.stderr
    names = sorted(
        line.split("  ", 1)[1]
        for line in (release / "checksums.txt").read_text(encoding="utf-8").splitlines()
    )
    assert names == ["fixture-1.0.0-py3-none-any.whl", "fixture_1.0.0_x86_64-unknown-linux-gnu.tar.gz"]


def test_crates_only_release_assets_chain_and_rerun_probe_complete(tmp_path: Path) -> None:
    manifest = write_crates_only_manifest(tmp_path)
    validated = subprocess.run(
        [
            sys.executable,
            str(SCRIPTS / "release_artifacts.py"),
            "validate-manifest",
            "--manifest",
            str(manifest),
            "--workspace-toml",
            str(tmp_path / "Cargo.toml"),
        ],
        cwd=tmp_path,
        text=True,
        capture_output=True,
        check=False,
        timeout=30,
    )
    assert validated.returncode == 0, validated.stdout + validated.stderr

    # The steps run from the checkout root, where .github/scripts lives.
    (tmp_path / ".github").mkdir()
    (tmp_path / ".github" / "scripts").symlink_to(SCRIPTS, target_is_directory=True)
    env = {"RELEASE_ARTIFACT_MANIFEST": str(manifest)}

    output = tmp_path / "github-output"
    output.write_text("", encoding="utf-8")
    expected = run_step(
        release_step("Read expected release asset patterns from manifest"),
        tmp_path,
        {**env, "GITHUB_OUTPUT": str(output)},
    )
    assert expected.returncode == 0, expected.stdout + expected.stderr
    patterns = output.read_text(encoding="utf-8").split("patterns<<EOF\n", 1)[1].split("EOF\n", 1)[0]
    # Nothing but checksums.txt is required of a crates-only Release.
    assert patterns == "^checksums\\.txt$\n"

    for step, cwd in (
        ("Collect archives", tmp_path),
        ("Verify collected Python distributions", tmp_path),
        ("Generate checksums", tmp_path / "release"),
    ):
        result = run_step(release_step(step), cwd, env)
        assert result.returncode == 0, f"{step}: {result.stdout}{result.stderr}"
    assert sorted(path.name for path in (tmp_path / "release").iterdir()) == ["checksums.txt"]

    # "Create GitHub Release" uploads release/*; a re-run's probe must then
    # read that Release as complete and skip the upload.
    probe_dir = tmp_path / "probe"
    probe_dir.mkdir()
    result, outputs = run_release_probe(
        probe_dir,
        mode="found",
        probe="true",
        release_json='{"isDraft": false, "assets": [{"name": "checksums.txt"}]}',
        asset_patterns=patterns,
    )
    assert result.returncode == 0, result.stderr
    assert outputs["release_state"] == "complete"
