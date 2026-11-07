"""Execute the canonical crate publication shells against deterministic registries."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys

import pytest
import yaml

KIT = next(p for p in Path(__file__).resolve().parents if (p / ".github/workflows/release.yml").is_file())


def publish_shell(workflow: str) -> str:
    document = yaml.safe_load((KIT / ".github/workflows" / workflow).read_text())
    return next(
        step["run"]
        for job in document["jobs"].values()
        for step in job.get("steps", [])
        if step.get("name") == "Publish crates in order (idempotent)"
    ).replace("${{ needs.gate-and-tag.outputs.release_version }}", "1.5.0")


def run_publication(tmp_path: Path, workflow: str, *, opt_in: bool, base_states: list[str]):
    scripts = tmp_path / ".github/scripts"
    scripts.mkdir(parents=True)
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    trace = tmp_path / "trace.jsonl"
    (scripts / "release_artifacts.py").write_text('''import json, os, pathlib, sys
command = sys.argv[1]
trace = pathlib.Path(os.environ["TRACE"])
def record(event, **extra):
    with trace.open("a") as stream:
        stream.write(json.dumps({"event": event, **extra}) + "\\n")
if command == "list-publish-plan":
    for name in ["base", "dependent"]:
        row = f"{name}|0|crates/{name}/Cargo.toml"
        if "--include-visibility" in sys.argv:
            row += "|" + os.environ["OPT_IN"]
        print(row)
elif command == "public-registry-inquiry-plan":
    name = sys.argv[sys.argv.index("--name") + 1]
    version = sys.argv[sys.argv.index("--version") + 1]
    record("inquiry", name=name, version=version)
    print(json.dumps({"checks": [{"version_lookup_url": f"https://registry.invalid/crates/{name}/{version}"}]}))
elif command == "registry-status":
    url = sys.argv[sys.argv.index("--url") + 1]
    name, version = url.split("/")[-2:]
    count_path = pathlib.Path(name + ".count")
    count = int(count_path.read_text()) if count_path.exists() else 0
    states = json.loads(os.environ["BASE_STATES"]) if name == "base" else ["absent", "published"]
    state = states[min(count, len(states) - 1)]
    count_path.write_text(str(count + 1))
    record("lookup", name=name, version=version, state=state)
    print(state)
else:
    raise SystemExit("unexpected command " + command)
''')
    (bin_dir / "jq").write_text(
        f"#!{sys.executable}\nimport json,sys\nprint(json.load(sys.stdin)['checks'][0]['version_lookup_url'])\n"
    )
    (bin_dir / "cargo").write_text(
        f"#!{sys.executable}\nimport json,os,pathlib,sys\n"
        "assert sys.argv[1] == 'publish' and '--locked' in sys.argv\n"
        "manifest = sys.argv[sys.argv.index('--manifest-path') + 1]\n"
        "with pathlib.Path(os.environ['TRACE']).open('a') as stream:\n"
        "    stream.write(json.dumps({'event':'upload','name':manifest.split('/')[-2]}) + '\\n')\n"
    )
    for command in bin_dir.iterdir():
        command.chmod(0o755)
    result = subprocess.run(
        ["bash", "-c", publish_shell(workflow)], cwd=tmp_path,
        env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
             "TRACE": str(trace), "OPT_IN": str(opt_in).lower(),
             "BASE_STATES": json.dumps(base_states), "RELEASE_TAG": "v1.5.0",
             "RELEASE_ARTIFACT_MANIFEST": "release/publish-artifacts.toml"},
        capture_output=True, text=True, timeout=10, check=False,
    )
    return result, [json.loads(line) for line in trace.read_text().splitlines()]


@pytest.mark.parametrize("workflow", ["release.yml", "crates-publish.yml"])
def test_visibility_opt_in_confirms_uploaded_predecessor_before_dependent(tmp_path, workflow):
    result, trace = run_publication(tmp_path, workflow, opt_in=True, base_states=["absent", "published"])
    assert result.returncode == 0, result.stderr
    visible = trace.index({"event": "lookup", "name": "base", "version": "1.5.0", "state": "published"})
    dependent_upload = trace.index({"event": "upload", "name": "dependent"})
    assert visible < dependent_upload
    assert all(event["version"] == "1.5.0" for event in trace if event["event"] in {"inquiry", "lookup"})


@pytest.mark.parametrize("workflow", ["release.yml", "crates-publish.yml"])
@pytest.mark.parametrize("after_upload", ["absent", "indeterminate"])
def test_visibility_opt_in_stops_before_dependent_if_upload_not_visible(tmp_path, workflow, after_upload):
    result, trace = run_publication(tmp_path, workflow, opt_in=True, base_states=["absent", after_upload])
    assert result.returncode != 0
    assert [entry["name"] for entry in trace if entry["event"] == "upload"] == ["base"]
    assert not any(entry["name"] == "dependent" for entry in trace)
    assert ("not visible after upload" if after_upload == "absent" else "indeterminate") in result.stderr


@pytest.mark.parametrize("workflow", ["release.yml", "crates-publish.yml"])
def test_already_published_predecessor_is_positively_checked_and_not_uploaded(tmp_path, workflow):
    result, trace = run_publication(tmp_path, workflow, opt_in=True, base_states=["published"])
    assert result.returncode == 0, result.stderr
    assert [entry["name"] for entry in trace if entry["event"] == "upload"] == ["dependent"]
    assert trace[:2] == [
        {"event": "inquiry", "name": "base", "version": "1.5.0"},
        {"event": "lookup", "name": "base", "version": "1.5.0", "state": "published"},
    ]


@pytest.mark.parametrize("workflow", ["release.yml", "crates-publish.yml"])
def test_non_adopter_retains_existing_upload_sequence_without_postcheck(tmp_path, workflow):
    result, trace = run_publication(tmp_path, workflow, opt_in=False, base_states=["absent"])
    assert result.returncode == 0, result.stderr
    assert [entry["name"] for entry in trace if entry["event"] == "upload"] == ["base", "dependent"]
    assert sum(entry["event"] == "lookup" and entry["name"] == "base" for entry in trace) == 1


@pytest.mark.parametrize("declared", [None, False, True])
def test_plan_visibility_is_optional_and_legacy_output_unchanged(tmp_path, declared):
    manifest = tmp_path / "publish-artifacts.toml"
    manifest.write_text('''[[crates]]
artifact="base"
package="base"
cargo_toml="crates/base/Cargo.toml"
publish=true
publish_order=1
wait_after_publish_seconds=0
''' + ("" if declared is None else "verify_registry_visibility=" + str(declared).lower() + "\n"))
    args = [sys.executable, str(KIT / ".github/scripts/release_artifacts.py"), "list-publish-plan", "--manifest", str(manifest)]
    legacy = subprocess.run(args, text=True, capture_output=True, timeout=10)
    assert legacy.returncode == 0, legacy.stderr
    assert legacy.stdout == "base|0\n"
    extended = subprocess.run(args + ["--include-manifest", "--include-visibility"], text=True, capture_output=True, timeout=10)
    assert extended.returncode == 0, extended.stderr
    assert extended.stdout == "base|0|crates/base/Cargo.toml|" + str(bool(declared)).lower() + "\n"


def test_manifest_rejects_nonboolean_visibility_policy(tmp_path):
    manifest = tmp_path / "publish-artifacts.toml"
    manifest.write_text('''[[crates]]
artifact="base"
package="base"
publish_order=1
verify_registry_visibility="true"
''')
    result = subprocess.run(
        [sys.executable, str(KIT / ".github/scripts/release_artifacts.py"), "list-publish-plan", "--manifest", str(manifest)],
        text=True, capture_output=True, timeout=10,
    )
    assert result.returncode != 0
    assert "verify_registry_visibility must be a boolean" in result.stderr
