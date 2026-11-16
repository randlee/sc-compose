from pathlib import Path

def isolated_env(env, workspace, port):
    workspace = Path(workspace).resolve()
    port = int(port)
    if port == 3308 or not 1024 <= port <= 65535:
        raise ValueError('unsafe port')
    root = workspace.parent
    expected_home = root / 'home'
    expected_beads = workspace / '.beads'
    if Path(env.get('HOME', '/')).resolve() != expected_home:
        raise ValueError('unsafe home')
    beads = Path(env.get('BEADS_DIR', '/')).resolve()
    if beads != expected_beads or beads == Path('/Users/randlee/.beads') or Path('/Users/randlee/.beads') in beads.parents:
        raise ValueError('unsafe beads directory')
    if env.get('SC_COMPOSE_TEST_DOLT_PORT') != str(port):
        raise ValueError('port mismatch')
    return dict(env)
