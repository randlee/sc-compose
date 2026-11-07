#!/usr/bin/env bash
# Release preflight: sc-docli CLI HTML must match the consumer generate script.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MANIFEST="${ROOT}/release/publish-preflight.toml"

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
docli = data.get("docli")
if not isinstance(docli, dict):
    print("release-preflight-docli: missing [docli] table in publish-preflight.toml", file=sys.stderr)
    sys.exit(1)

generate_script = docli.get("generate_script")
drift_paths = docli.get("drift_paths")
if not generate_script or not drift_paths:
    print("release-preflight-docli: docli.generate_script and docli.drift_paths are required", file=sys.stderr)
    sys.exit(1)
if not isinstance(drift_paths, list) or not all(isinstance(path, str) and path for path in drift_paths):
    print("release-preflight-docli: docli.drift_paths must be a non-empty string array", file=sys.stderr)
    sys.exit(1)

subprocess.run(f"bash {generate_script!r}", shell=True, cwd=root, check=True)
diff = subprocess.run(
    ["git", "diff", "--exit-code", "--", *drift_paths],
    cwd=root,
)
if diff.returncode != 0:
    sys.exit(diff.returncode)
status = subprocess.run(
    ["git", "status", "--porcelain", "--", *drift_paths],
    cwd=root,
    capture_output=True,
    text=True,
    check=True,
)
if status.stdout.strip():
    print(status.stdout, end="")
    sys.exit(1)

print("release-preflight-docli: OK")
PY
