"""Real packaging/hook subprocesses with controlled staged payloads; no uploads."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib
import zipfile

import pytest
import yaml

sys.path.insert(0, str(Path(__file__).parents[1]))
import archive_verification as hook
from test_install import INSTALL, InstallValuesTests

WORKFLOWS = INSTALL.PACKAGE_ROOT / ".github/workflows"


@pytest.mark.parametrize("value", [None, [], "shell command", [""], [None], ["bad\0arg"],
    ["{unknown}"], ["{target.attribute}"], ["{target[0]}"], ["{target!r}"], ["{target:>10}"], ["{"]])
def test_invalid_declarations_fail_closed(value):
    with pytest.raises(ValueError): hook.configuration({"archive_verification": value})


def test_nonadopters_preserve_workflow_bytes_and_do_not_import_hook(tmp_path, monkeypatch):
    for name in ["release.yml", "prerelease-archive.yml"]:
        text = (WORKFLOWS / name).read_text()
        assert hook.extend_workflow(text, {}) == text
    path = tmp_path / "input.json"; path.write_text(json.dumps(InstallValuesTests.valid_values()))
    monkeypatch.setitem(sys.modules, "archive_verification", None)
    values = INSTALL.load_install_values(path)
    copies = [(WORKFLOWS / name, Path(".github/workflows") / name)
              for name in ["release.yml", "prerelease-archive.yml"]]
    assert INSTALL.install_sources(copies, values, tmp_path) == copies


def test_installer_renders_argv_and_keeps_nonadopter_manifest_bytes(tmp_path, monkeypatch):
    values = InstallValuesTests.valid_values()
    plain = tmp_path / "plain.toml"
    INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), values, plain)
    # Remove only the new template extension to compare against the old render.
    source = (INSTALL.PACKAGE_ROOT / "release/publish-artifacts.toml.j2").read_text()
    source = source.replace("  - has_archive_verification\n  - archive_verification\n", "")
    source = source.replace('{{ "\\narchive_verification = " ~ archive_verification if has_archive_verification else "" }}', '')
    baseline_root = tmp_path / "baseline"; baseline_root.mkdir()
    (baseline_root / "template.j2").write_text(source)
    baseline = tmp_path / "baseline.toml"
    with monkeypatch.context() as patch:
        patch.setattr(INSTALL, "PACKAGE_ROOT", baseline_root)
        INSTALL.render_template(Path("template.j2"), values, baseline)
    assert plain.read_bytes() == baseline.read_bytes()
    values["archive_verification"] = ["python3", "scripts/verify.py", "{stage_dir}", "{target}", "{manifest}"]
    path = tmp_path / "input.json"; path.write_text(json.dumps(values))
    loaded = INSTALL.load_install_values(path)
    output = tmp_path / "adopter.toml"
    INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), loaded, output)
    assert tomllib.loads(output.read_text())["archive_verification"] == values["archive_verification"]
    values["archive_verification"] = []
    path.write_text(json.dumps(values))
    with pytest.raises(argparse.ArgumentTypeError): INSTALL.load_install_values(path)


@pytest.mark.parametrize("workflow", ["release.yml", "prerelease-archive.yml"])
@pytest.mark.parametrize("exit_code", [0, 7])
def test_generated_packaging_blocks_upload_after_failed_hook(tmp_path, workflow, exit_code):
    if not (INSTALL.PACKAGE_ROOT / ".sc-publish-source-root").is_file():
        pytest.skip("installed consumers do not re-run the package source installer")
    target = "fixture-target"
    binary = tmp_path / "target" / target / "release/tool"
    binary.parent.mkdir(parents=True); binary.write_text("binary fixture")
    (tmp_path / "payload").mkdir(); (tmp_path / "payload/data.txt").write_text("bundled fixture")
    script = tmp_path / "verify payload.py"
    script.write_text('''import json,sys
from pathlib import Path
stage,target,manifest,code=sys.argv[1:]
assert (Path(stage)/'bin/tool').read_text()=='binary fixture'
assert (Path(stage)/'share/data/data.txt').read_text()=='bundled fixture'
Path('verified.json').write_text(json.dumps([stage,target,manifest]))
raise SystemExit(int(code))
''')
    argv = [sys.executable, str(script), "{stage_dir}", "{target}", "{manifest}", str(exit_code)]
    values = {"archive_verification": argv, "project": {"name": "tool", "archive_prefix": "tool",
        "description": "Fixture", "homepage": "https://example.test/tool", "license": "MIT"},
        "release_binaries": [{"name": "tool", "bundled_paths": [{"source": "payload", "destination": "share/data"}]}],
        "release_targets": [{"target": target, "os": "ubuntu-latest", "archive": "zip"}]}
    manifest = tmp_path / "release/publish-artifacts.toml"; manifest.parent.mkdir()
    manifest.write_text("\n".join(key + "=" + INSTALL._toml_literal(value) for key, value in values.items()))
    shutil.copytree(INSTALL.PACKAGE_ROOT / ".github/scripts", tmp_path / ".github/scripts")
    rendered = tmp_path / "rendered"; rendered.mkdir()
    sources = INSTALL.install_sources([(WORKFLOWS / workflow, Path(".github/workflows") / workflow)], values, rendered)
    jobs = yaml.safe_load(sources[0][0].read_text())["jobs"]
    steps = jobs["build"]["steps"]
    package = next(step for step in steps if step.get("name", "").startswith("Package manifest-declared"))
    upload = next(step for step in steps if step.get("name") == "Upload artifact")
    assert steps.index(package) < steps.index(upload)
    assert not package.get("continue-on-error") and "if" not in upload
    body = package["run"]
    assert body.index("archive_verification.py") < body.index('archive = Path(')
    assert '[sys.executable, ".github/scripts/archive_verification.py"' in body
    body = body.replace("${{ matrix.target }}", target)
    body = body.replace("${{ needs.gate-and-tag.outputs.release_version }}", "1.2.3")
    body = body.replace("${{ needs.plan.outputs.version }}", "1.2.3")
    # A marker represents the subsequent on-success upload action. This runs
    # the generated packaging code and real verifier, with no GitHub service.
    body += "\nprintf uploaded > upload.marker\n"
    result = subprocess.run(["bash", "-c", body], cwd=tmp_path, capture_output=True, text=True,
        env=dict(os.environ, RELEASE_ARTIFACT_MANIFEST=str(manifest), GITHUB_ENV=str(tmp_path / "github-env")), timeout=30)
    stage, observed_target, observed_manifest = json.loads((tmp_path / "verified.json").read_text())
    assert Path(stage).is_absolute() and observed_target == target and Path(observed_manifest) == manifest
    archive = tmp_path / f"tool_1.2.3_{target}.zip"
    assert (tmp_path / "upload.marker").exists() is (exit_code == 0)
    assert archive.exists() is (exit_code == 0)
    assert (result.returncode == 0) is (exit_code == 0), result.stderr
    if exit_code == 0:
        with zipfile.ZipFile(archive) as payload:
            assert f"tool_1.2.3_{target}/share/data/data.txt" in payload.namelist()


def test_declared_argv_is_executed_without_shell_expansion(tmp_path):
    stage = tmp_path / "stage"; stage.mkdir()
    manifest = tmp_path / "manifest.toml"
    script = tmp_path / "record.py"
    script.write_text("import json,sys; print(json.dumps(sys.argv[1:]))\n")
    argv = [sys.executable, str(script), "{stage_dir}", "{target}", "{manifest}", "literal; echo unsafe"]
    manifest.write_text("archive_verification=" + INSTALL._toml_literal(argv))
    # Shell metacharacters remain data. Escaped braces are supported too.
    assert hook.configuration({"archive_verification": ["tool", "{{literal}}"]}) == ["tool", "{{literal}}"]
    result = subprocess.run([sys.executable, str(Path(__file__).parents[1] / "archive_verification.py"),
        "--manifest", str(manifest), "--stage-dir", str(stage), "--target", "fixture"], capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout) == [str(stage), "fixture", str(manifest), "literal; echo unsafe"]


def test_manifest_loader_validates_and_preserves_declared_hook(tmp_path):
    from release_manifest import load_manifest
    path = tmp_path / "manifest.toml"
    argv = ["verifier", "{stage_dir}", "{target}", "{manifest}"]
    path.write_text("archive_verification=" + INSTALL._toml_literal(argv))
    assert load_manifest(path)["archive_verification"] == argv
    path.write_text("archive_verification=[]")
    with pytest.raises(ValueError): load_manifest(path)


def test_archive_and_rc_extensions_compose_without_losing_either(tmp_path):
    if not (INSTALL.PACKAGE_ROOT / ".sc-publish-source-root").is_file():
        pytest.skip("installed consumers do not re-run the package source installer")
    values = {"archive_verification": ["verify", "{stage_dir}"],
        "python_distributions": [{"wheels": ["ubuntu-latest"]}],
        "rc_python_compatibility": {"versions": ["3.11"], "build_python": "3.11",
            "build": ["python", "build.py"], "smoke": ["python", "smoke.py"]}}
    names = ["release.yml", "prerelease-archive.yml", "release-candidate.yml"]
    sources = INSTALL.install_sources([(WORKFLOWS / name, Path(".github/workflows") / name) for name in names], values, tmp_path)
    result = {path.name: yaml.safe_load(source.read_text()) for source, path in sources}
    release = result["release.yml"]["jobs"]
    assert "archive_verification.py" in next(step["run"] for step in release["build"]["steps"] if step.get("name", "").startswith("Package manifest-declared"))
    assert any("rc_python_compatibility.py check" in step.get("run", "") for step in release["gate-and-tag"]["steps"])
    assert "python-compat" in result["release-candidate.yml"]["jobs"]
