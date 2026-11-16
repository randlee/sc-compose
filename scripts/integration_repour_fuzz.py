"""Small seeded revisions, run serially inside integration_repour's owned database.

Replay: python3 scripts/integration_repour.py --fuzz --fuzz-seed 24034
Two seeds maximum; each has three plans with three nodes (four identities total).
No timing, concurrency, or throughput assertion is part of the oracle.
"""
from __future__ import annotations

import json
from pathlib import Path
import random


def plans_for_seed(seed: int) -> list[dict]:
    """Generate acyclic small graphs; remove and restore identities deliberately."""
    rng = random.Random(seed)
    plans = []
    for revision, names in enumerate((['alpha', 'beta', 'gamma'], ['gamma', 'alpha', 'delta'], ['beta', 'delta', 'alpha'])):
        order = rng.sample(names, len(names))
        steps = []
        for index, name in enumerate(order):
            needs = [previous for previous in order[:index] if rng.choice((False, True))]
            steps.append({
                'id': name,
                'title': f'{name} revision {revision}: ' + rng.choice(('quotes " and =', 'Unicode café 日本語', 'line\nbreak', 'semicolon ; $literal')),
                'description': f'seed={seed}; revision={revision}; node={name}',
                'priority': rng.randrange(5),
                'needs': needs,
            })
        plans.append({'formula': f'fuzz_{seed}', 'steps': steps})
    return plans


def formula_text(plan: dict) -> str:
    lines = [f'formula = {json.dumps(plan["formula"])}', 'version = 1', 'type = "workflow"']
    for step in plan['steps']:
        lines.append('[[steps]]')
        lines.extend(f'{key} = {json.dumps(value, ensure_ascii=False)}' for key, value in step.items())
    return '\n'.join(lines) + '\n'


def external_pairs(known: dict[str, str], incoming: str, outgoing: str) -> set[tuple[str, str]]:
    """Distinct outside endpoints cannot close a cycle through the owned graph."""
    assert incoming != outgoing, 'external fixtures require distinct endpoints'
    return {(known['alpha'], outgoing), (incoming, known['gamma'])}


def assert_plan_state(plan: dict, known: dict[str, str], rows: dict, runtime: dict) -> None:
    """Compare real stored rows and edges against the generated plan model."""
    active = {step['id']: step for step in plan['steps']}
    for name, bead in known.items():
        row = rows[bead]['row']
        if bead in runtime:
            for field, value in runtime[bead].items():
                assert row.get(field) == value, (name, field, value, row.get(field))
        assert 'user:fuzz' in row.get('labels', []), (name, row)
        if name in active:
            for field in ('title', 'description', 'priority'):
                assert row[field] == active[name][field], (name, field, active[name][field], row[field])
            assert 'repour:removed' not in row.get('labels', []), (name, row)
        else:
            assert 'repour:removed' in row.get('labels', []), (name, row)
    owned = set(known.values())
    actual = {(bead, dep['id']) for bead in owned for dep in rows[bead]['deps'] if dep['id'] in owned}
    expected = {(known[step['id']], known[need]) for step in plan['steps'] for need in step['needs']}
    assert actual == expected, (expected, actual)


def run_seed(seed: int, workspace: Path, cli, bd_run, show, snapshot) -> str:
    """Run one bounded case; every failure includes exact generated inputs."""
    plans = plans_for_seed(seed)
    trace: list[dict] = []
    evidence = {'seed': seed, 'plans': plans, 'trace': trace}
    path = workspace / f'fuzz_{seed}' / 'plan.formula.toml.j2'
    path.parent.mkdir()
    known: dict[str, str] = {}
    runtime: dict[str, dict] = {}
    try:
        path.write_text(formula_text(plans[0]), encoding='utf-8')
        initial = cli('pour', path)['payload']['graph']
        root = initial['parent']
        known.update(initial['ids'])
        for index, bead in enumerate(known.values()):
            bd_run('update', bead, '--status', 'in_progress', '--assignee', f'fuzz{index}', '--notes', f'keep seed {seed}', '--add-label', 'user:fuzz')
            if index == 1:
                bd_run('close', bead, '--reason', 'fuzz runtime state', '--force')
            row = show(bead)
            runtime[bead] = {field: row.get(field) for field in ('status', 'notes', 'assignee')}
        incoming = json.loads(bd_run('create', f'Incoming seed {seed}', '--type', 'task', '--json'))['id']
        outgoing = json.loads(bd_run('create', f'Outgoing seed {seed}', '--type', 'task', '--json'))['id']
        external = external_pairs(known, incoming, outgoing)
        for source, target in sorted(external):
            bd_run('dep', 'add', source, target)
        assert_plan_state(plans[0], known, snapshot(), runtime)
        for revision, plan in enumerate(plans[1:], start=1):
            path.write_text(formula_text(plan), encoding='utf-8')
            before = snapshot()
            conflict = cli('pour', path, '--root', root, expected=2)['payload']['error']
            assert conflict['code'] == 'BEADS_GRAPH_CONFLICT', conflict
            assert snapshot() == before, 'changed pour wrote database state'
            flags = ('--root', root, '--allow-uncommitted')
            preview = cli('repour', path, *flags, '--dry-run')['payload']['graph']
            assert snapshot() == before, 'dry-run wrote database state'
            applied = cli('repour', path, *flags)['payload']['graph']
            trace.append({'revision': revision, 'changes': applied['changes'], 'ids': applied['ids']})
            assert preview['changes'] == applied['changes'], (preview, applied)
            for name, bead in applied['ids'].items():
                assert name not in known or known[name] == bead, (known, applied['ids'])
                if name not in known:
                    bd_run('update', bead, '--notes', f'new seed {seed}', '--add-label', 'user:fuzz')
                    row = show(bead)
                    runtime[bead] = {field: row.get(field) for field in ('status', 'notes', 'assignee')}
                known[name] = bead
            after = snapshot()
            assert_plan_state(plan, known, after, runtime)
            for source, target in external:
                assert any(dep['id'] == target for dep in after[source]['deps']), (source, target)
            assert sum(change['change'] == 'retain_external_edge' for change in applied['changes']) == 2, applied
            removed = {known[name] for name in known if name not in {step['id'] for step in plan['steps']}}
            reported = {change['bead'] for change in applied['changes'] if change['change'] == 'mark_removed_node' and change.get('remove_command')}
            assert removed == reported, (removed, reported)
            cli('repour', path, *flags)
            assert snapshot() == after, 'exact replay wrote database state'
        return json.dumps(evidence, ensure_ascii=False, indent=2)
    except Exception as error:
        raise AssertionError(f'{error}\nReproduce: python3 scripts/integration_repour.py --fuzz --fuzz-seed {seed}\n{json.dumps(evidence, ensure_ascii=False, indent=2)}') from error
