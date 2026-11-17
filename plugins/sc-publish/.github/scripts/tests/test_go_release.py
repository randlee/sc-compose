"""Controlled native-module builds, immutable tags and proxy failures."""
import copy
from pathlib import Path
import subprocess
import sys
import zipfile

import pytest
import yaml

sys.path.insert(0, str(Path(__file__).parents[1]))
import go_release as go


def manifest():
    return {"go_modules": [{"module": "example.test/team/tool/binding", "source": "binding",
        "artifact_prefix": "native-binding", "targets": [
            {"target": "aarch64-apple-darwin", "os": "macos-15", "goos": "darwin", "goarch": "arm64"}]}]}


def bundles(tmp_path, values=None):
    values = values or manifest()
    for row in go.matrix(values)["include"]:
        with zipfile.ZipFile(tmp_path / (row["artifact"] + ".zip"), "w") as output:
            output.writestr("go.mod", "module " + row["module"] + "\n")
            output.writestr("native/library.a", "real-library-fixture")
    return tmp_path


def test_nonadopter_requires_no_go_commands(tmp_path):
    assert go.configuration({}) == []
    assert go.matrix({}) == {"include": []}
    go.publish({}, tmp_path, "1.3.0", "a" * 40, run=lambda *a, **k: pytest.fail("no commands"))


@pytest.mark.parametrize("change", ["traversal", "unknown", "bad-module", "duplicate", "duplicate-target", "bad-hook"])
def test_invalid_declarations_are_refused(change):
    values = manifest()
    row = values["go_modules"][0]
    if change == "traversal": row["source"] = "../outside"
    if change == "unknown": row["unrecognized"] = True
    if change == "bad-module": row["module"] = "invalid module"
    if change == "duplicate": values["go_modules"].append(copy.deepcopy(row))
    if change == "duplicate-target": row["targets"].append(copy.deepcopy(row["targets"][0]))
    if change == "bad-hook": row["build"] = "run a shell"
    with pytest.raises(go.GoReleaseError): go.configuration(values)


@pytest.mark.parametrize("damage", ["missing", "extra", "duplicate", "empty", "invalid", "wrong-module"])
def test_verification_refuses_incomplete_or_ambiguous_bundle_sets(tmp_path, damage):
    directory = bundles(tmp_path)
    archive = next(directory.glob("*.zip"))
    if damage == "missing": archive.unlink()
    if damage == "extra": (directory / "extra.zip").write_bytes(archive.read_bytes())
    if damage == "duplicate":
        (directory / "nested").mkdir()
        (directory / "nested" / archive.name).write_bytes(archive.read_bytes())
    if damage == "empty": archive.write_bytes(b"")
    if damage == "invalid": archive.write_bytes(b"not a zip")
    if damage == "wrong-module":
        with zipfile.ZipFile(archive, "w") as output: output.writestr("go.mod", "module example.test/wrong\n")
    with pytest.raises((go.GoReleaseError, zipfile.BadZipFile)):
        go.verify_bundles(manifest(), directory)


def test_tag_creation_reuse_and_conflict_are_immutable():
    calls = []
    commit = "a" * 40
    def absent(argv, **kwargs):
        calls.append(argv)
        return ""
    assert go.ensure_tag("binding/v1.3.0", commit, run=absent) == "created"
    assert calls[-1] == ["git", "push", "origin", commit + ":refs/tags/binding/v1.3.0"]
    calls.clear()
    def present(argv, **kwargs):
        calls.append(argv)
        return commit + "\trefs/tags/binding/v1.3.0\n"
    assert go.ensure_tag("binding/v1.3.0", commit, run=present) == "reused"
    assert len(calls) == 1
    with pytest.raises(go.GoReleaseError, match="another commit"):
        go.ensure_tag("binding/v1.3.0", "b" * 40, run=present)
    assert all(argv[1] == "ls-remote" for argv in calls)


def test_annotated_tag_reuses_peeled_commit():
    commit = "a" * 40
    assert go.ensure_tag("binding/v1.3.0", commit, run=lambda *a, **k:
        "b" * 40 + "\trefs/tags/binding/v1.3.0\n" + commit + "\trefs/tags/binding/v1.3.0^{}") == "reused"


def test_proxy_failure_stops_publication_and_missing_bundle_prevents_tag_push(tmp_path):
    calls = []
    def run(argv, **kwargs):
        calls.append((argv, kwargs))
        if argv[0] == "go": raise subprocess.CalledProcessError(1, argv)
        return ""
    with pytest.raises(go.GoReleaseError):
        go.publish(manifest(), tmp_path, "1.3.0", "a" * 40, run=run)
    assert calls == []
    bundles(tmp_path)
    with pytest.raises(subprocess.CalledProcessError):
        go.publish(manifest(), tmp_path, "1.3.0", "a" * 40, run=run)
    argv, options = calls[-1]
    assert argv == ["go", "list", "-m", "example.test/team/tool/binding@v1.3.0"]
    assert options["env"]["GOPROXY"] == "https://proxy.golang.org"
    assert calls[-2][0][1] == "push"


