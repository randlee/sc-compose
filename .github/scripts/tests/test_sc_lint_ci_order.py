"""Consumer-owned CI ordering coverage."""

import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]


class ScLintCiOrderTests(unittest.TestCase):
    def test_workspace_matrix_installs_managed_sc_lint_before_external_tests(self):
        root = ROOT
        workflow = yaml.safe_load((root / '.github/workflows/ci.yml').read_text())
        job = workflow['jobs']['test']
        self.assertEqual(set(job['strategy']['matrix']['os']),
                         {'ubuntu-latest', 'macos-latest', 'windows-latest'})
        steps = job['steps']
        setup_index = next(i for i, step in enumerate(steps)
                           if step.get('uses') == './.github/actions/setup-sc-lint-managed')
        test_index = next(i for i, step in enumerate(steps)
                          if 'sc-compose/external-lint-cli-tests' in step.get('run', ''))
        self.assertLess(setup_index, test_index)
        self.assertNotIn('if', steps[setup_index])
        action = yaml.safe_load((root / '.github/actions/setup-sc-lint-managed/action.yml').read_text())
        install = next(step for step in action['runs']['steps'] if step['name'] == 'Install verified sc-lint release')
        self.assertEqual(install['shell'], 'python')
        self.assertIn('.github/scripts/setup_sc_lint_managed.py', install['run'])
        managed = (root / '.github/scripts/setup_sc_lint_managed.py').read_text()
        self.assertIn("'scripts/sc_lint_python.py', '.sc-lint/bootstrap.py', 'setup'", managed)
        self.assertIn("'--config', 'sc-lint.toml'", managed)
        self.assertIn("GITHUB_PATH", managed)
        self.assertIn('minimum_version = "0.8.6"', (root / 'sc-lint.toml').read_text())
