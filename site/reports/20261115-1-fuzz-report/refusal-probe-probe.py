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
    path = workspace / 'refusal.formula.toml.j2'
    def write(steps):
        path.write_text('formula = "refusal"\nversion = 1\ntype = "workflow"\n' + steps, encoding='utf-8')
    original = '[[steps]]\nid = "alpha"\ntitle = "Alpha"\n[[steps]]\nid = "beta"\ntitle = "Beta"\nneeds = ["alpha"]\n'
    changed = '[[steps]]\nid = "alpha"\ntitle = "Alpha revised café"\n[[steps]]\nid = "beta"\ntitle = "Beta"\nneeds = ["alpha"]\n'
    dangling = '[[steps]]\nid = "alpha"\ntitle = "Alpha"\nneeds = ["missing-unicode-雪"]\n'
    write(original)
    graph = cli('pour',path)['payload']['graph']
    root_id = graph['parent']
    bd_run('dolt','commit','-m','baseline')
    before = snapshot()
    write(changed)
    result = cli('pour',path,'--root',root_id,expected=2)
    assert result['payload']['error']['code'] == 'BEADS_GRAPH_CONFLICT', result
    assert snapshot() == before
    print(json.dumps({'case':'changed-pour','input':changed,'diagnostic':result['payload']['error']['code'],'unchanged':True}),flush=True)
    write(dangling)
    result = cli('repour',path,'--root',root_id,'--allow-uncommitted',expected=2)
    assert result['payload']['outcome'] != 'succeeded', result
    assert snapshot() == before
    print(json.dumps({'case':'dangling-dependency','input':dangling,'result':result,'unchanged':True},ensure_ascii=False),flush=True)
    bd_run('--dolt-auto-commit=off','update',graph['ids']['alpha'],'--notes','dirty user change')
    dirty = snapshot()
    write(changed)
    result = cli('repour',path,'--root',root_id,expected=2)
    assert result['payload']['error']['code'] == 'BEADS_GRAPH_DIRTY', result
    assert snapshot() == dirty
    print(json.dumps({'case':'dirty-refusal','input':changed,'diagnostic':result['payload']['error']['code'],'unchanged':True},ensure_ascii=False),flush=True)

print('PASS: 3 refusal cases', flush=True)
print('servers_after=',sorted(h.server_processes()), flush=True)