def test_build_qualifies_staged_and_unpacked_modules_and_independent_consumer(tmp_path):
    values = manifest()
    row = values["go_modules"][0]
    row.update(consumer_test="consumer", build=["builder", "{target}"], stage=["stager", "{stage}"])
    source = tmp_path / "binding"; source.mkdir()
    (source / "go.mod").write_text("module " + row["module"] + "\n")
    consumer = tmp_path / "consumer"; consumer.mkdir()
    (consumer / "go.mod").write_text("module example.test/consumer\n")
    calls = []
    def run(argv, **kwargs):
        calls.append((argv, kwargs))
        if argv[:2] == ["go", "env"]: return "darwin\narm64"
        if argv[0] == "stager":
            stage = Path(argv[1]); stage.mkdir()
            (stage / "go.mod").write_text((source / "go.mod").read_text())
            (stage / "library.a").write_bytes(b"native-fixture")
        return ""
    archive = go.build(values, "native-binding-aarch64-apple-darwin", "1.3.0", tmp_path, tmp_path / "out", run=run)
    assert archive.is_file()
    assert sum(argv == ["go", "test", "./..."] for argv, _ in calls) == 4
    assert sum(argv == ["go", "run", "."] for argv, _ in calls) == 2
    replacements = [argv[-1] for argv, _ in calls if argv[:3] == ["go", "mod", "edit"]]
    assert len(replacements) == 2 and replacements[0] != replacements[1]
    assert "unpacked" in replacements[1]


def test_wrong_native_coordinate_never_builds(tmp_path):
    source = tmp_path / "binding"; source.mkdir()
    (source / "go.mod").write_text("module example.test/team/tool/binding\n")
    with pytest.raises(go.GoReleaseError, match="native coordinate"):
        go.build(manifest(), "native-binding-aarch64-apple-darwin", "1.3.0", tmp_path, tmp_path / "out",
                 run=lambda argv, **kwargs: "linux\namd64")


def test_existing_workflows_gate_irreversible_publication_on_go_verification():
    workflows = Path(__file__).parents[2] / "workflows"
    release = yaml.safe_load((workflows / "release.yml").read_text())["jobs"]
    for job in ("publish", "publish-testpypi", "release"):
        assert "go-verified" in release[job]["needs"]
    assert "build-go-modules" in release["go-verified"]["needs"]
    tag_step = next(step for step in release["gate-and-tag"]["steps"] if step.get("id") == "release-ref")
    assert tag_step["run"].index('if [[ "${GO_ENABLED:-false}" == "true" ]]') < tag_step["run"].index("git push")
    preflight = yaml.safe_load((workflows / "release-preflight.yml").read_text())["jobs"]
    assert "go-verified" in preflight
    assert preflight["build-go-modules"]["if"] == "${{ needs.go-plan.outputs.enabled == 'true' }}"


def test_real_local_git_remote_preserves_existing_tag_identity(tmp_path):
    remote = tmp_path / "remote.git"
    repo = tmp_path / "repo"
    go.execute(["git", "init", "--bare", str(remote)])
    go.execute(["git", "init", str(repo)])
    def run(argv, **kwargs):
        return go.execute(argv, cwd=repo)
    run(["git", "config", "user.name", "Fixture"])
    run(["git", "config", "user.email", "fixture@example.test"])
    run(["git", "remote", "add", "origin", str(remote)])
    (repo / "source").write_text("first")
    run(["git", "add", "source"]); run(["git", "commit", "-m", "first"])
    first = run(["git", "rev-parse", "HEAD"])
    assert go.ensure_tag("binding/v1.3.0", first, run=run) == "created"
    assert go.ensure_tag("binding/v1.3.0", first, run=run) == "reused"
    (repo / "source").write_text("second")
    run(["git", "commit", "-am", "second"])
    second = run(["git", "rev-parse", "HEAD"])
    with pytest.raises(go.GoReleaseError, match="another commit"):
        go.ensure_tag("binding/v1.3.0", second, run=run)
    assert run(["git", "ls-remote", "origin", "refs/tags/binding/v1.3.0"]).split()[0] == first


def test_mixed_release_assets_preserve_the_exact_go_bundle_set(tmp_path):
    bundles(tmp_path)
    (tmp_path / "cli-1.3.0-windows.zip").write_bytes(b"other artifact family")
    with pytest.raises(go.GoReleaseError):
        go.verify_bundles(manifest(), tmp_path)
    assert len(go.verify_bundles(manifest(), tmp_path, mixed=True)) == 1
    (tmp_path / "native-binding-undeclared.zip").write_bytes(b"unexpected Go target")
    with pytest.raises(go.GoReleaseError):
        go.verify_bundles(manifest(), tmp_path, mixed=True)


