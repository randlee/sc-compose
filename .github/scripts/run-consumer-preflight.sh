#!/usr/bin/env bash
# Run repository-specific checks declared in release/publish-preflight.toml.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MANIFEST="${ROOT}/release/publish-preflight.toml"

if [[ ! -f "$MANIFEST" ]]; then
  echo "run-consumer-preflight: no ${MANIFEST}; skipping" >&2
  exit 0
fi

cd "$ROOT"
python3 - "$MANIFEST" <<'PY'
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore[no-redef]

manifest_path = Path(sys.argv[1]).resolve()
root = manifest_path.parent.parent
data = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
checks = data.get("checks", [])
if not isinstance(checks, list):
    sys.exit("run-consumer-preflight: checks must be an array")
if not checks:
    sys.exit(0)
for entry in checks:
    if not isinstance(entry, dict):
        sys.exit("run-consumer-preflight: each check must be a table")
    check_id = entry.get("id", entry.get("name"))
    run = entry.get("run")
    if not isinstance(check_id, str) or not check_id.strip():
        sys.exit("run-consumer-preflight: check id is required")
    if not isinstance(run, str) or not run.strip():
        sys.exit(f"run-consumer-preflight: {check_id}: command is required")

failures: list[str] = []
for entry in checks:
    check_id = entry.get("id", entry.get("name"))
    run = entry.get("run")
    print(f"run-consumer-preflight: running {check_id} ({run})", flush=True)
    result = subprocess.run(run, shell=True, cwd=root)
    print(f"run-consumer-preflight: {check_id}: exit {result.returncode}", flush=True)
    if result.returncode != 0:
        failures.append(f"{check_id}: exit {result.returncode}")

if failures:
    for line in failures:
        print(f"run-consumer-preflight: FAIL {line}", file=sys.stderr)
    sys.exit(1)

print(f"run-consumer-preflight: OK ({len(checks)} check(s))")
PY
