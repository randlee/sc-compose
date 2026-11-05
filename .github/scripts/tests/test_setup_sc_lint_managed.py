"""Deterministic tests for managed release setup retries."""
import importlib.util
from pathlib import Path
import subprocess
from unittest import mock

import pytest

SCRIPT = Path(__file__).resolve().parents[1] / 'setup_sc_lint_managed.py'
spec = importlib.util.spec_from_file_location('managed_setup', SCRIPT)
managed = importlib.util.module_from_spec(spec)
spec.loader.exec_module(managed)


def result(code, error=''):
    return subprocess.CompletedProcess(['bootstrap'], code, stdout='', stderr=error)


def test_transient_504_then_success_retries_with_bounded_delays(capsys):
    error = 'HTTP Error 504: Gateway Timeout (CLI.SC_LINT_RELEASE_UNAVAILABLE)\n'
    runner = mock.Mock(side_effect=[result(4, error), result(4, error), result(0)])
    sleeper = mock.Mock()
    managed.install(['bootstrap'], {}, run=runner, sleep=sleeper)
    assert runner.call_count == 3
    assert sleeper.call_args_list == [mock.call(1), mock.call(2)]
    assert error in capsys.readouterr().err


def test_exhaustion_is_loud_and_nontransient_error_is_not_retried(capsys):
    error = 'HTTP Error 504 (CLI.SC_LINT_RELEASE_UNAVAILABLE)\n'
    runner = mock.Mock(return_value=result(4, error))
    sleeper = mock.Mock()
    with pytest.raises(subprocess.CalledProcessError) as caught:
        managed.install(['bootstrap'], {}, run=runner, sleep=sleeper)
    assert caught.value.returncode == 4
    assert runner.call_count == 4
    assert sleeper.call_args_list == [mock.call(1), mock.call(2), mock.call(4)]
    assert capsys.readouterr().err.count(error) == 4
    runner = mock.Mock(return_value=result(4, 'checksum mismatch\n'))
    sleeper = mock.Mock()
    with pytest.raises(subprocess.CalledProcessError):
        managed.install(['bootstrap'], {}, run=runner, sleep=sleeper)
    runner.assert_called_once()
    sleeper.assert_not_called()


def test_action_passes_release_wheel_constraints_to_bootstrap(tmp_path, monkeypatch):
    install_dir = tmp_path / 'sc-lint' / 'bin'
    install_dir.mkdir(parents=True)
    (install_dir / ('sc-lint.exe' if managed.os.name == 'nt' else 'sc-lint')).touch()
    output = tmp_path / 'github-output-path'
    monkeypatch.setenv('RUNNER_TEMP', str(tmp_path))
    monkeypatch.setenv('GITHUB_PATH', str(output))
    monkeypatch.setenv('SC_LINT_BIN', 'untrusted-override')
    with mock.patch.object(managed, 'install') as install:
        managed.main()
    command, env = install.call_args.args
    assert command[1:] == ['.sc-lint/bootstrap.py', 'setup', '--config', 'sc-lint.toml']
    assert 'SC_LINT_BIN' not in env
    assert env['PIP_FIND_LINKS'].endswith('/releases/expanded_assets/v0.8.6')
    assert env['PIP_CONSTRAINT'] == str(Path('scripts/sc_lint_constraints.txt').resolve())
    assert output.read_text().strip() == str(install_dir)
