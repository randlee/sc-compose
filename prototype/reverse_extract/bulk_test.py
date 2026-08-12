#!/usr/bin/env python3
"""Bulk test harness for reverse template variable extractor.

Scans rendered XML payloads, matches to known templates by root element,
runs extraction, and reports pass/fail/skip statistics.
"""

import json
import os
import re
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

# Add prototype to path BEFORE local imports
_PKG_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_PKG_DIR))

from reverse_extract import extract_variables

# Template registry: root_tag → template path
TEMPLATE_DIR = os.path.expanduser(
    "~/Documents/github/atm-core/.claude/skills/codex-orchestration"
)
SHARE_DIR = os.path.expanduser("~/.config/atm/share/atm-dev")

TEMPLATES = {
    "repo-qa-task": os.path.join(TEMPLATE_DIR, "qa-template.xml.j2"),
    "atm-task": os.path.join(TEMPLATE_DIR, "dev-template.xml.j2"),
    "atm-review": os.path.join(TEMPLATE_DIR, "review-template.xml.j2"),
}


def get_root_tag(filepath: str) -> str | None:
    """Extract the root XML element tag from a file."""
    try:
        with open(filepath, "rb") as f:
            head = f.read(4096)
        text = head.decode("utf-8", errors="replace")
        # Strip any leading non-XML content (ATM message headers etc.)
        m = re.search(r"<([\w:-]+)[>\s]", text)
        return m.group(1) if m else None
    except Exception:
        return None


def find_xml_files(directory: str, limit: int = 500) -> list[str]:
    """Find actual XML files by content sniffing the first non-whitespace char."""
    xml_files = []
    for entry in sorted(os.listdir(directory)):
        path = os.path.join(directory, entry)
        if not os.path.isfile(path):
            continue
        # Content-sniff: first non-whitespace must be '<'
        try:
            with open(path, "rb") as f:
                head = f.read(256).lstrip()
            if head and head[0] == ord("<"):
                xml_files.append(path)
        except Exception:
            pass
        if len(xml_files) >= limit:
            break
    return xml_files


def main():
    xml_files = find_xml_files(SHARE_DIR)
    print(f"Found {len(xml_files)} XML files in {SHARE_DIR}")

    stats = {"pass": 0, "fail": 0, "skip": 0, "error": 0}
    skip_reasons: dict[str, int] = defaultdict(int)
    failures: list[dict] = []

    for filepath in xml_files:
        filename = os.path.basename(filepath)
        root_tag = get_root_tag(filepath)

        if not root_tag:
            stats["skip"] += 1
            skip_reasons["no_root_tag"] += 1
            continue

        # As a quick test, only process repo-qa-task for now
        if root_tag != "repo-qa-task":
            stats["skip"] += 1
            skip_reasons[f"root={root_tag}"] += 1
            continue

        template = TEMPLATES.get(root_tag)
        if not template:
            stats["skip"] += 1
            skip_reasons[f"no_template_for_{root_tag}"] += 1
            continue

        # Quick extraction: just task_id, sprint, branch
        try:
            result = extract_variables(
                template, filepath,
                include_vars=["task_id", "sprint", "branch"],
                include_metadata=True,
            )
            confidence = result.get("_confidence", 0)
            # Simple validation: check task_id is not empty
            if result.get("task_id"):
                stats["pass"] += 1
            else:
                stats["fail"] += 1
                failures.append({
                    "file": filename,
                    "root": root_tag,
                    "template": os.path.basename(template),
                    "confidence": confidence,
                    "result": {k: v for k, v in result.items() if not k.startswith("_")},
                })
        except Exception as e:
            stats["error"] += 1
            failures.append({
                "file": filename,
                "root": root_tag,
                "template": os.path.basename(template),
                "error": str(e)[:200],
            })

    # Report
    print(f"\n{'='*60}")
    print(f"BULK TEST RESULTS")
    print(f"{'='*60}")
    print(f"  Pass:  {stats['pass']}")
    print(f"  Fail:  {stats['fail']}")
    print(f"  Skip:  {stats['skip']}")
    print(f"  Error: {stats['error']}")
    print(f"  Total: {sum(stats.values())}")
    print(f"\nSkip reasons:")
    for reason, count in sorted(skip_reasons.items(), key=lambda x: -x[1])[:10]:
        print(f"  {reason}: {count}")

    if failures:
        print(f"\nFailures ({len(failures)}):")
        for f in failures[:10]:
            print(f"  {f['file']}")
            if "error" in f:
                print(f"    ERROR: {f['error']}")
            else:
                print(f"    result: {json.dumps(f['result'])}")

    # Also show a few passing examples
    print(f"\nSample passing extractions (first 5):")
    count = 0
    for filepath in xml_files:
        if count >= 5:
            break
        root_tag = get_root_tag(filepath)
        if root_tag != "repo-qa-task":
            continue
        template = TEMPLATES.get(root_tag)
        if not template:
            continue
        try:
            result = extract_variables(
                template, filepath,
                include_vars=["task_id", "sprint", "branch"],
                include_metadata=False,
            )
            if result.get("task_id"):
                print(f"  {os.path.basename(filepath)}")
                print(f"    task_id={result.get('task_id')}, sprint={result.get('sprint')}, branch={result.get('branch')}")
                count += 1
        except Exception:
            pass


if __name__ == "__main__":
    main()
