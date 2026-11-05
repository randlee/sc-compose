"""Consumer checks use the existing preflight verdict."""
from pathlib import Path
import shutil
import subprocess

RUNNER = Path(__file__).resolve().parents[1] / "run-consumer-preflight.sh"


def setup_repo(tmp_path):
    script = tmp_path / ".github/scripts/run-consumer-preflight.sh"
    script.parent.mkdir(parents=True)
    shutil.copyfile(RUNNER, script)
    (tmp_path / "release").mkdir()
    return script


def run(script):
    return subprocess.run(["bash", str(script)], capture_output=True, text=True)


def test_no_checks_declared_is_noop(tmp_path):
    script = setup_repo(tmp_path)
    assert run(script).returncode == 0
    (tmp_path / "release/publish-preflight.toml").write_text("schema_version = 1\n")
    assert run(script).returncode == 0


def test_check_outcomes_and_stderr(tmp_path):
    script = setup_repo(tmp_path)
    (tmp_path / "release/publish-preflight.toml").write_text(
        "[[checks]]\nid = 'failure'\nrun = \"printf 'diagnostic!\\n' >&2; exit 7\"\n"
        "[[checks]]\nid = 'later'\nrun = 'exit 0'\n"
    )
    result = run(script)
    assert result.returncode == 1
    assert "diagnostic!\n" in result.stderr
    assert "failure: exit 7" in result.stdout
    assert "later: exit 0" in result.stdout


def test_bad_configuration_rejected_before_any_command(tmp_path):
    script = setup_repo(tmp_path)
    (tmp_path / "release/publish-preflight.toml").write_text(
        "[[checks]]\nid='first'\nrun='touch should-not-exist'\n"
        "[[checks]]\nid='invalid'\nrun=123\n"
    )
    assert run(script).returncode == 1
    assert not (tmp_path / "should-not-exist").exists()
