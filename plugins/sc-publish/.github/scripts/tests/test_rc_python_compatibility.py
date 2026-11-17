"""Controlled RC evidence and installer extension checks; no hosted calls."""
import copy
import json
import os
from pathlib import Path
import subprocess
import sys

import pytest
import yaml

sys.path.insert(0, str(Path(__file__).parents[1]))
import rc_python_compatibility as rc
from test_install import INSTALL, InstallValuesTests

CANDIDATE = "a" * 40


def manifest():
    return {"python_distributions": [{"wheels": ["ubuntu-latest", {"os": "windows-11-arm"}]}],
        "rc_python_compatibility": {"versions": ["3.9", "3.11", "3.15"], "build_python": "3.11",
            "setup": ["python", "-m", "pip", "install", "build==1.3.0"],
            "build": ["python", "scripts/build.py"], "smoke": ["python", "scripts/smoke.py"],
            "exclusions": {"windows-11-arm": ["3.9"]},
            "runner_overrides": {"ubuntu-latest": "ubuntu-24.04"}}}


def test_matrix_uses_both_wheel_forms_and_excludes_only_declared_interpreters():
    rows = {row["os"]: row for row in rc.matrix(manifest())}
    assert rows["ubuntu-latest"]["runner"] == "ubuntu-24.04"
    assert json.loads(rows["ubuntu-latest"]["versions"]) == ["3.9", "3.11", "3.15"]
    assert json.loads(rows["windows-11-arm"]["versions"]) == ["3.11", "3.15"]


@pytest.mark.parametrize("field,value", [("versions", []), ("versions", ["3.9", "3.9"]),
    ("versions", ["3.9; unsafe"]), ("build_python", ""), ("build", "shell command"),
    ("smoke", []), ("exclusions", {"unknown": ["3.9"]}),
    ("exclusions", {"windows-11-arm": ["3.9", "3.11", "3.15"]}),
    ("runner_overrides", {"ubuntu-latest": "unsafe runner"}), ("unexpected", True),
    ("python_distributions", []), ("python_distributions", [{"wheels": []}])])
def test_invalid_or_vacuous_declarations_fail(field, value):
    values = manifest()
    if field == "python_distributions": values[field] = value
    else: values["rc_python_compatibility"][field] = value
    with pytest.raises(rc.CompatibilityError): rc.configuration(values)
    with pytest.raises(rc.CompatibilityError):
        rc.check(values, CANDIDATE, "team/repo", query=lambda _: pytest.fail("invalid policy must not access API"))


def test_one_build_per_candidate_platform_and_each_declared_smoke_uses_selected_sdk():
    job = yaml.safe_load(rc.workflow_job(manifest()))["python-compat"]
    assert job["needs"] == "establish-provenance"
    assert job["runs-on"] == "${{ matrix.runner }}"
    assert job["name"] == "python-compat (${{ matrix.os }}, ${{ needs.establish-provenance.outputs.release_candidate_sha }})"
    steps = job["steps"]
    assert steps[0]["with"]["ref"] == "${{ needs.establish-provenance.outputs.release_candidate_sha }}"
    assert sum(step.get("run", "").endswith(".py build") for step in steps) == 1
    for version in manifest()["rc_python_compatibility"]["versions"]:
        setup = next(step for step in steps if step.get("name") == "Set up Python " + version)
        smoke = next(step for step in steps if step.get("name") == "Smoke-test wheels on Python " + version)
        assert setup["with"]["python-version"] == version
        assert setup["if"] == smoke["if"] == "${{ contains(fromJSON(matrix.versions), '" + version + "') }}"
        assert steps.index(setup) < steps.index(smoke)


def evidence(candidate=CANDIDATE):
    return [{"name": f"python-compat ({row['os']}, {candidate})", "conclusion": "success"}
            for row in rc.matrix(manifest())]


def check_jobs(jobs, conclusion="success"):
    def query(endpoint):
        return [{"workflow_runs": [{"id": 42, "conclusion": conclusion}]}] if "/workflows/" in endpoint else [{"jobs": jobs}]
    return rc.check(manifest(), CANDIDATE, "team/repo", query=query)


def test_only_complete_success_for_exact_candidate_admits_release():
    assert check_jobs(evidence())
    assert not check_jobs(evidence("b" * 40))
    assert not check_jobs(evidence()[:-1])
    assert not check_jobs(evidence(), conclusion="failure")
    for status in ["failure", "skipped", "cancelled", None]:
        jobs = evidence(); jobs[0]["conclusion"] = status
        assert not check_jobs(jobs)
    with pytest.raises(OSError):
        rc.check(manifest(), CANDIDATE, "team/repo", query=lambda _: (_ for _ in ()).throw(OSError("offline")))


