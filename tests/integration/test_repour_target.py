"""Controlled installation, mode selection, and report identity checks."""
import hashlib
import datetime
import json
import os
from pathlib import Path
import subprocess
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
        with patch.object(sys, 'argv', ['target']), patch.object(target.subprocess, 'run', return_value=completed) as run:
            self.assertEqual(target.main(), 0)
        command = run.call_args.args[0]
        self.assertEqual(command[-2], 'atmbench@localhost')
        self.assertNotIn('--install-bd-from', command[-1])

    def test_report_preserves_target_build_and_mode_panels(self):
        with tempfile.TemporaryDirectory() as tmp:
            reports = Path(tmp)
            (reports / 'index.html').write_text('<html><body>\n    <ul>\n    </ul>\n</body></html>')
            with patch.object(suite, 'REPORTS', reports), patch.object(suite, 'RUN_DATE', datetime.date(2001, 1, 2)):
                suite.write_report('stock 1.3.4', 'stockhash', 'dolt', [('embedded', 'PASS', '')], profile='fuzz', mode='embedded', target='atmbench')
                report = suite.write_report('fork 1.3.4', 'forkhash', 'dolt', [('server', 'PASS', '')], profile='fuzz', mode='server', target='atmbench')
                suite.write_report('fork 1.3.4', 'localhash', 'dolt', [('local', 'PASS', '')], profile='fuzz', target='local')
            sections = list(ET.parse(report).getroot().iter('{http://www.w3.org/1999/xhtml}section'))
            self.assertEqual(report.name, '20010102-repour-fuzz.xhtml')
            self.assertEqual(len(sections), 3)
            self.assertTrue(any('atmbench|' in s.attrib['data-key'] and s.attrib['data-key'].endswith('|embedded') for s in sections))
            text = ''.join(ET.parse(report).getroot().itertext())
            self.assertIn('stockhash', text)
            self.assertIn('forkhash', text)
            self.assertIn('localhash', text)


if __name__ == '__main__':
    unittest.main()
