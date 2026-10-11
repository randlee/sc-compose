"""Real-bd attach through the installed wheel, without calling the CLI."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys

import pytest
import sc_composer_beads as beads


def _worker(root: Path, bd: str) -> None:
    def run(*args: str) -> str:
        result = subprocess.run([bd, *args], cwd=root, capture_output=True, text=True, timeout=120)
        assert result.returncode == 0, result.stderr
        return result.stdout

    def snapshot() -> dict:
        rows = json.loads(run("list", "--all", "-n", "0", "--json"))
        return {"beads": rows, "edges": {row["id"]: json.loads(run("dep", "list", row["id"], "--json")) for row in rows}}

    assert Path(os.environ["BEADS_DIR"]) == root / ".beads"
    assert os.environ["BEADS_NO_DAEMON"] == "1"
    run("init", "--non-interactive", "--quiet", "--skip-agents", "--skip-hooks", "--prefix", "wheel")
    parent = json.loads(run("create", "Release", "--type", "epic", "--json"))["id"]
    template = root / "release.formula.toml.j2"
    template.write_text('formula = "release"\nversion = 1\ntype = "workflow"\n[[steps]]\nid = "build"\ntitle = "Build {{{ version }}}"\n[[steps]]\nid = "verify"\ntitle = "Verify"\nneeds = ["build"]\n', encoding="utf-8")
    request = beads.BeadComposeRequest(
        root, template, root / "release.formula.toml", {"version": "1.6.1"},
        operation="attach", formula_name="release", bd_executable=bd,
        pour_authorization="CreatePersistentBeads", parent=parent, ref="release",
    )
    before = snapshot()
    preview = beads.preview_attach(request).to_json()
    assert preview["outcome"] == "succeeded"
    assert all(node["action"] == "create" for node in preview["graph"]["nodes"])
    assert snapshot() == before
    applied = beads.attach(request).to_json()
    assert applied["outcome"] == "succeeded"
    expected = {step: f"{parent}.release-{step}" for step in ("build", "verify")}
    assert applied["graph"]["ids"] == expected
    assert all(node["action"] == "created" for node in applied["graph"]["nodes"])
    assert {row["id"] for row in json.loads(run("children", parent, "--json"))} == set(expected.values())
    run("update", expected["build"], "--notes", "keep wheel evidence")
    run("close", expected["build"])
    before = snapshot()
    repeated = beads.attach(request).to_json()
    assert repeated["outcome"] == "succeeded"
    assert repeated["graph"]["ids"] == expected
    assert all(node["action"] == "existing" for node in repeated["graph"]["nodes"])
    assert all(edge["action"] == "existing" for edge in repeated["graph"]["edges"])
    assert "plan_path" not in repeated["graph"]
    assert snapshot() == before
    assert not (root / ".beads" / "formulas").exists()


def test_installed_wheel_attach_and_repeat_preserve_ids_and_existing_beads(tmp_path: Path) -> None:
    bd = os.environ.get("BD_EXECUTABLE")
    if bd is None:
        pytest.skip("BD_EXECUTABLE is not configured")
    # Configure only this process tree, so other pytest tests never inherit fixture state.
    env = dict(os.environ, BEADS_DIR=str(tmp_path / ".beads"), BEADS_NO_DAEMON="1")
    result = subprocess.run([sys.executable, str(Path(__file__).resolve()), str(tmp_path), bd],
                            env=env, capture_output=True, text=True, timeout=180)
    assert result.returncode == 0, result.stdout + result.stderr


if __name__ == "__main__":
    _worker(Path(sys.argv[1]), sys.argv[2])