def test_nonadopters_preserve_workflow_bytes_and_never_access_git_gh_or_python_sdk(tmp_path):
    workflows = INSTALL.PACKAGE_ROOT / ".github/workflows"
    for name in ["release.yml", "release-candidate.yml"]:
        text = (workflows / name).read_text()
        assert rc.extend_workflow(text, ".github/workflows/" + name, {}) == text
    assert rc.check({}, "invalid candidate", "invalid repository", query=lambda _: pytest.fail("no API access"))
    assert rc.workflow_job({}) == ""
    path = tmp_path / "manifest.toml"; path.write_text("schema_version=1\n")
    result = subprocess.run([sys.executable, str(Path(__file__).parents[1] / "rc_python_compatibility.py"),
                             "check", "--manifest", str(path)], cwd=tmp_path,
                            env=dict(os.environ, PATH=""), capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result.stderr


def test_missing_candidate_declaration_cannot_produce_passing_compatibility_jobs():
    with pytest.raises(rc.CompatibilityError, match="requires its candidate"):
        rc.execute_hook({}, "build")


def test_installer_renders_declaration_and_extends_only_selected_workflows(tmp_path):
    if not (INSTALL.PACKAGE_ROOT / ".sc-publish-source-root").is_file():
        pytest.skip("installed consumers do not re-run the package source installer")
    values = InstallValuesTests.valid_values()
    values["rc_python_compatibility"] = copy.deepcopy(manifest()["rc_python_compatibility"])
    # This fixture's declared wheel platform is ubuntu-latest only.
    values["rc_python_compatibility"]["exclusions"] = {}
    path = tmp_path / "input.json"; path.write_text(json.dumps(values))
    loaded = INSTALL.load_install_values(path)
    import tomllib
    output = tmp_path / "manifest.toml"
    INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), loaded, output)
    assert tomllib.loads(output.read_text())["rc_python_compatibility"] == values["rc_python_compatibility"]
    workflows = INSTALL.PACKAGE_ROOT / ".github/workflows"
    copies = [(workflows / name, Path(".github/workflows") / name) for name in ["release.yml", "release-candidate.yml"]]
    sources = INSTALL.install_sources(copies, loaded, tmp_path)
    release = yaml.safe_load(sources[0][0].read_text())
    assert "actions" not in release["permissions"]
    assert release["jobs"]["gate-and-tag"]["permissions"] == {"contents": "write", "actions": "read"}
    steps = release["jobs"]["gate-and-tag"]["steps"]
    admission = next(i for i, step in enumerate(steps) if "rc_python_compatibility.py check" in step.get("run", ""))
    tag = next(i for i, step in enumerate(steps) if step.get("id") == "release-ref")
    assert admission < tag
    assert steps[admission]["env"]["GH_TOKEN"] == "${{ github.token }}"
    assert "--source-ref" in steps[admission]["run"]
    assert "python-compat" in yaml.safe_load(sources[1][0].read_text())["jobs"]


def test_nonadopter_installer_keeps_all_copy_sources_and_never_imports_extension(tmp_path, monkeypatch):
    path = tmp_path / "input.json"; path.write_text(json.dumps(InstallValuesTests.valid_values()))
    monkeypatch.setitem(sys.modules, "rc_python_compatibility", None)
    values = INSTALL.load_install_values(path)
    copies = [(Path("source"), Path(".github/workflows/release.yml"))]
    assert INSTALL.install_sources(copies, values, tmp_path) == copies



def test_two_real_installs_have_identical_bytes(tmp_path, monkeypatch):
    if not (INSTALL.PACKAGE_ROOT / ".sc-publish-source-root").is_file():
        pytest.skip("installed consumers do not re-run the package source installer")
    values = InstallValuesTests.valid_values()
    values["rc_python_compatibility"] = copy.deepcopy(manifest()["rc_python_compatibility"])
    values["rc_python_compatibility"]["exclusions"] = {}
    config = tmp_path / "install.json"; config.write_text(json.dumps(values))
    consumer = tmp_path / "consumer"; consumer.mkdir()
    monkeypatch.setattr(sys, "argv", [str(INSTALL.PACKAGE_ROOT / "install.py"), "--input", str(config), str(consumer)])
    assert INSTALL.main() == 0
    before = {path.relative_to(consumer): path.read_bytes() for path in consumer.rglob("*") if path.is_file()}
    assert b"python-compat:" in before[Path(".github/workflows/release-candidate.yml")]
    assert INSTALL.main() == 0
    after = {path.relative_to(consumer): path.read_bytes() for path in consumer.rglob("*") if path.is_file()}
    assert after == before
