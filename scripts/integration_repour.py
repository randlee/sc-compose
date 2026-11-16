#!/usr/bin/env python3
"""Exercise real pour and repour against one disposable local Dolt server."""
from __future__ import annotations

import datetime
from contextlib import ExitStack
import hashlib
import html
import json
import os
import platform
import queue
import shlex
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import traceback
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / "site" / "reports"
# Keep one report date per process, including runs or controlled checks spanning midnight.
RUN_DATE = datetime.date.today()
INSTALL_MEANING = "Install means per-run atmbench-owned temporary bd staging. It persists nothing after the run; no persistent account installation is performed."
TIMEOUT = 120
SCENARIOS = (
    "pour", "changed pour refuses with changes", "set runtime state",
    "preview, apply, retained runtime and remove/add", "repeated repour changes no stored state",
    "restore removed node", "external edges retained in both directions",
    "conditional add and remove with renamed step", "dangling reference refuses before writes",
    "clean apply, dirty refusal and explicit override", "attach and repour via request using bd rendering",
    "partial apply reports exact progress and resumes", "sequential evolving revisions preserve identities",
)
DEFAULT_FUZZ_SEEDS = (24034, 24035)


class ServerBindError(AssertionError):
    """Dolt could not bind the selected port; the child log is the evidence."""


def fuzz_seeds_from_args(fuzz: bool, seeds: list[int] | None) -> tuple[int, ...]:
    """Validate CLI fuzz options and return the seeds the run will use."""
    if seeds and (not fuzz or len(seeds) > 2):
        raise ValueError("--fuzz-seed requires --fuzz and accepts at most two seeds")
    if not fuzz:
        return ()
    return tuple(seeds) if seeds else DEFAULT_FUZZ_SEEDS


def expand_unrun_results(
    results: list[tuple[str, str, str]], expected_scenarios: tuple[str, ...]
) -> list[tuple[str, str, str]]:
    """Add a stable NOT RUN row for each expected scenario without a result."""
    completed = {name for name, _, _ in results}
    return [*results, *((name, "NOT RUN", "A prior scenario failed")
                         for name in expected_scenarios if name not in completed)]


def select_server_port(configured: str | None) -> int:
    """Select a candidate; Dolt must bind it itself and can reject a collision."""
    if configured is None:
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
    else:
        try:
            port = int(configured)
        except ValueError as error:
            raise AssertionError("DOLT_PORT_INVALID: expected a numeric port") from error
    if not 1024 <= port <= 65535 or port == 3308:
        raise AssertionError(f"DOLT_PORT_INVALID: refusing port {port}")
    return port


def raise_server_failure(log: Path) -> None:
    output = log.read_text(errors="replace")
    message = output.lower()
    if "address already in use" in message or ("port " in message and "already in use" in message):
        raise ServerBindError(f"DOLT_SERVER_BIND_FAILED: {output}")
    raise AssertionError(f"DOLT_SERVER_START_FAILED: {output}")


def assert_server_running(server: subprocess.Popen[bytes], log: Path) -> None:
    if server.poll() is not None:
        raise_server_failure(log)


def read_server_output(stream, log, port: int, events: queue.Queue) -> None:
    """Drain the owned child and signal only its configured port's ready line."""
    configured = False
    signalled = False
    try:
        for line in iter(stream.readline, b""):
            log.write(line)
            log.flush()
            if b"Starting server with Config" in line and f"127.0.0.1:{port}".encode() in line:
                configured = True
            if b"Server ready. Accepting connections" in line and not signalled:
                events.put("ready" if configured else "wrong-port")
                signalled = True
        if not signalled:
            events.put("eof")
    except Exception as error:
        events.put(f"reader-error: {error}")


def wait_for_server(events: queue.Queue, server: subprocess.Popen[bytes], log: Path) -> None:
    try:
        event = events.get(timeout=30)
    except queue.Empty as error:
        raise AssertionError(f"DOLT_SERVER_NOT_READY: {log.read_text(errors='replace')}") from error
    if event == "eof":
        raise_server_failure(log)
    if event != "ready":
        raise AssertionError(f"DOLT_SERVER_NOT_READY ({event}): {log.read_text(errors='replace')}")
    assert_server_running(server, log)


