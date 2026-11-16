"""Controlled installation, mode selection, and report identity checks."""
import hashlib
import datetime
import json
import os
from pathlib import Path
import subprocess
import shlex
import sys
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import integration_repour as suite
import integration_repour_target as target
sys.path.pop(0)


class TargetTests(unittest.TestCase):
    def test_tool_lookup_preserves_alias_and_reports_missing_executable(self):
        alias = os.path.abspath('/portable command/bd')
        with patch.object(target.shutil, 'which', return_value=alias), patch.object(Path, 'resolve', return_value=Path('/canonical/beads')):
            self.assertEqual(target.required_tool('bd'), alias)
        with patch.object(target.shutil, 'which', return_value=None):
            with self.assertRaisesRegex(AssertionError, 'required executable not found on PATH: bd'):
                target.required_tool('bd')

    def test_worker_path_prefers_staged_and_discovered_tools_and_retains_metadata(self):
        record = {'installed': True, 'source': 'selected bd'}
        directories = ('/portable/python', '/portable/beads', '/portable/dolt')
        with patch.dict(os.environ, {'PATH': '/inherited/bin'}, clear=True):
            env = target.worker_environment(Path('/stage'), record, directories)
        self.assertEqual(env['PATH'].split(os.pathsep), [str(Path('/stage/bin')), *directories, '/inherited/bin'])
        self.assertEqual(json.loads(env['SC_COMPOSE_TEST_INSTALLATION']), record)
        self.assertEqual(env['SC_COMPOSE_TEST_TARGET'], 'atmbench')

    def test_installation_outcome_and_source_are_retained_as_escaped_panel_data(self):
        with tempfile.TemporaryDirectory() as tmp:
            reports = Path(tmp)
            (reports / 'index.html').write_text('<html><body>\n    <ul>\n    </ul>\n</body></html>')
            source = '/tmp/bd <release> & "fork"'
            with patch.object(suite, 'REPORTS', reports), patch.object(suite, 'RUN_DATE', datetime.date(2001, 1, 2)):
                suite.write_report('stock', 'stockhash', 'dolt', [], profile='fuzz', mode='embedded', target='atmbench', installation={'installed': True, 'source': source})
                report = suite.write_report('fork', 'forkhash', 'dolt', [], profile='fuzz', target='atmbench', installation={'installed': False, 'source': 'existing bd'})
            paragraphs = [p for p in ET.parse(report).getroot().iter('{http://www.w3.org/1999/xhtml}p') if 'data-installed' in p.attrib]
            self.assertEqual([(p.attrib['data-installed'], p.attrib['data-source']) for p in paragraphs], [('true', source), ('false', 'existing bd')])
            self.assertIn('per-run temporary staging', ''.join(paragraphs[0].itertext()))
            self.assertIn('not installed', ''.join(paragraphs[1].itertext()))

    def test_legacy_key_migration_prefers_explicit_panel_and_local_rerun_replaces_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            reports = Path(tmp)
            (reports / 'index.html').write_text('<html><body>\n    <ul>\n    </ul>\n</body></html>')
            old_key = f'{suite.platform.system()}-{suite.platform.machine()}|bd test|server'
            with patch.object(suite, 'REPORTS', reports), patch.object(suite, 'RUN_DATE', datetime.date(2001, 1, 2)):
                report = reports / '20010102-repour-fuzz.xhtml'
                report.write_text(f'<html xmlns="http://www.w3.org/1999/xhtml"><body><section data-key="local|{old_key}"><p>explicit</p></section><section data-key="{old_key}"><p>legacy</p></section></body></html>')
                suite.write_report('other', 'hash', 'dolt', [], profile='fuzz', target='atmbench', installation={'installed': False, 'source': 'existing bd'})
                sections = list(ET.parse(report).getroot().iter('{http://www.w3.org/1999/xhtml}section'))
                self.assertEqual(len(sections), 2)
                self.assertIn('explicit', ''.join(sections[0].itertext()))
                self.assertNotIn('legacy', ''.join(sections[0].itertext()))
                suite.write_report('bd test', 'newhash', 'dolt', [('rerun', 'PASS', '')], profile='fuzz', installation={'installed': False, 'source': 'existing bd'})
            sections = list(ET.parse(report).getroot().iter('{http://www.w3.org/1999/xhtml}section'))
            self.assertEqual(len(sections), 2)
            self.assertEqual(sum(s.attrib['data-key'] == f'local|{old_key}' for s in sections), 1)
            self.assertIn('rerun', ''.join(sections[0].itertext()))

    def test_embedded_unknown_contract_rejects_unrelated_read_failures(self):
        error = {'code': 'BEADS_GRAPH_DIRTY_UNKNOWN', 'details': {
            'cause_code': 'BEADS_GRAPH_READ_FAILED', 'cause': "Error: 'bd sql' is not yet supported in embedded mode"}}
        suite.assert_embedded_unknown(error)
        error['details']['cause'] = 'unrelated transport failure'
        with self.assertRaises(AssertionError):
            suite.assert_embedded_unknown(error)
        self.assertIn('unknown-state', suite.database_scenario('embedded'))
        self.assertIn('clean apply, dirty refusal', suite.database_scenario('server'))

    def test_default_install_does_not_create_destination(self):
        with tempfile.TemporaryDirectory() as tmp:
            destination = Path(tmp) / 'bin'
            self.assertEqual(target.install_bd(None, destination), {'installed': False})
            self.assertFalse(destination.exists())

    def test_explicit_install_copies_exact_binary_without_changing_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / 'fork bd'
            source.write_bytes(b'controlled executable fixture')
            source.chmod(0o755)
            result = target.install_bd(source, root / 'bin')
            self.assertTrue(result['installed'])
            self.assertEqual(result['sha256'], hashlib.sha256(source.read_bytes()).hexdigest())
            self.assertEqual((root / 'bin/bd').read_bytes(), source.read_bytes())
            self.assertTrue(os.access(root / 'bin/bd', os.X_OK))
            with patch.dict(os.environ, target.worker_environment(root, result), clear=True):
                paragraph = ET.fromstring(suite.installation_markup())
            self.assertEqual(paragraph.attrib['data-installed'], 'true')
            self.assertEqual(paragraph.attrib['data-source'], str(source.resolve()))

    def test_modes_never_share_server_configuration(self):
        embedded = suite.initialization_args('embedded', None)
        self.assertNotIn('--server', embedded)
        self.assertNotIn('--server-port', embedded)
        server = suite.initialization_args('server', 49152)
        self.assertIn('--server', server)
        self.assertEqual(server[server.index('--server-port') + 1], '49152')
        with self.assertRaises(AssertionError):
            suite.initialization_args('server', 3308)

    def test_default_target_invocation_never_requests_install(self):
        payload = {'exit_code': 0, 'installation': {'installed': False}, 'reports': {}}
        completed = subprocess.CompletedProcess([], 0, json.dumps(payload), '')
        tools = {name: os.path.abspath(f'/portable {name}/bin/{name}') for name in ('python3', 'bd', 'dolt')}
        with patch.object(sys, 'argv', ['target']), patch.object(target.shutil, 'which', side_effect=tools.get), patch.object(target.subprocess, 'run', return_value=completed) as run:
            self.assertEqual(target.main(), 0)
        command = run.call_args.args[0]
        self.assertEqual(command[-2], 'atmbench@localhost')
        self.assertNotIn('--install-bd-from', command[-1])
        remote = shlex.split(command[-1])
        self.assertEqual(remote[0], tools['python3'])
        self.assertEqual({remote[index + 1] for index, arg in enumerate(remote) if arg == '--tool-dir'}, {str(Path(path).parent) for path in tools.values()})

    def test_report_preserves_target_build_and_mode_panels(self):
        installation = {'installed': False, 'source': 'controlled fixture bd'}
        with tempfile.TemporaryDirectory() as tmp:
            reports = Path(tmp)
            (reports / 'index.html').write_text('<html><body>\n    <ul>\n    </ul>\n</body></html>')
            with patch.object(suite, 'REPORTS', reports), patch.object(suite, 'RUN_DATE', datetime.date(2001, 1, 2)), patch.dict(os.environ, {'SC_COMPOSE_TEST_INSTALLATION': 'malformed JSON', 'PATH': ''}):
                suite.write_report('stock 1.3.4', 'stockhash', 'dolt', [('embedded', 'PASS', '')], profile='fuzz', mode='embedded', target='atmbench', installation=installation)
                report = suite.write_report('fork 1.3.4', 'forkhash', 'dolt', [('server', 'PASS', '')], profile='fuzz', mode='server', target='atmbench', installation=installation)
                suite.write_report('fork 1.3.4', 'localhash', 'dolt', [('local', 'PASS', '')], profile='fuzz', target='local', installation=installation)
            sections = list(ET.parse(report).getroot().iter('{http://www.w3.org/1999/xhtml}section'))
            self.assertEqual(report.name, '20010102-repour-fuzz.xhtml')
            self.assertEqual(len(sections), 3)
            self.assertTrue(any('atmbench|' in s.attrib['data-key'] and s.attrib['data-key'].endswith('|embedded') for s in sections))
            text = ''.join(ET.parse(report).getroot().itertext())
            self.assertIn('stockhash', text)
            self.assertIn('forkhash', text)
            self.assertIn('localhash', text)
            self.assertIn(suite.INSTALL_MEANING, text)


if __name__ == '__main__':
    unittest.main()
