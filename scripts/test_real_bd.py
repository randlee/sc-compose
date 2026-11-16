#!/usr/bin/env python3
"""Run the real-bd integration tests as an explicit qualification run.

The real-bd tests are `#[ignore]`d, so a plain `cargo test` reports them as
ignored instead of passing without executing bd. This script is the opt-in:
it requires a pinned bd (BD_EXECUTABLE), records its provenance, runs the
ignored tests serially, and fails unless real tests actually executed.

Test binaries are compiled before execution. Each ignored test then runs in its
own process with a hang-guard timeout, so suite length and runner load do not
share one wall-clock budget.

It does not claim atomic-ownership qualification; it only proves these tests
ran against the identified executable.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# (label, Cargo target arguments, optional test-name filter)
SUITES = (
    ("sc-composer-beads/graph_bd", ["-p", "sc-composer-beads", "--test", "graph_bd"], None),
    ("sc-composer-beads/bd_integration", ["-p", "sc-composer-beads", "--test", "bd_integration"], None),
    ("sc-compose/cli bead", ["-p", "sc-compose", "--test", "cli"], "bead"),
)
RESULT = re.compile(
    r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored"
)
PROCESS_TIMEOUT_SECONDS = 600


def timeout_diagnostic(error: subprocess.TimeoutExpired) -> None:
    """Report captured output when the failure-only process deadline expires."""
    print(f"real-bd process timed out: {error.cmd!r}", file=sys.stderr)
    for label, output in (("stdout", error.stdout), ("stderr", error.stderr)):
        if output:
            if isinstance(output, bytes):
                output = output.decode("utf-8", errors="replace")
            print(f"{label}: {output}", file=sys.stderr)


def parse_counts(output: str) -> tuple[int, int, int]:
    """Sum (passed, failed, ignored) over every `test result:` line."""
    totals = [0, 0, 0]
    for match in RESULT.finditer(output):
        for index in range(3):
            totals[index] += int(match.group(index + 1))
    return totals[0], totals[1], totals[2]


def verdict(returncode: int, counts: tuple[int, int, int]) -> str | None:
    """Return a failure reason, or None when real tests executed and passed."""
    passed, failed, _ = counts
    if returncode != 0 or failed:
        return f"exit {returncode}, {failed} failed"
    if passed == 0:
        return "no real-bd test executed (0 passed)"
    return None


def resolve_bd(environ: dict[str, str]) -> Path | str:
    """Return the pinned bd path, or an error message string."""
    raw = environ.get("BD_EXECUTABLE", "")
    if not raw:
        return "BD_EXECUTABLE is not set: real-bd qualification needs a pinned bd"
    path = Path(raw)
    if not path.is_file() or not os.access(path, os.X_OK):
        return f"BD_EXECUTABLE is not an executable file: {raw}"
    return path


def isolated_environment(environ: dict[str, str], home: Path) -> dict[str, str]:
    """Keep every qualification child away from the operator's bd home."""
    result = environ.copy()
    original_home = Path(environ.get("HOME", str(Path.home())))
    result.setdefault("CARGO_HOME", str(original_home / ".cargo"))
    result.setdefault("RUSTUP_HOME", str(original_home / ".rustup"))
    result.update({"HOME": str(home), "USERPROFILE": str(home), "XDG_CONFIG_HOME": str(home / ".config")})
    # The fixtures select a distinct BEADS_DIR per workspace. An ambient value
    # must not override that selection or leak into child processes.
    result.pop("BEADS_DIR", None)
    return result


def database_mode(environ: dict[str, str]) -> str:
    return "server" if "SC_COMPOSE_TEST_DOLT_PORT" in environ else "embedded"


def selected_suites(database_mode: bool) -> tuple[tuple[str, list[str], str | None], ...]:
    if database_mode:
        return (("database-mode", ["-p", "sc-composer-beads", "--test", "graph_bd"], "repour_clean_dirty_database_mode"),)
    return SUITES


def executable_from_build(output: str, target_name: str) -> str | None:
    """Find the built test executable for one Cargo test target."""
    for line in output.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = event.get("target", {})
        if (
            event.get("reason") == "compiler-artifact"
            and target.get("name") == target_name
            and target.get("test") is True
            and event.get("executable")
        ):
            return event["executable"]
    return None


def ignored_test_names(output: str) -> list[str]:
    """Parse the `--list --ignored` output of a Rust test binary."""
    names = []
    for line in output.splitlines():
        name, separator, kind = line.partition(": ")
        if separator and kind == "test":
            names.append(name)
    return names


def build_diagnostics(output: str) -> str:
    """Extract human-readable compiler messages from Cargo's JSON stream."""
    diagnostics = []
    for line in output.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            if line.strip():
                diagnostics.append(line)
            continue
        if event.get("reason") == "compiler-message":
            rendered = event.get("message", {}).get("rendered")
            if rendered:
                diagnostics.append(rendered)
    return "".join(diagnostics)


