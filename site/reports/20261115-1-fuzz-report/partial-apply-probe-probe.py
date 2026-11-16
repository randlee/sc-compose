import json, os, sys, tempfile, queue, subprocess, threading
from contextlib import ExitStack
from pathlib import Path

repo = Path('/Users/randlee/github/sc-compose-worktrees/sprint/w-6-adversarial-fuzz')
sys.path.insert(0, str(repo / 'scripts'))
import integration_repour as h

existing = h.server_processes()
assert all('shared-server' in s for s in existing), existing
servers = []
with ExitStack() as stack:
    root = Path(tempfile.mkdtemp(prefix='sc-partial-repour-')).resolve()
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
    binary = str(repo/'target/debug/sc-compose')
    def cli(operation,path,*args,expected=0):
        return json.loads(h.run([binary,'bead',operation,str(path),*args,'--json'],cwd=workspace,env=env,expected=expected).stdout)
    def show(bead): return json.loads(bd_run('show',bead,'--json'))[0]
    def snapshot():
        rows = json.loads(bd_run('list','--all','-n','0','--json'))
        return {row['id']:{'row':show(row['id']),'deps':json.loads(bd_run('dep','list',row['id'],'--json'))} for row in rows}
    path = workspace/'partial.formula.toml.j2'
    initial = 'formula = "partial"\nversion = 1\ntype = "workflow"\n[[steps]]\nid = "alpha"\ntitle = "Alpha"\n[[steps]]\nid = "beta"\ntitle = "Beta"\nneeds = ["alpha"]\n'
    revised = 'formula = "partial"\nversion = 1\ntype = "workflow"\n[[steps]]\nid = "alpha"\ntitle = "Alpha revised"\n[[steps]]\nid = "beta"\ntitle = "Beta"\nneeds = ["alpha"]\n[[steps]]\nid = "gamma"\ntitle = "Gamma"\nneeds = ["beta"]\n'
    path.write_text(initial)
    graph = cli('pour',path)['payload']['graph']
    root_id, ids = graph['parent'],graph['ids']
    before = snapshot()
    path.write_text(revised)
    wrapper = workspace/'failing_bd.py'
    counter = workspace/'mutation-count'
    wrapper.write_text('#!/usr/bin/env python3\n' + 'import os, pathlib, sys\n' + f'real={bd!r}; counter=pathlib.Path({str(counter)!r})\n' + "args=sys.argv[1:]\nmutating=bool(args) and (args[0] == 'update' or (args[0] == 'create' and '--dry-run' not in args) or args[:2] in (['dep','add'],['dep','remove']))\ncount=int(counter.read_text()) if counter.exists() else 0\ncount += int(mutating)\ncounter.write_text(str(count)) if mutating else None\nsys.exit('injected second write failure') if mutating and count == 2 else None\nos.execv(real, [real, *args])\n")
    wrapper.chmod(0o755)
    request = {'schema':'sc-compose/beads/v1','operation':'repour','working_directory':str(workspace),'template':str(path),'rendered_formula':str(workspace/'partial.formula.toml'),'compose_variables':{},'formula_name':'partial','bead_variables':{},'bd_executable':str(wrapper),'pour_authorization':'CreatePersistentBeads','relations':[]}
    request_path = workspace/'request.json'
    request_path.write_text(json.dumps(request))
    command = [binary,'bead','repour','--request',str(request_path),'--root',root_id,'--allow-uncommitted','--json']
    result = json.loads(h.run(command,cwd=workspace,env=env,expected=2).stdout)['payload']
    assert result['error']['code'] == 'BEADS_GRAPH_APPLY_PARTIAL', result
    details = result['error']['details']
    assert details['applied'] == [{'change':'add_node','step':'gamma'},{'change':'add_edge','from':'step:gamma','to':ids['beta'],'kind':'blocks'}], details
    assert details['pending'] == [{'change':'field_change','bead':ids['alpha'],'status':'open','field':'title','before':'Alpha','after':'Alpha revised'}], details
    after = snapshot()
    assert len(after)==len(before)+1 and show(ids['alpha'])['title']=='Alpha'
    resumed = cli('repour',path,'--root',root_id,'--allow-uncommitted')['payload']['graph']
    assert show(ids['alpha'])['title']=='Alpha revised'
    assert resumed['ids']['gamma'] in snapshot()
    print(json.dumps({'case':'partial-apply-resume','initial':initial,'revised':revised,'injection':'fail second mutating bd command','command':command,'error':result['error'],'resume_ids':resumed['ids'],'status':'PASS'},ensure_ascii=False),flush=True)

print('servers_after=',sorted(h.server_processes()),flush=True)
