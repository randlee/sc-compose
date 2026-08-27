from __future__ import annotations

import subprocess
from pathlib import Path


REPO_ROOT = next(
    path
    for path in Path(__file__).resolve().parents
    if (path / "install.py").is_file()
)


def test_setup_beads_uses_a_portable_fail_closed_sha256_probe() -> None:
    action = (
        REPO_ROOT / ".github" / "actions" / "setup-beads" / "action.yml"
    ).read_text(encoding="utf-8")

    assert "if command -v sha256sum >/dev/null 2>&1; then" in action
    assert 'sha256sum -b "${download_dir}/${archive}" | awk' in action
    assert "elif command -v shasum >/dev/null 2>&1; then" in action
    assert 'shasum -a 256 -b "${download_dir}/${archive}" | awk' in action
    assert "sed 's/^\\\\//'" in action
    assert "No SHA-256 checksum utility is available" in action
    assert 'echo "Checksum verification failed for ${archive}" >&2' in action
    assert 'echo "Expected SHA-256: ${expected_sha}" >&2' in action
    assert 'echo "Actual SHA-256:   ${actual_sha}" >&2' in action


def test_windows_sha256sum_filename_escape_marker_is_not_part_of_digest() -> None:
    digest = "1f00c29cd9599e182a4a4e829f5210daca2da14155920aee2836d8bc613b2feb"
    result = subprocess.run(
        ["bash", "-c", r"awk '{ print $1 }' | sed 's/^\\//'"],
        input=rf"\{digest} *D:\a\_temp\beads.zip" + "\n",
        text=True,
        capture_output=True,
        check=True,
    )

    assert result.stdout.strip() == digest
