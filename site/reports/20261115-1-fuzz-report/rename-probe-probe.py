import json, os, sys, tempfile, queue, subprocess, threading
from contextlib import ExitStack
from pathlib import Path

repo = Path('/Users/randlee/github/sc-compose-worktrees/sprint/w-6-adversarial-fuzz')
sys.path.insert(0, str(repo / 'scripts'))
import integration_repour as h
import integration_repour_fuzz as f

existing = h.server_processes()
assert all('shared-server' in s for s in existing), existing
servers = []
with ExitStack() as stack:
    root = Path(tempfile.mkdtemp(prefix='sc-deep-repour-')).resolve()
    stack.callback(h.cleanup_root, root, servers)
    home, workspace, data = root/'home', root/'work', root/'data'
    for directory in (home, workspace, data): directory.mkdir()
    env = {k:v for k,v in os.environ.items() if not k.startswith(('BEADS_', 'BD_DOLT_'))}
    env.update(HOME=str(home), USERPROFILE=str(home), XDG_CONFIG_HOME=str(home/'.config'), BEADS_DIR=str(workspace/'.beads'), BEADS_NO_DAEMON='1', DOLT_ROOT_PATH=str(home), DOLT_DISABLE_EVENT_FLUSH='1', BD_DISABLE_EVENT_FLUSH='1', BD_DISABLE_METRICS='1', BD_DOLT_AUTO_PUSH='false', BD_DOLT_LOCAL_ONLY='true')
    bd, dolt = '/opt/homebrew/bin/bd', '/opt/homebrew/bin/dolt'
    for key, value in [('versioncheck.disabled','true'), ('user.name','integration'), ('user.email','integration@example.invalid')]:
        h.run([dolt,'config','--global','--add',key,value],cwd=data,env=env)
    port = h.select_server_port(None)
    env['SC_COMPOSE_TEST_DOLT_PORT'] = str(port)
    log = stack.enter_context((root/'server.log').open('wb'))
    server = subprocess.Popen([dolt, 'sql-server', '--host', '127.0.0.1', '--port', str(port)], cwd=data, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    servers.append(server)
    events = queue.Queue()
    reader = threading.Thread(target=h.read_server_output, args=(server.stdout,log,port,events), daemon=True)
    reader.start()
    stack.callback(server.stdout.close)
    stack.callback(h.join_server_reader, reader)
    stack.callback(h.stop_server, server)
    h.wait_for_server(events,server,root/'server.log')
    def bd_run(*args): return h.run([bd,*args],cwd=workspace,env=env).stdout
    bd_run(*h.initialization_args('server',port))
    def cli(operation,path,*args,expected=0):
        return json.loads(h.run([str(repo/'target/debug/sc-compose'),'bead',operation,str(path),*args,'--json'],cwd=workspace,env=env,expected=expected).stdout)
    def show(bead): return json.loads(bd_run('show',bead,'--json'))[0]
    def snapshot():
        rows = json.loads(bd_run('list','--all','-n','0','--json'))
        return {row['id']:{'row':show(row['id']),'deps':json.loads(bd_run('dep','list',row['id'],'--json'))} for row in rows}
    path = workspace / 'rename.formula.toml.j2'
    initial = 'formula = "rename"\nversion = 1\ntype = "workflow"\n[[steps]]\nid = "old"\ntitle = "Old"\n[[steps]]\nid = "dependent"\ntitle = "Dependent"\nneeds = ["old"]\n'
    revised = 'formula = "rename"\nversion = 1\ntype = "workflow"\n[[steps]]\nid = "new"\ntitle = "New"\n[[steps]]\nid = "dependent"\ntitle = "Dependent revised"\nneeds = ["new"]\n'
    print(json.dumps({'initial':initial, 'revised':revised}), flush=True)
    path.write_text(initial)
    original = cli('pour',path)['payload']['graph']
    root_id, ids = original['parent'], original['ids']
    bd_run('update',ids['old'],'--notes','preserve old runtime','--add-label','user:keep')
    bd_run('update',ids['dependent'],'--notes','preserve dependent runtime')
    before = snapshot()
    path.write_text(revised)
    flags = ('--root',root_id,'--allow-uncommitted')
    preview = cli('repour',path,*flags,'--dry-run')['payload']['graph']
    assert snapshot() == before, 'preview changed state'
    applied = cli('repour',path,*flags)['payload']['graph']
    assert preview['changes'] == applied['changes'], (preview, applied)
    after = snapshot()
    new_id = applied['ids']['new']
    assert applied['ids']['dependent'] == ids['dependent']
    assert new_id != ids['old']
    assert 'repour:removed' in after[ids['old']]['row']['labels']
    assert 'user:keep' in after[ids['old']]['row']['labels']
    assert after[ids['old']]['row']['notes'] == 'preserve old runtime'
    assert after[ids['dependent']]['row']['notes'] == 'preserve dependent runtime'
    deps = {d['id'] for d in after[ids['dependent']]['deps']}
    assert new_id in deps and ids['old'] not in deps, deps
    assert any(c['change'] == 'mark_removed_node' and c['bead'] == ids['old'] for c in applied['changes'])
    replay = cli('repour',path,*flags)['payload']['graph']
    assert snapshot() == after, 'replay changed stored state'
    print(json.dumps({'case':'rename-id','original_ids':ids,'new_id':new_id,'preview':preview,'applied':applied,'replay':replay,'dependent_deps':list(deps),'passed':True}),flush=True)
print('PASS: 1 step-ID rename case', flush=True)
print('servers_after=',sorted(h.server_processes()), flush=True)