def prepare_suites(
    env: dict[str, str], database_mode: bool = False
) -> list[tuple[str, str, list[str]]]:
    """Build every target, then discover ignored tests before running any."""
    binaries = []
    for label, args, test_filter in selected_suites(database_mode):
        target_name = args[args.index("--test") + 1]
        command = ["cargo", "test", "--no-run", "--message-format=json", "-j2", *args]
        if test_filter:
            command.append(test_filter)
        build = subprocess.run(command, capture_output=True, text=True, check=False, env=env)
        sys.stderr.write(build.stderr)
        executable = executable_from_build(build.stdout, target_name)
        if build.returncode != 0 or executable is None:
            sys.stderr.write(build_diagnostics(build.stdout))
            print(f"real-bd suite {label} build FAILED: cargo test --no-run exit {build.returncode}", file=sys.stderr)
            binaries.append((label, "", test_filter))
            continue
        binaries.append((label, executable, test_filter))

    prepared = []
    for label, executable, test_filter in binaries:
        if not executable:
            prepared.append((label, "", []))
            continue
        list_command = [executable]
        if test_filter:
            list_command.append(test_filter)
        list_command.extend(["--list", "--ignored"])
        listed = subprocess.run(list_command, capture_output=True, text=True, check=False, env=env)
        names = ignored_test_names(listed.stdout) if listed.returncode == 0 else []
        if listed.returncode != 0 or not names:
            sys.stderr.write(listed.stderr)
            sys.stderr.write(listed.stdout)
            print(f"real-bd suite {label} discovery FAILED: --list --ignored exit {listed.returncode}; found {len(names)} tests", file=sys.stderr)
            prepared.append((label, executable, []))
            continue
        if not database_mode and label == "sc-composer-beads/graph_bd":
            names = [name for name in names if name != "repour_clean_dirty_database_mode"]
        prepared.append((label, executable, names))
    return prepared


def main() -> int:
    mode = sys.argv[1:]
    if mode not in ([], ["--database-mode"]):
        print("usage: test_real_bd.py [--database-mode]", file=sys.stderr)
        return 2
    bd = resolve_bd(dict(os.environ))
    if isinstance(bd, str):
        print(f"real-bd qualification NOT RUN: {bd}", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory(prefix="sc-compose-bd-home-") as directory:
        env = isolated_environment(dict(os.environ), Path(directory))
        version_env = dict(env, BEADS_DIR=str(Path(directory) / ".beads"))
        try:
            version_result = subprocess.run(
                [str(bd), "version"], capture_output=True, text=True, check=False, env=version_env,
                timeout=PROCESS_TIMEOUT_SECONDS,
            )
        except subprocess.TimeoutExpired as error:
            timeout_diagnostic(error)
            return 1
        version = version_result.stdout.strip()
        if version_result.returncode != 0 or not version:
            print("real-bd qualification FAILED: bd version probe failed or returned empty output", file=sys.stderr)
            return 1
        provenance = {
            "bd_path": str(bd),
            "bd_sha256": hashlib.sha256(bd.read_bytes()).hexdigest(),
            "bd_version": version,
            "database_mode": database_mode(env),
        }
        print(json.dumps(provenance))
        return run_suites(env, provenance, database_mode=bool(mode))


def run_suites(env: dict[str, str], provenance: dict[str, str], database_mode: bool = False) -> int:
    status = 0
    suites = prepare_suites(env, database_mode)
    for label, executable, tests in suites:
        passed = failed = ignored = 0
        suite_exit = 0
        unsupported = False
        if not executable or not tests:
            failed = 1
            suite_exit = 1
        for test_name in tests:
            command = [executable, test_name, "--exact", "--ignored", "--test-threads=1", "--nocapture"]
            try:
                proc = subprocess.run(
                    command,
                    capture_output=True,
                    text=True,
                    check=False,
                    env=env,
                    timeout=PROCESS_TIMEOUT_SECONDS,
                )
            except subprocess.TimeoutExpired as error:
                print(f"real-bd test timed out: {label}::{test_name}", file=sys.stderr)
                timeout_diagnostic(error)
                failed += 1
                suite_exit = 1
                continue
            sys.stdout.write(proc.stdout)
            sys.stderr.write(proc.stderr)
            counts = parse_counts(proc.stdout)
            passed += counts[0]
            failed += counts[1]
            ignored += counts[2]
            reason = verdict(proc.returncode, counts)
            unsupported = unsupported or bool(re.search(
                r"(?m)^SC_COMPOSE_MODE_UNSUPPORTED: BEADS_GRAPH_DIRTY_UNKNOWN(?:\s|$)", proc.stdout
            ))
            if reason:
                print(f"real-bd test {label}::{test_name} FAILED: {reason}", file=sys.stderr)
                suite_exit = proc.returncode or 1
        counts = (passed, failed, ignored)
        reason = verdict(suite_exit, counts)
        print(
            json.dumps(
                {**provenance, "suite": label, "passed": counts[0], "failed": counts[1],
                 "exit": suite_exit, "ok": reason is None and not unsupported,
                 "qualification": "failed" if reason else ("not_qualified" if unsupported else "qualified")}
            )
        )
        if unsupported and reason is None:
            summary = os.environ.get("GITHUB_STEP_SUMMARY")
            if summary:
                with open(summary, "a", encoding="utf-8") as stream:
                    stream.write(f"bd {provenance['bd_version']}: {provenance['database_mode']} not_qualified (unsupported)\n")
        if reason:
            print(f"real-bd suite {label} FAILED: {reason}", file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main())
