#!/usr/bin/env python3
"""Prepare the pinned wheel before delegating to the managed sc-lint bootstrap."""
import os
from pathlib import Path
import subprocess
import sys
import venv

ROOT = Path(__file__).resolve().parents[1]
REQUIREMENTS = ROOT / 'scripts/sc_lint_constraints.txt'
RELEASE_LINKS = 'https://github.com/randlee/sc-lint/releases/expanded_assets/v0.8.6'


def prepare(run=subprocess.run, create=venv.create):
    directory = ROOT / '.sc-lint' / 'venv'
    python = directory / ('Scripts/python.exe' if os.name == 'nt' else 'bin/python')
    if not python.is_file():
        create(directory, with_pip=True)
    env = dict(os.environ, PIP_FIND_LINKS=RELEASE_LINKS)
    env.pop('PIP_CONSTRAINT', None)
    run([str(python), '-m', 'pip', 'install', '--disable-pip-version-check',
         '--force-reinstall', '--require-hashes', '-r', str(REQUIREMENTS)], cwd=ROOT, env=env, check=True)


def main():
    args = sys.argv[1:]
    if args and Path(args[0]).as_posix().endswith('.sc-lint/bootstrap.py') and '--dry-run' not in args:
        prepare()
    return subprocess.call([sys.executable, *args])


if __name__ == '__main__':
    raise SystemExit(main())