def test_installer_roundtrips_go_only_when_declared(tmp_path):
    import json
    import tomllib
    from test_install import INSTALL, InstallValuesTests
    values = InstallValuesTests.valid_values()
    values.update(manifest())
    config = tmp_path / "install.json"; config.write_text(json.dumps(values))
    loaded = INSTALL.load_install_values(config)
    destination = tmp_path / "manifest.toml"
    INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), loaded, destination)
    assert tomllib.loads(destination.read_text())["go_modules"] == values["go_modules"]
    values["go_modules"][0]["source"] = "../outside"
    config.write_text(json.dumps(values))
    import argparse
    with pytest.raises(argparse.ArgumentTypeError): INSTALL.load_install_values(config)



def test_go_adopter_tag_step_defers_all_mutation_and_preserves_build_ref(tmp_path):
    import os
    workflow = Path(__file__).parents[2] / "workflows/release.yml"
    steps = yaml.safe_load(workflow.read_text())["jobs"]["gate-and-tag"]["steps"]
    script = next(step["run"] for step in steps if step.get("id") == "release-ref")
    script = script.replace("${{ steps.meta.outputs.release_tag }}", "v1.3.0")
    script = script.replace("${{ steps.meta.outputs.release_target }}", "production")
    script = script.replace("${{ steps.release_gate.outputs.release_sha }}", "a" * 40)
    tools = tmp_path / "bin"; tools.mkdir()
    git = tools / "git"
    git.write_text('#!/bin/sh\ncase "$1" in\nfetch|rev-parse) exit 0 ;;\n*) echo "unexpected mutation $*" >&2; exit 22 ;;\nesac\n')
    git.chmod(0o755)
    output = tmp_path / "output"
    env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"],
               GO_ENABLED="true", GITHUB_OUTPUT=str(output))
    result = subprocess.run(["bash", "-c", script], env=env, capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result.stderr
    assert output.read_text() == "build_ref=" + "a" * 40 + "\n"



@pytest.mark.parametrize("command", ["plan", "verify", "release-tag"])
def test_non_go_cli_noops_without_go_git_or_bundle_directory(tmp_path, command):
    import json
    import os
    manifest = tmp_path / "manifest.toml"; manifest.write_text("schema_version=1\n")
    script = Path(__file__).parents[1] / "go_release.py"
    result = subprocess.run([sys.executable, str(script), command, "--manifest", str(manifest)],
                            cwd=tmp_path, env=dict(os.environ, PATH=""), capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result.stderr
    if command == "plan": assert json.loads(result.stdout) == {"enabled": False, "matrix": {"include": []}}
    assert not (tmp_path / "go-bundles").exists()


@pytest.mark.parametrize("missing", [False, True])
def test_release_declaration_reads_non_go_commit_even_without_committed_manifest(tmp_path, missing):
    def git(*args):
        return go.execute(["git", *args], cwd=tmp_path)
    git("init"); git("config", "user.name", "Fixture"); git("config", "user.email", "fixture@example.test")
    (tmp_path / "manifest.toml").write_text("schema_version=1\n")
    git("add", "manifest.toml"); git("commit", "-m", "fixture")
    script = Path(__file__).parents[1] / "go_release.py"
    result = subprocess.run([sys.executable, str(script), "plan", "--manifest",
                             "missing.toml" if missing else "manifest.toml", "--source-ref", "HEAD"],
                            cwd=tmp_path, capture_output=True, text=True, timeout=10)
    import json
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout) == {"enabled": False, "matrix": {"include": []}}


def test_non_go_workflow_conditions_admit_normal_publication():
    import re
    workflows = Path(__file__).parents[2] / "workflows"
    jobs = yaml.safe_load((workflows / "release.yml").read_text())["jobs"]
    values = {"needs.go-plan.outputs.enabled": "false", "needs.go-plan.result": "success",
              "needs.build-go-modules.result": "skipped", "needs.go-verified.result": "success",
              "needs.release-plan.outputs.has_crates": "true",
              "needs.gate-and-tag.outputs.crates_already_published": "false",
              "needs.build.result": "skipped", "needs.build-python-wheels.result": "success",
              "needs.build-python-sdists.result": "skipped"}
    def admitted(expression):
        expression = expression.removeprefix("${{").removesuffix("}}").strip()
        expression = re.sub(r"needs\.[A-Za-z0-9_.-]+", lambda match: repr(values[match.group()]), expression)
        expression = expression.replace("!failure()", "True").replace("!cancelled()", "True")
        expression = expression.replace("&&", " and ").replace("||", " or ")
        return eval(" ".join(expression.split()), {"__builtins__": {}}, {})
    assert admitted(jobs["go-verified"]["if"])
    for name, target in [("publish", "production"), ("publish-testpypi", "testpypi"), ("release", "production")]:
        values["needs.gate-and-tag.outputs.release_target"] = target
        assert admitted(jobs[name]["if"]), name
        assert "go-verified" in jobs[name]["needs"]
