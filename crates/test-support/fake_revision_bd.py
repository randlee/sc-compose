#!/usr/bin/env python3
"""Closed revision-test responses shared by CLI and Python adapter tests.

The executable is committed and never rewritten by a test. JSON next to each
fixture selects responses, so changing a scenario cannot race an executable
write against process creation.
"""
from __future__ import annotations

import json
from pathlib import Path
import sys


def main() -> int:
    args = sys.argv[1:]
    if args[:1] == ["--config"]:
        config_path = Path(args[1])
        args = args[2:]
    else:
        config_path = Path(sys.argv[0]).with_name("revision-bd.json")
    config = json.loads(config_path.read_text(encoding="utf-8"))
    trace = Path(config["trace"])
    with trace.open("a", encoding="utf-8") as stream:
        stream.write(" ".join(args) + "\n")
    command = args[0] if args else ""
    graph = config.get("scenario") == "graph"
    if command == "cook" and config.get("invalid"):
        print("invalid", file=sys.stderr)
        return 1
    if command == "cook":
        steps = [{"id": "build", "title": "Build"}]
        if graph:
            steps.append({"id": "verify", "title": "Verify", "needs": ["build"]})
        result = {"formula": "sample", "type": "workflow", "steps": steps}
    elif command == "show":
        result = [{"id": "proj-1", "title": "old", "description": "", "status": "open",
                   "metadata": {"sc_compose_graph": {"mode": "pour", "formula": "sample", "step": None}}}]
    elif command == "children":
        result = ([{"id": "proj-3", "status": "closed", "labels": [],
                    "metadata": {"sc_compose_graph": {"mode": "pour", "formula": "sample", "step": "obsolete"}}}]
                  if graph else [])
    elif command == "dep" and args[1:2] == ["list"]:
        if graph and args[-1] == "proj-3" and "--direction=down" in args:
            result = [{"id": "proj-1", "dependency_type": "blocks"},
                      {"id": "outside-2", "dependency_type": "blocks"}]
        elif graph and args[-1] == "proj-3" and "--direction=up" in args:
            result = [{"id": "outside-1", "dependency_type": "blocks"}]
        else:
            result = []
    elif command == "sql":
        if config.get("sql") != "dirty":
            print("probe unsupported", file=sys.stderr)
            return 1
        result = [{"table_name": "issues", "staged": False, "status": "modified"}]
    elif command == "create":
        result = {} if graph else {"ids": {"build": "proj-2"}}
    elif command == "update" and not graph:
        print("injected update failure", file=sys.stderr)
        return 1
    else:
        print(f"unexpected revision fixture command: {args!r}", file=sys.stderr)
        return 2
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
