#!/usr/bin/env python3
"""Generate or check the CLI reference with the external ``docli`` tool.

Thin wrapper only: the neutral model comes from ``sc-compose reference --json``
(``payload.model``) and all rendering is done by the pinned ``docli`` executable.
"""

from __future__ import annotations

import argparse
import filecmp
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_HTML = REPO_ROOT / "site" / "cli"
DEFAULT_MARKDOWN = REPO_ROOT / "crates" / "sc-compose" / "docs" / "reference" / "cli-reference.md"
EXPORT_COMMAND = ["cargo", "run", "--quiet", "--bin", "sc-compose", "--", "reference", "--json"]


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="write generated references")
    mode.add_argument("--check", action="store_true", help="fail if references differ")
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="write <dir>/html and <dir>/cli-reference.md instead of the production paths",
    )
    parser.add_argument("--model-json", type=Path, help="use this reference --json envelope instead of running the exporter")
    parser.add_argument("--docli", default="docli", help="docli executable (default: docli on PATH)")
    return parser.parse_args(argv)


def load_model(model_json: Path | None) -> dict:
    if model_json is not None:
        raw = model_json.read_text(encoding="utf-8")
    else:
        run = subprocess.run(EXPORT_COMMAND, cwd=REPO_ROOT, capture_output=True, text=True, check=False)
        if run.returncode != 0:
            raise SystemExit(f"reference export failed ({run.returncode}): {run.stderr.strip()}")
        raw = run.stdout
    try:
        return json.loads(raw)["payload"]["model"]
    except (KeyError, TypeError, json.JSONDecodeError) as err:
        raise SystemExit(f"reference output has no payload.model: {err!r}")


def generate(model: dict, docli: str, scratch: Path) -> tuple[Path, Path]:
    model_path = scratch / "model.json"
    model_path.write_text(json.dumps(model), encoding="utf-8")
    html, markdown = scratch / "html", scratch / "cli-reference.md"
    cmd = [docli, "generate", "--input", str(model_path), "--html", str(html), "--markdown", str(markdown)]
    try:
        run = subprocess.run(cmd, capture_output=True, text=True, check=False)
    except FileNotFoundError:
        raise SystemExit(f"docli not found ({docli}); install with: cargo install sc-docli --version 0.2.0 --locked")
    if run.returncode != 0:
        raise SystemExit(f"docli generate failed ({run.returncode}): {run.stderr.strip()}")
    return html, markdown


def tree_drift(expected: Path, actual: Path) -> list[str]:
    if not actual.is_dir():
        return [f"missing: {actual}"]
    drift = []
    for src in sorted(p for p in expected.rglob("*") if p.is_file()):
        dst = actual / src.relative_to(expected)
        if not dst.is_file() or not filecmp.cmp(src, dst, shallow=False):
            drift.append(f"stale: {dst}")
    for dst in sorted(p for p in actual.rglob("*") if p.is_file()):
        if not (expected / dst.relative_to(actual)).is_file():
            drift.append(f"unexpected: {dst}")
    return drift


def file_drift(expected: Path, actual: Path) -> list[str]:
    if not actual.is_file() or not filecmp.cmp(expected, actual, shallow=False):
        return [f"stale: {actual}"]
    return []


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.output_dir is not None:
        html_target, md_target = args.output_dir / "html", args.output_dir / "cli-reference.md"
    else:
        html_target, md_target = DEFAULT_HTML, DEFAULT_MARKDOWN
    model = load_model(args.model_json)
    with tempfile.TemporaryDirectory() as tmp:
        html, markdown = generate(model, args.docli, Path(tmp))
        if args.write:
            if html_target.exists():
                shutil.rmtree(html_target)
            shutil.copytree(html, html_target)
            md_target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(markdown, md_target)
            print(f"wrote {html_target} and {md_target}")
            return 0
        drift = tree_drift(html, html_target) + file_drift(markdown, md_target)
    if drift:
        print("CLI reference is out of date; run scripts/generate_cli_reference.py --write", file=sys.stderr)
        for line in drift:
            print(f"  {line}", file=sys.stderr)
        return 1
    print("CLI reference is current")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
