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


def test_release_retry_is_bounded_and_injectable(tmp_path: Path) -> None:
    helper = REPO_ROOT / ".github/actions/setup-beads/download.sh"
    script = r'''
beads_download_once() {
  attempts=$((attempts + 1))
  if (( attempts < 3 )); then return 22; fi
  printf 'asset' > "$2"
}
beads_retry_sleep() { delays="${delays}$1,"; }
source "$1"
attempts=0; delays=""
download_beads_release 'https://example.invalid/asset' "$2"
printf '%s %s\n' "$attempts" "$delays"
'''
    output = subprocess.run(
        ["bash", "-c", script, "test", str(helper), str(tmp_path / "asset")],
        capture_output=True, text=True, check=True,
    )
    assert output.stdout.strip() == "3 1,2,"
    assert (tmp_path / "asset").read_text() == "asset"


def test_release_retry_exhaustion_reports_typed_cause_without_waiting(tmp_path: Path) -> None:
    helper = REPO_ROOT / ".github/actions/setup-beads/download.sh"
    script = r'''
beads_download_once() { attempts=$((attempts + 1)); return 22; }
beads_retry_sleep() { delays="${delays}$1,"; }
source "$1"
attempts=0; delays=""
if download_beads_release 'https://example.invalid/checksums.txt' "$2"; then exit 99; else status=$?; fi
printf '%s %s %s\n' "$status" "$attempts" "$delays"
'''
    output = subprocess.run(
        ["bash", "-c", script, "test", str(helper), str(tmp_path / "checksums")],
        capture_output=True, text=True, check=True,
    )
    assert output.stdout.strip() == "22 5 1,2,4,8,"
    assert "BEADS.RELEASE_DOWNLOAD_FAILED" in output.stderr
    assert "checksums.txt" in output.stderr
    assert "curl exit 22" in output.stderr
    assert not (tmp_path / "checksums").exists()


def test_action_retries_archive_and_checksum_before_verification() -> None:
    action = (REPO_ROOT / ".github/actions/setup-beads/action.yml").read_text()
    archive = 'download_beads_release "${release_url}/${archive}"'
    checksum = 'download_beads_release "${release_url}/checksums.txt"'
    assert action.index(archive) < action.index(checksum) < action.index('expected_sha=')
