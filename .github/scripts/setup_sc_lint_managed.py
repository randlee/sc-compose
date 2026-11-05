"""Run the managed sc-lint bootstrap with bounded release-download retries."""
import os
from pathlib import Path
import subprocess
import sys
import time


def install(command, env, run=subprocess.run, sleep=time.sleep):
    """Retry transient release 504s while preserving bootstrap diagnostics."""
    delays = (1, 2, 4)
    for attempt in range(len(delays) + 1):
        result = run(command, env=env, capture_output=True, text=True)
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        if result.returncode == 0:
            return
        retryable = ('CLI.SC_LINT_RELEASE_UNAVAILABLE' in result.stderr
                     and 'HTTP Error 504' in result.stderr)
        if not retryable or attempt == len(delays):
            raise subprocess.CalledProcessError(result.returncode, command)
        print(f'sc-lint release download returned HTTP 504; retry {attempt + 1}/{len(delays)}', file=sys.stderr)
        sleep(delays[attempt])


def main():
    install_dir = Path(os.environ['RUNNER_TEMP']) / 'sc-lint' / 'bin'
    cargo_home = Path(os.environ['RUNNER_TEMP']) / 'sc-lint' / 'cargo-home'
    env = dict(os.environ, SC_LINT_INSTALL_DIR=str(install_dir), CARGO_HOME=str(cargo_home))
    env.pop('SC_LINT_BIN', None)
    install([sys.executable, 'scripts/sc_lint_python.py', '.sc-lint/bootstrap.py', 'setup', '--config', 'sc-lint.toml'], env)
    binary = install_dir / ('sc-lint.exe' if os.name == 'nt' else 'sc-lint')
    if not binary.is_file():
        raise SystemExit('setup succeeded but no sc-lint executable was found')
    with open(env['GITHUB_PATH'], 'a', encoding='utf-8') as path_file:
        path_file.write(str(install_dir) + '\n')
        path_file.write(str(cargo_home / 'bin') + '\n')
    with open(env['GITHUB_ENV'], 'a', encoding='utf-8') as env_file:
        env_file.write(f'CARGO_HOME={cargo_home}\n')


if __name__ == '__main__':
    main()
