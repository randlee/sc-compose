"""Controlled checks of the bounded plan generator and database oracle."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('repour_fuzz', ROOT / 'scripts/integration_repour_fuzz.py')
FUZZ = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FUZZ)


class FuzzOracleTests(unittest.TestCase):
    def test_external_fixture_does_not_close_generated_internal_cycle(self):
        def acyclic(edges):
            remaining = set(edges)
            while remaining:
                sources = {source for source, _ in remaining}
                leaves = {target for _, target in remaining} - sources
                if not leaves:
                    return False
                remaining = {(source, target) for source, target in remaining if target not in leaves}
            return True
        known = {name: name for name in ('alpha', 'beta', 'gamma', 'delta')}
        plans = FUZZ.plans_for_seed(24034)
        initial = {(step['id'], need) for step in plans[0]['steps'] for need in step['needs']}
        self.assertFalse(acyclic(initial | {('alpha', 'outside'), ('outside', 'gamma')}))
        for plan in plans:
            internal = {(step['id'], need) for step in plan['steps'] for need in step['needs']}
            self.assertTrue(acyclic(internal | FUZZ.external_pairs(known, 'incoming', 'outgoing')))
        with self.assertRaises(AssertionError):
            FUZZ.external_pairs(known, 'same', 'same')

    def test_seed_replays_identical_bounded_acyclic_inputs_and_restores_nodes(self):
        plans = FUZZ.plans_for_seed(24034)
        self.assertEqual(plans, FUZZ.plans_for_seed(24034))
        self.assertNotEqual(plans, FUZZ.plans_for_seed(24035))
        self.assertEqual(len(plans), 3)
        self.assertEqual({step['id'] for plan in plans for step in plan['steps']}, {'alpha', 'beta', 'gamma', 'delta'})
        for plan in plans:
            prior = set()
            self.assertEqual(len(plan['steps']), 3)
            for step in plan['steps']:
                self.assertLessEqual(set(step['needs']), prior)
                prior.add(step['id'])
        self.assertNotIn('beta', {step['id'] for step in plans[1]['steps']})
        self.assertIn('beta', {step['id'] for step in plans[2]['steps']})

    def test_oracle_rejects_runtime_change_missing_edge_and_removed_node_without_label(self):
        plan = {'steps': [{'id': 'alpha', 'title': 'changed', 'description': 'plan', 'priority': 2, 'needs': []},
                          {'id': 'beta', 'title': 'second', 'description': 'plan', 'priority': 1, 'needs': ['alpha']}]}
        known = {'alpha': 'x.alpha', 'beta': 'x.beta', 'old': 'x.old'}
        runtime = {'x.alpha': {'status': 'closed', 'notes': 'keep', 'assignee': 'owner'}}
        rows = {known[step['id']]: {'row': {**step, 'labels': ['user:fuzz']}, 'deps': []} for step in plan['steps']}
        rows['x.alpha']['row'].update(runtime['x.alpha'])
        rows['x.beta']['deps'] = [{'id': 'x.alpha'}]
        rows['x.old'] = {'row': {'labels': ['user:fuzz', 'repour:removed']}, 'deps': []}
        FUZZ.assert_plan_state(plan, known, rows, runtime)
        changed = copy.deepcopy(rows)
        changed['x.alpha']['row']['status'] = 'open'
        with self.assertRaises(AssertionError):
            FUZZ.assert_plan_state(plan, known, changed, runtime)
        missing = copy.deepcopy(rows)
        missing['x.beta']['deps'] = []
        with self.assertRaises(AssertionError):
            FUZZ.assert_plan_state(plan, known, missing, runtime)
        unlabelled = copy.deepcopy(rows)
        unlabelled['x.old']['row']['labels'] = ['user:fuzz']
        with self.assertRaises(AssertionError):
            FUZZ.assert_plan_state(plan, known, unlabelled, runtime)


if __name__ == '__main__':
    unittest.main()