def join_server_reader(reader: threading.Thread) -> None:
    reader.join(timeout=15)
    if reader.is_alive():
        raise AssertionError("DOLT_SERVER_READER_NOT_STOPPED")


def run(args: list[str], *, cwd: Path, env: dict[str, str], expected: int = 0) -> subprocess.CompletedProcess[str]:
    try:
        result = subprocess.run(args, cwd=cwd, env=env, text=True, capture_output=True, timeout=TIMEOUT, check=False)
    except subprocess.TimeoutExpired as error:
        raise AssertionError(f"command timed out: {args!r}\nstdout: {error.stdout!r}\nstderr: {error.stderr!r}") from error
    if result.returncode != expected:
        raise AssertionError(f"command {args!r}: expected exit {expected}, got {result.returncode}\nstdout: {result.stdout}\nstderr: {result.stderr}")
    return result


def server_processes() -> set[str]:
    if os.name == "nt":
        result = subprocess.run(["powershell", "-NoProfile", "-Command", "Get-CimInstance Win32_Process | Where-Object { (($_.Name -eq 'dolt.exe') -or ($_.Name -eq 'dolt')) -and $_.CommandLine -match 'sql-server' } | ForEach-Object { \"$($_.ProcessId) $($_.CommandLine)\" }"], capture_output=True, text=True, timeout=10, check=False)
        if result.returncode:
            raise AssertionError(f"process listing failed: {result.stderr}")
        return set(result.stdout.splitlines())
    result = subprocess.run(["pgrep", "-fl", "dolt"], capture_output=True, text=True, timeout=10, check=False)
    if result.returncode not in (0, 1):
        raise AssertionError(f"pgrep failed: {result.stderr}")
    servers = set()
    for line in result.stdout.splitlines():
        _, _, command = line.strip().partition(" ")
        try:
            args = shlex.split(command)
        except ValueError:
            continue
        if len(args) >= 2 and Path(args[0]).name == "dolt" and args[1] == "sql-server":
            servers.add(line)
    return servers


def processes_referencing(root: Path, cwd: Path, env: dict[str, str]) -> list[str]:
    if os.name == "nt":
        command = ["powershell", "-NoProfile", "-Command", "Get-CimInstance Win32_Process | ForEach-Object { \"$($_.ProcessId) $($_.CommandLine)\" }"]
    else:
        command = ["ps", "-axo", "pid=,command="]
    processes = run(command, cwd=cwd, env=env).stdout
    return [line for line in processes.splitlines() if str(root) in line]


def stop_server(server: subprocess.Popen[bytes]) -> None:
    if server.poll() is not None:
        return
    server.terminate()
    try:
        server.wait(timeout=15)
    except subprocess.TimeoutExpired:
        server.kill()
        server.wait(timeout=15)


def cleanup_root(root: Path, servers: list[subprocess.Popen[bytes]]) -> None:
    owned = processes_referencing(root, root, dict(os.environ))
    owned.extend(f"owned Dolt server pid {server.pid}" for server in servers if server.poll() is None)
    if owned:
        raise AssertionError(f"refusing cleanup with live processes: {owned}; retained {root}")
    shutil.rmtree(root)


def xml_escape(value: str, *, quote: bool = False) -> str:
    valid = "".join(char for char in value if ord(char) in (9, 10, 13) or 0x20 <= ord(char) <= 0xD7FF or 0xE000 <= ord(char) <= 0xFFFD or 0x10000 <= ord(char) <= 0x10FFFF)
    return html.escape(valid, quote=quote)


