"""Deterministic tests for managed release setup retries."""
import importlib.util
from pathlib import Path
import subprocess
from unittest import mock

import pytest
import yaml

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
    env_output = tmp_path / 'github-env'
    monkeypatch.setenv('GITHUB_ENV', str(env_output))
    existing = tmp_path / 'default-cargo' / 'bin' / ('cargo-deny.exe' if managed.os.name == 'nt' else 'cargo-deny')
    existing.parent.mkdir(parents=True)
    existing.write_text('unregistered')
    monkeypatch.setenv('CARGO_HOME', str(existing.parent.parent))
    monkeypatch.setenv('SC_LINT_BIN', 'untrusted-override')
    def simulated_bootstrap(command, env):
        cargo_bin = Path(env['CARGO_HOME']) / 'bin'
        cargo_bin.mkdir(parents=True)
        binary = cargo_bin / existing.name
        if binary.exists():
            raise RuntimeError('unregistered cargo-deny collision')
        binary.write_text('pinned cargo-deny 0.19.4')

    with mock.patch.object(managed, 'install', side_effect=simulated_bootstrap) as install:
        managed.main()
    command, env = install.call_args.args
    assert command[1:] == ['scripts/sc_lint_python.py', '.sc-lint/bootstrap.py', 'setup', '--config', 'sc-lint.toml']
    assert 'SC_LINT_BIN' not in env
    isolated = tmp_path / 'sc-lint' / 'cargo-home'
    assert env['CARGO_HOME'] == str(isolated)
    assert existing.read_text() == 'unregistered'
    assert (isolated / 'bin' / existing.name).read_text() == 'pinned cargo-deny 0.19.4'
    assert output.read_text().splitlines() == [str(install_dir), str(isolated / 'bin')]
    assert env_output.read_text().strip() == f'CARGO_HOME={isolated}'


def test_ci_caches_the_effective_managed_cargo_home_before_setup(tmp_path, monkeypatch):
    """The cache path is the CARGO_HOME the installer really exports, restored first."""
    root = Path(__file__).resolve().parents[3]
    steps = yaml.safe_load((root / '.github/workflows/ci.yml').read_text(encoding='utf-8'))['jobs']['test']['steps']
    setup_index = next(i for i, step in enumerate(steps) if step.get('uses') == './.github/actions/setup-sc-lint-managed')
    cache_index = next(i for i, step in enumerate(steps)
                       if str(step.get('uses', '')).startswith('actions/cache@')
                       and 'sc-lint/cargo-home' in step['with']['path'])
    assert cache_index < setup_index
    rust = next(step for step in steps if str(step.get('uses', '')).startswith('dtolnay/rust-toolchain@'))

    # The effective managed CARGO_HOME, taken from what the installer itself exports.
    monkeypatch.setenv('RUNNER_TEMP', str(tmp_path))
    monkeypatch.setenv('GITHUB_PATH', str(tmp_path / 'github-path'))
    monkeypatch.setenv('GITHUB_ENV', str(tmp_path / 'github-env'))
    (tmp_path / 'sc-lint' / 'bin').mkdir(parents=True)
    (tmp_path / 'sc-lint' / 'bin' / ('sc-lint.exe' if managed.os.name == 'nt' else 'sc-lint')).touch()
    with mock.patch.object(managed, 'install'):
        managed.main()
    exported = (tmp_path / 'github-env').read_text(encoding='utf-8').strip()
    assert exported.startswith('CARGO_HOME=')
    cache_path = steps[cache_index]['with']['path'].replace('${{ runner.temp }}', str(tmp_path))
    assert Path(cache_path) == Path(exported.removeprefix('CARGO_HOME='))

    # Keyed by OS/architecture, the Rust toolchain and the declared tool versions.
    cache = steps[cache_index]['with']
    for key in (cache['key'], cache['restore-keys'].strip()):
        assert '${{ runner.os }}' in key and '${{ runner.arch }}' in key
        assert f"rust-{rust['with']['toolchain']}" in key
        assert "hashFiles('.sc-lint/install.json')" in key
    assert "hashFiles('**/Cargo.lock')" in cache['key']

    # Version verification is not skipped on a hit, and the stale ~/.cargo caches are gone.
    assert 'if' not in steps[setup_index]
    assert 'id' not in steps[cache_index]
    assert not [step for step in steps if str(step.get('with', {}).get('path', '')).startswith('~/.cargo/')]
