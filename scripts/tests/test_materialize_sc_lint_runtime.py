from __future__ import annotations

import subprocess
import sys
from pathlib import Path
import shutil


def test_materialize_sc_lint_runtime_copies_the_pinned_helper_bundle(tmp_path: Path) -> None:
    source_root = tmp_path / "sc-lint-source"
    helpers = source_root / ".just"
    helpers.mkdir(parents=True)
    for name in ("run_lint.py", "check_version_sync.py", "lint_common.py", "lint_manifests.py"):
        (helpers / name).write_text(f"# {name}\n", encoding="utf-8")
    (helpers / "lint-config.toml").write_text("[portability]\n", encoding="utf-8")
    repo_root = tmp_path / "consumer"
    repo_root.mkdir()

    result = subprocess.run(
        [
            sys.executable,
            str(Path(__file__).parents[1] / "materialize_sc_lint_runtime.py"),
            "--root",
            str(repo_root),
            "--source-root",
            str(source_root),
        ],
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    assert "materialized sc-lint helpers v0.4.0" in result.stdout
    assert (repo_root / ".just" / "check_version_sync.py").is_file()
    assert (repo_root / ".just" / "lint-config.toml").is_file()
    assert (repo_root / ".just" / ".sc-lint-runtime-version").read_text(
        encoding="utf-8"
    ) == "0.4.0\n"


def test_materialize_sc_lint_runtime_reuses_a_matching_cached_bundle(
    tmp_path: Path,
) -> None:
    source_root = tmp_path / "sc-lint-source"
    helpers = source_root / ".just"
    helpers.mkdir(parents=True)
    for name in ("run_lint.py", "check_version_sync.py", "lint_common.py"):
        (helpers / name).write_text(f"# {name}\n", encoding="utf-8")
    repo_root = tmp_path / "consumer"
    repo_root.mkdir()
    script = Path(__file__).parents[1] / "materialize_sc_lint_runtime.py"

    first = subprocess.run(
        [
            sys.executable,
            str(script),
            "--root",
            str(repo_root),
            "--source-root",
            str(source_root),
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    assert first.returncode == 0, first.stderr
    helper = repo_root / ".just" / "check_version_sync.py"
    original_contents = helper.read_text(encoding="utf-8")
    shutil.rmtree(source_root)

    second = subprocess.run(
        [
            sys.executable,
            str(script),
            "--root",
            str(repo_root),
            "--source-root",
            str(source_root),
        ],
        text=True,
        capture_output=True,
        check=False,
    )

    assert second.returncode == 0, second.stderr
    assert "already materialized" in second.stdout
    assert helper.read_text(encoding="utf-8") == original_contents