def installation_markup(installation: dict | None = None) -> str:
    if installation is None:
        installation = json.loads(os.environ.get("SC_COMPOSE_TEST_INSTALLATION", "{}"))
    installed = installation.get("installed") is True
    executable = shutil.which("bd")
    source = installation.get("source") or (str(Path(executable).resolve()) if executable else "not recorded")
    outcome = "installed (per-run temporary staging)" if installed else "not installed (existing executable)"
    return f'<p data-installed="{str(installed).lower()}" data-source="{xml_escape(source, quote=True)}">Installation: {outcome}<br/>Source: {xml_escape(source)}</p>'


def report_sections(previous: Path) -> dict[str, str]:
    sections: dict[str, str] = {}
    if previous.exists():
        for section in ET.parse(previous).getroot().iter("{http://www.w3.org/1999/xhtml}section"):
            original = section.get("data-key")
            if original:
                key = f"local|{original}" if len(original.split("|")) == 3 else original
                section.set("data-key", key)
                # Prefer a recorded target-prefixed panel if both forms exist,
                # regardless of their order in the historical file.
                if key not in sections or original == key:
                    sections[key] = ET.tostring(section, encoding="unicode")
    return sections


def write_report(version: str, digest: str, dolt_version: str, results: list[tuple[str, str, str]], *, profile: str = "integration", mode: str = "server", target: str = "local", installation: dict | None = None) -> Path:
    REPORTS.mkdir(parents=True, exist_ok=True)
    name = f"{RUN_DATE:%Y%m%d}-repour-{profile}.xhtml"
    report = REPORTS / name
    legacy_name = name.removesuffix(".xhtml") + ".html"
    previous = report if report.exists() else REPORTS / legacy_name
    esc = xml_escape
    rows = "\n".join(f"<tr><td>{esc(name)}</td><td>{esc(status)}</td><td><pre>{esc(detail)}</pre></td></tr>" for name, status, detail in results)
    key = f"{target}|{platform.system()}-{platform.machine()}|{version}|{mode}"
    sections = report_sections(previous)
    sections[key] = f'''<section data-key="{esc(key, quote=True)}"><h2>{esc(platform.system())} {esc(platform.machine())} — {esc(target)} {esc(mode)} mode</h2><p>bd: {esc(version)}<br/>SHA-256: {esc(digest)}<br/>Dolt: {esc(dolt_version)}</p>
{installation_markup(installation)}
<table><thead><tr><th>Scenario</th><th>Result</th><th>Evidence / failure</th></tr></thead><tbody>{rows}</tbody></table></section>'''
    panels = "\n".join(sections.values())
    report.write_text(f'''<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en"><head><meta charset="utf-8"/><title>Repour {profile}</title></head><body>
<h1>Repour {profile}</h1><p>{esc(INSTALL_MEANING)}</p>{panels}</body></html>\n''', encoding="utf-8")
    index = REPORTS / "index.html"
    text = index.read_text(encoding="utf-8")
    entry = f'<li><a href="{name}">{name} — local repour {profile}</a></li>'
    legacy_entry = f'<li><a href="{legacy_name}">{legacy_name} — local repour {profile}</a></li>'
    text = text.replace(legacy_entry, entry)
    (REPORTS / legacy_name).unlink(missing_ok=True)
    if entry not in text:
        text = text.replace("    <ul>\n", "    <ul>\n      " + entry + "\n", 1)
    index.write_text(text, encoding="utf-8")
    return report


def database_scenario(mode: str) -> str:
    return "embedded unknown-state refusal and explicit override" if mode == "embedded" else "clean apply, dirty refusal and explicit override"


def assert_embedded_unknown(error: dict) -> None:
    assert error["code"] == "BEADS_GRAPH_DIRTY_UNKNOWN", error
    assert error["details"]["cause_code"] == "BEADS_GRAPH_READ_FAILED", error
    assert "not yet supported in embedded mode" in error["details"]["cause"], error


def initialization_args(mode: str, port: int | None) -> list[str]:
    args = ["init", "--non-interactive", "--quiet", "--skip-agents", "--skip-hooks", "--prefix", "it"]
    if mode == "server":
        if port is None or port == 3308:
            raise AssertionError("server mode requires an isolated port")
        args.extend(["--server", "--server-port", str(port), "--database", "integration"])
    elif mode != "embedded":
        raise AssertionError(f"unsupported storage mode: {mode}")
    return args


