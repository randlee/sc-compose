from __future__ import annotations

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
    assert 'sha256sum -b "${download_dir}/${archive}"' in action
    assert "elif command -v shasum >/dev/null 2>&1; then" in action
    assert 'shasum -a 256 -b "${download_dir}/${archive}"' in action
    assert "No SHA-256 checksum utility is available" in action
    assert 'echo "Checksum verification failed for ${archive}" >&2' in action
