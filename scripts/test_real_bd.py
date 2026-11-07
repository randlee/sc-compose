#!/usr/bin/env python3
"""Run the real-bd integration tests as an explicit qualification run.

The real-bd tests are `#[ignore]`d, so a plain `cargo test` reports them as
ignored instead of passing without executing bd. This script is the opt-in:
it requires a pinned bd (BD_EXECUTABLE), records its provenance, runs the
ignored tests serially, and fails unless real tests actually executed.

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
from pathlib import Path

# (label, cargo test arguments before `--`)
SUITES = (
    ("sc-composer-beads/graph_bd", ["-p", "sc-composer-beads", "--test", "graph_bd"]),
    ("sc-composer-beads/bd_integration", ["-p", "sc-composer-beads", "--test", "bd_integration"]),
    ("sc-compose/cli bead", ["-p", "sc-compose", "--test", "cli", "bead"]),
)
RESULT = re.compile(
    r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored"
)


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


def main() -> int:
    bd = resolve_bd(dict(os.environ))
    if isinstance(bd, str):
        print(f"real-bd qualification NOT RUN: {bd}", file=sys.stderr)
        return 2
    version = subprocess.run(
        [str(bd), "version"], capture_output=True, text=True, check=False
    ).stdout.strip()
    provenance = {
        "bd_path": str(bd),
        "bd_sha256": hashlib.sha256(bd.read_bytes()).hexdigest(),
        "bd_version": version,
    }
    print(json.dumps(provenance))
    status = 0
    for label, args in SUITES:
        proc = subprocess.run(
            ["cargo", "test", "-j2", *args, "--", "--ignored", "--test-threads=1"],
            capture_output=True,
            text=True,
            check=False,
        )
        sys.stdout.write(proc.stdout)
        sys.stderr.write(proc.stderr)
        counts = parse_counts(proc.stdout)
        reason = verdict(proc.returncode, counts)
        print(
            json.dumps(
                {"suite": label, "passed": counts[0], "failed": counts[1],
                 "exit": proc.returncode, "ok": reason is None}
            )
        )
        if reason:
            print(f"real-bd suite {label} FAILED: {reason}", file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main())
