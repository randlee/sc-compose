#!/usr/bin/env python3
"""Run the same bounded suite on atmbench, optionally staging a temporary bd binary."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile

import integration_repour as suite

ROOT = Path(__file__).resolve().parents[1]


def install_bd(source: Path | None, destination: Path) -> dict:
    """No source means no installation or destination creation."""
    if source is None:
        return {'installed': False}
    source = source.resolve(strict=True)
    if not source.is_file() or not os.access(source, os.X_OK):
        raise AssertionError(f'bd source is not executable: {source}')
    destination.mkdir()
    binary = destination / 'bd'
    shutil.copy2(source, binary)
    return {'installed': True, 'source': str(source), 'sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}


def worker(mode: str, source: Path | None) -> dict:
    if sys.platform != 'darwin':
        raise AssertionError('atmbench target requires this macOS host')
    import pwd
    if pwd.getpwuid(os.geteuid()).pw_name != 'atmbench':
        raise AssertionError('target worker must execute as atmbench')
    root = Path(tempfile.mkdtemp(prefix='sc-repour-target-')).resolve()
    try:
        for relative in ('scripts', 'examples/beads/release-under-epic', 'site/reports', 'target/debug'):
            (root / relative).mkdir(parents=True, exist_ok=True)
        for name in ('integration_repour.py', 'integration_repour_fuzz.py'):
            shutil.copy2(ROOT / 'scripts' / name, root / 'scripts' / name)
        for path in (ROOT / 'examples/beads/release-under-epic').iterdir():
            if path.is_file():
                shutil.copy2(path, root / 'examples/beads/release-under-epic' / path.name)
        for path in (ROOT / 'site/reports').iterdir():
            if path.suffix == '.xhtml' or path.name == 'index.html':
                shutil.copy2(path, root / 'site/reports' / path.name)
        shutil.copy2(ROOT / 'target/debug/sc-compose', root / 'target/debug/sc-compose')
        installation = install_bd(source, root / 'bin')
        env = dict(os.environ)
        env['PATH'] = os.pathsep.join((str(root / 'bin'), '/opt/homebrew/bin', '/usr/bin', '/bin'))
        env['SC_COMPOSE_TEST_TARGET'] = 'atmbench'
        result = subprocess.run([sys.executable, str(root / 'scripts/integration_repour.py'), '--mode', mode, '--fuzz'],
                                cwd=root, env=env, text=True, capture_output=True, timeout=1800, check=False)
        reports = {path.name: path.read_text(encoding='utf-8') for path in (root / 'site/reports').glob('*-repour-fuzz.xhtml')}
        reports['index.html'] = (root / 'site/reports/index.html').read_text(encoding='utf-8')
        return {'exit_code': result.returncode, 'installation': installation, 'mode': mode,
                'target': 'atmbench', 'stdout': result.stdout, 'stderr': result.stderr, 'reports': reports}
    finally:
        owned = suite.processes_referencing(root, root, dict(os.environ))
        if owned:
            raise AssertionError(f'retaining target files used by live processes: {owned}; {root}')
        shutil.rmtree(root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mode', choices=('embedded', 'server'), default='server')
    parser.add_argument('--install-bd-from', type=Path, help='Explicitly install this release/fork executable in a temporary atmbench-owned bin; never changes the account installation')
    parser.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.worker:
        print(json.dumps(worker(args.mode, args.install_bd_from)))
        return 0
    command = ['/opt/homebrew/bin/python3', str(Path(__file__).resolve()), '--worker', '--mode', args.mode]
    if args.install_bd_from is not None:
        command.extend(['--install-bd-from', str(args.install_bd_from.resolve(strict=True))])
    result = subprocess.run(['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', 'atmbench@localhost', shlex.join(command)],
                            capture_output=True, text=True, timeout=1860, check=False)
    if result.returncode:
        raise AssertionError(f'atmbench worker failed: {result.stdout}\n{result.stderr}')
    payload = json.loads(result.stdout)
    for name, content in payload.pop('reports').items():
        if Path(name).name != name or (name != 'index.html' and not name.endswith('-repour-fuzz.xhtml')):
            raise AssertionError(f'invalid target report name: {name}')
        (ROOT / 'site/reports' / name).write_text(content, encoding='utf-8')
    print(json.dumps(payload, indent=2))
    return payload['exit_code']


if __name__ == '__main__':
    raise SystemExit(main())