def main(*, fuzz_seeds: tuple[int, ...] = (), mode: str = "server") -> int:
    results: list[tuple[str, str, str]] = []
    version = digest = dolt_version = "unavailable"
    server: subprocess.Popen[bytes] | None = None
    owned_servers: list[subprocess.Popen[bytes]] = []
    scenario_name = "setup"
    expected_scenarios = (*(database_scenario(mode) if name == "clean apply, dirty refusal and explicit override" else name for name in SCENARIOS), *(f"fuzz seed {seed}" for seed in fuzz_seeds))
    try:
        bd_name, dolt_name = shutil.which("bd"), shutil.which("dolt")
        if not bd_name or not dolt_name:
            raise AssertionError("installed bd and dolt are required")
        bd, dolt = str(Path(bd_name).resolve()), str(Path(dolt_name).resolve())
        binary = ROOT / "target" / "debug" / ("sc-compose.exe" if os.name == "nt" else "sc-compose")
        if not binary.is_file():
            raise AssertionError(f"build sc-compose first: {binary}")
        existing_servers = server_processes()
        other_servers = {line for line in existing_servers if "shared-server" not in line}
        if other_servers:
            raise AssertionError(f"refusing integration while another Dolt server is running: {other_servers}")
        with ExitStack() as stack:
            root = Path(tempfile.mkdtemp(prefix="sc-repour-integration-")).resolve()
            stack.callback(cleanup_root, root, owned_servers)
            home, workspace, data = root / "home", root / "work", root / "data"
            for directory in (home, workspace, data):
                directory.mkdir()
            env = {key: value for key, value in os.environ.items() if not key.startswith(("BEADS_", "BD_DOLT_"))}
            env.update(HOME=str(home), USERPROFILE=str(home), XDG_CONFIG_HOME=str(home / ".config"), BEADS_DIR=str(workspace / ".beads"), BEADS_NO_DAEMON="1", DOLT_ROOT_PATH=str(home), DOLT_DISABLE_EVENT_FLUSH="1", BD_DISABLE_EVENT_FLUSH="1", BD_DISABLE_METRICS="1", BD_DOLT_AUTO_PUSH="false", BD_DOLT_LOCAL_ONLY="true")
            version = run([bd, "version"], cwd=workspace, env=env).stdout.strip()
            if "1.3.4" not in version:
                raise AssertionError(f"expected installed bd 1.3.4 or its fork, got {version}")
            digest = hashlib.sha256(Path(bd).read_bytes()).hexdigest()
            run([dolt, "config", "--global", "--add", "versioncheck.disabled", "true"], cwd=data, env=env)
            dolt_version = run([dolt, "version"], cwd=data, env=env).stdout.splitlines()[0].strip()
            run([dolt, "config", "--global", "--add", "user.name", "integration"], cwd=data, env=env)
            run([dolt, "config", "--global", "--add", "user.email", "integration@example.invalid"], cwd=data, env=env)
            port = None
            if mode == "server":
                port = select_server_port(env.get("SC_COMPOSE_TEST_DOLT_PORT"))
                env["SC_COMPOSE_TEST_DOLT_PORT"] = str(port)
                server_log = stack.enter_context((root / "server.log").open("wb"))
                server = subprocess.Popen([dolt, "sql-server", "--host", "127.0.0.1", "--port", str(port)], cwd=data, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                owned_servers.append(server)
                assert server.stdout is not None
                events: queue.Queue = queue.Queue()
                reader = threading.Thread(target=read_server_output, args=(server.stdout, server_log, port, events), daemon=True)
                reader.start()
                stack.callback(server.stdout.close)
                stack.callback(join_server_reader, reader)
                stack.callback(stop_server, server)
                wait_for_server(events, server, root / "server.log")
            def bd_run(*args: str) -> str:
                return run([bd, *args], cwd=workspace, env=env).stdout
            bd_run(*initialization_args(mode, port))
            def scenario(name: str, action):
                nonlocal scenario_name
                scenario_name = name
                detail = action()
                results.append((name, "PASS", detail or ""))
            def cli(operation: str, path: Path, *args: str, expected: int = 0) -> dict:
                result = run([str(binary), "bead", operation, str(path), *args, "--json"], cwd=workspace, env=env, expected=expected)
                return json.loads(result.stdout)
            def show(bead: str) -> dict:
                value = json.loads(bd_run("show", bead, "--json"))
                return value[0]
            def snapshot() -> dict:
                rows = json.loads(bd_run("list", "--all", "-n", "0", "--json"))
                return {row["id"]: {"row": show(row["id"]), "deps": json.loads(bd_run("dep", "list", row["id"], "--json"))} for row in rows}
            def formula(path: Path, steps: str):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('formula = "release"\nversion = 1\ntype = "workflow"\n' + steps, encoding="utf-8")
            a, b = workspace / "v1" / "release.formula.toml.j2", workspace / "v2" / "release.formula.toml.j2"
            formula(a, '[[steps]]\nid = "build"\ntitle = "Build {{literal}}"\n[[steps]]\nid = "verify"\ntitle = "Verify"\nneeds = ["build"]\n[[steps]]\nid = "publish"\ntitle = "Publish"\nneeds = ["verify"]\n')
            formula(b, '[[steps]]\nid = "build"\ntitle = "Build {{literal}}"\n[[steps]]\nid = "verify"\ntitle = "Verify revised"\nneeds = ["build"]\n[[steps]]\nid = "notify"\ntitle = "Notify"\nneeds = ["verify"]\n')
            state: dict = {}
            def pour():
                payload = cli("pour", a, "--var", "literal=v1")["payload"]
                state.update(root=payload["graph"]["parent"], ids=payload["graph"]["ids"])
                assert show(state["ids"]["build"])["title"] == "Build v1"
                assert len(snapshot()) >= 4
            scenario("pour", pour)
            root_id, ids = state["root"], state["ids"]
            def conflict():
                before = snapshot()
                out = cli("pour", b, "--var", "literal=v1", "--root", root_id, expected=2)
                assert out["payload"]["error"]["code"] == "BEADS_GRAPH_CONFLICT", out
                assert out["payload"]["error"]["details"].get("changes"), out
                assert snapshot() == before
            scenario("changed pour refuses with changes", conflict)
            def runtime():
                bd_run("update", ids["build"], "--status", "in_progress", "--assignee", "owner", "--notes", "keep", "--add-label", "user")
                bd_run("update", ids["publish"], "--assignee", "closed-owner", "--notes", "closed note", "--add-label", "closed-user")
                bd_run("close", ids["publish"], "--reason", "finished", "--force")
                assert show(ids["build"])["status"] == "in_progress"
                assert show(ids["publish"])["status"] == "closed"
            scenario("set runtime state", runtime)
            def revision():
                before = snapshot()
                preview = cli("repour", b, "--var", "literal=v1", "--root", root_id, "--dry-run", "--allow-uncommitted")["payload"]["graph"]
                assert snapshot() == before
                applied = cli("repour", b, "--var", "literal=v1", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                assert preview["changes"] == applied["changes"], (preview, applied)
                build, old = show(ids["build"]), show(ids["publish"])
                assert build["status"] == "in_progress" and build["assignee"] == "owner" and build["notes"] == "keep" and "user" in build["labels"]
                assert old["status"] == "closed" and old["assignee"] == "closed-owner" and old["notes"] == "closed note" and "closed-user" in old["labels"] and "repour:removed" in old["labels"]
                assert show(ids["verify"])["title"] == "Verify revised"
                state["notify"] = applied["ids"]["notify"]
                snapshot()
            scenario("preview, apply, retained runtime and remove/add", revision)
            def no_op():
                before = snapshot()
                graph = cli("repour", b, "--var", "literal=v1", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                assert snapshot() == before
                assert any(change["change"] == "mark_removed_node" and change["bead"] == ids["publish"] and change["remove_command"] for change in graph["changes"])
            scenario("repeated repour changes no stored state", no_op)
            def restore():
                before_id = ids["publish"]
                result = cli("repour", a, "--var", "literal=v1", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                assert result["ids"]["publish"] == before_id
                restored = show(before_id)
                assert restored["status"] == "closed" and restored["assignee"] == "closed-owner" and restored["notes"] == "closed note" and "closed-user" in restored["labels"] and "repour:removed" not in restored["labels"]
                snapshot()
            scenario("restore removed node", restore)
            def external_edges():
                gate = json.loads(bd_run("create", "External gate", "--type", "task", "--json"))["id"]
                outside = json.loads(bd_run("create", "External task", "--type", "task", "--json"))["id"]
                bd_run("dep", "add", gate, ids["build"])
                bd_run("dep", "add", ids["publish"], outside)
                before = snapshot()
                graph = cli("repour", a, "--var", "literal=v1", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                after = snapshot()
                assert any(d["id"] == ids["build"] for d in after[gate]["deps"])
                assert any(d["id"] == outside for d in after[ids["publish"]]["deps"])
                assert sum(c["change"] == "retain_external_edge" for c in graph["changes"]) >= 2
                assert before == after
            scenario("external edges retained in both directions", external_edges)
            conditional = workspace / "conditional" / "release.formula.toml.j2"
            formula(conditional, '[[steps]]\nid = "build"\ntitle = "Build {{literal}}"\n[[steps]]\nid = "verify"\ntitle = "Verify"\n{% if extra == "yes" %}\n[[steps]]\nid = "optional"\ntitle = "Optional"\nneeds = ["build"]\n{% endif %}\n')
            def condition_and_rename():
                graph = cli("repour", conditional, "--var", "literal=v1", "--var", "extra=yes", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                optional = graph["ids"]["optional"]
                after = snapshot()
                assert after[optional]["row"]["title"] == "Optional"
                assert any(d["id"] == ids["build"] for d in after[optional]["deps"])
                assert "repour:removed" in after[ids["publish"]]["row"]["labels"]
                before = after
                preview = cli("repour", conditional, "--var", "literal=v1", "--var", "extra=no", "--root", root_id, "--dry-run", "--allow-uncommitted")["payload"]["graph"]
                assert snapshot() == before
                applied = cli("repour", conditional, "--var", "literal=v1", "--var", "extra=no", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                assert preview["changes"] == applied["changes"]
                assert "repour:removed" in show(optional)["labels"]
                assert not any(d["id"] == ids["build"] for d in snapshot()[optional]["deps"])
            scenario("conditional add and remove with renamed step", condition_and_rename)
            def dangling():
                bad = workspace / "dangling" / "release.formula.toml.j2"
                formula(bad, '[[steps]]\nid = "build"\ntitle = "Build"\nneeds = ["missing"]\n')
                before = snapshot()
                out = cli("repour", bad, "--root", root_id, "--allow-uncommitted", expected=2)
                assert out["payload"]["outcome"] != "succeeded", out
                assert snapshot() == before
            scenario("dangling reference refuses before writes", dangling)
            def dirty():
                bd_run("dolt", "commit", "-m", "integration baseline")
                if mode == "embedded":
                    before = snapshot()
                    unknown = cli("repour", a, "--var", "literal=v1", "--root", root_id, expected=2)
                    assert_embedded_unknown(unknown["payload"]["error"])
                    assert snapshot() == before
                else:
                    clean = cli("repour", a, "--var", "literal=v1", "--root", root_id)
                    assert clean["payload"]["outcome"] == "succeeded"
                bd_run("--dolt-auto-commit=off", "update", ids["build"], "--notes", "dirty")
                if mode == "server":
                    assert json.loads(bd_run("sql", "--readonly", "SELECT table_name, staged, status FROM dolt_status", "--json"))
                before = snapshot()
                refused = cli("repour", b, "--var", "literal=v1", "--root", root_id, expected=2)
                if mode == "embedded":
                    assert_embedded_unknown(refused["payload"]["error"])
                else:
                    assert refused["payload"]["error"]["code"] == "BEADS_GRAPH_DIRTY", refused
                assert snapshot() == before
                changed = cli("repour", b, "--var", "literal=v1", "--root", root_id, "--allow-uncommitted")
                assert changed["payload"]["outcome"] == "succeeded"
                assert show(ids["build"])["notes"] == "dirty"
                assert show(ids["verify"])["title"] == "Verify revised"
                snapshot()
                if mode == "embedded":
                    return "Stock bd 1.3.4 has no embedded bd sql: exact unknown-state refusal and no-write behavior verified before and after an uncommitted edit; explicit override succeeds. Clean/dirty detection is not supported or claimed."
            scenario(database_scenario(mode), dirty)
            def attach_request():
                example = ROOT / "examples" / "beads" / "release-under-epic"
                template = workspace / "attached.formula.toml.j2"
                shutil.copyfile(example / "release.formula.toml.j2", template)
                req = json.loads((example / "request.json").read_text())
                parent = json.loads(bd_run("create", "Attached parent", "--type", "epic", "--json"))["id"]
                req.update(working_directory=str(workspace), template=str(template), rendered_formula=str(workspace / "attached.formula.toml"), parent=parent, bd_executable=bd)
                request_path = workspace / "attach-request.json"
                request_path.write_text(json.dumps(req))
                def call(operation: str, *flags: str) -> dict:
                    return json.loads(run([str(binary), "bead", operation, "--request", str(request_path), "--render-by", "bd", *flags, "--json"], cwd=workspace, env=env).stdout)
                created = call("attach")["payload"]["graph"]
                build = created["ids"]["build"]
                bd_run("update", build, "--status", "in_progress", "--notes", "attached runtime")
                req["compose_variables"] = {"version": "1.6.2"}
                request_path.write_text(json.dumps(req))
                changed = call("repour", "--allow-uncommitted")["payload"]["graph"]
                assert changed["ids"]["build"] == build
                row = show(build)
                assert row["title"] == "Build 1.6.2" and row["status"] == "in_progress" and row["notes"] == "attached runtime"
                snapshot()
            scenario("attach and repour via request using bd rendering", attach_request)
            def partial_failure():
                partial = workspace / "partial" / "release.formula.toml.j2"
                formula(partial, '[[steps]]\nid = "build"\ntitle = "Build v1"\n[[steps]]\nid = "verify"\ntitle = "Verify partial"\nneeds = ["build"]\n[[steps]]\nid = "notify"\ntitle = "Notify"\nneeds = ["verify"]\n[[steps]]\nid = "urgent"\ntitle = "Urgent"\nneeds = ["verify"]\n')
                wrapper_py = workspace / "failing_bd.py"
                counter = workspace / "mutation-count"
                wrapper_py.write_text("#!/usr/bin/env python3\n" +
                    "import os, pathlib, sys\n" +
                    f"real={bd!r}; counter=pathlib.Path({str(counter)!r})\n" +
                    "args=sys.argv[1:]\n" +
                    "mutating=bool(args) and (args[0] == 'update' or (args[0] == 'create' and '--dry-run' not in args) or args[:2] in (['dep','add'],['dep','remove']))\n" +
                    "count=int(counter.read_text()) if counter.exists() else 0\n" +
                    "count += int(mutating)\n" +
                    "counter.write_text(str(count)) if mutating else None\n" +
                    "sys.exit('injected second write failure') if mutating and count == 2 else None\n" +
                    "os.execv(real, [real, *args])\n")
                wrapper_py.chmod(0o755)
                wrapper = wrapper_py
                if os.name == "nt":
                    wrapper = workspace / "failing_bd.cmd"
                    wrapper.write_text(f'@echo off\r\n"{sys.executable}" "{wrapper_py}" %*\r\nexit /b %errorlevel%\r\n')
                request = {"schema": "sc-compose/beads/v1", "operation": "repour", "working_directory": str(workspace), "template": str(partial), "rendered_formula": str(workspace / "partial.formula.toml"), "compose_variables": {}, "formula_name": "release", "bead_variables": {}, "bd_executable": str(wrapper), "pour_authorization": "CreatePersistentBeads", "relations": []}
                request_path = workspace / "partial-request.json"
                request_path.write_text(json.dumps(request))
                before = snapshot()
                proc = run([str(binary), "bead", "repour", "--request", str(request_path), "--root", root_id, "--allow-uncommitted", "--json"], cwd=workspace, env=env, expected=2)
                payload = json.loads(proc.stdout)["payload"]
                assert payload["error"]["code"] == "BEADS_GRAPH_APPLY_PARTIAL", payload
                details = payload["error"]["details"]
                assert details["applied"] == [
                    {"change": "add_node", "step": "urgent"},
                    {"change": "add_edge", "from": "step:urgent", "to": ids["verify"], "kind": "blocks"},
                ], details
                assert details["pending"] == [{
                    "change": "field_change", "bead": ids["verify"], "status": "open",
                    "field": "title", "before": "Verify revised", "after": "Verify partial",
                }], details
                after = snapshot()
                assert len(after) == len(before) + 1 and after[ids["verify"]]["row"]["title"] == "Verify revised"
                resumed = cli("repour", partial, "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                assert show(ids["verify"])["title"] == "Verify partial"
                assert resumed["ids"]["urgent"] in snapshot()
            scenario("partial apply reports exact progress and resumes", partial_failure)
            def sequential_revisions():
                graph = cli("repour", a, "--var", "literal=v1", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                assert graph["ids"]["publish"] == ids["publish"] and "repour:removed" not in show(ids["publish"]).get("labels", [])
                snapshot()
                graph = cli("repour", b, "--var", "literal=v1", "--root", root_id, "--allow-uncommitted")["payload"]["graph"]
                assert graph["ids"]["notify"] == state["notify"] and "repour:removed" not in show(state["notify"]).get("labels", [])
                assert show(ids["build"])["status"] == "in_progress"
                snapshot()
            scenario("sequential evolving revisions preserve identities", sequential_revisions)
            if fuzz_seeds:
                from integration_repour_fuzz import run_seed
                results.append(("fuzz executable", "PASS", f"sc-compose SHA-256: {hashlib.sha256(binary.read_bytes()).hexdigest()}"))
                for seed in fuzz_seeds:
                    scenario(f"fuzz seed {seed}", lambda seed=seed: run_seed(seed, workspace, cli, bd_run, show, snapshot))
            scenario_name = "teardown"
            if server is not None:
                stop_server(server)
                server = None
            owned = processes_referencing(root, workspace, env)
            assert not owned, f"processes still reference temporary workspace: {owned}"
        results.append(("teardown", "PASS", ""))
    except Exception:
        results.append((scenario_name, "FAIL", traceback.format_exc()))
        results = expand_unrun_results(results, expected_scenarios)
    finally:
        if server is not None:
            stop_server(server)
        report = write_report(version, digest, dolt_version, results, profile="fuzz" if fuzz_seeds else "integration", mode=mode, target=os.environ.get("SC_COMPOSE_TEST_TARGET", "local"))
        print(f"integration report: {report}")
        for name, status, detail in results:
            print(f"{status}: {name}" + (f": {detail}" if detail else ""))
    return int(any(status != "PASS" for _, status, _ in results))


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("server", "embedded"), default="server", help="Isolated storage mode; never installs executables")
    parser.add_argument("--fuzz", action="store_true", help="After integration passes, run two fixed small seeded cases in the same database")
    parser.add_argument("--fuzz-seed", type=int, action="append", help="Replay a seed (at most two, requires --fuzz)")
    args = parser.parse_args()
    try:
        fuzz_seeds = fuzz_seeds_from_args(args.fuzz, args.fuzz_seed)
    except ValueError as error:
        parser.error(str(error))
    raise SystemExit(main(fuzz_seeds=fuzz_seeds, mode=args.mode))
