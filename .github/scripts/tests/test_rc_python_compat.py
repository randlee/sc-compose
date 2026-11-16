"""The release gate accepts only complete RC evidence for the candidate SHA."""

import importlib.util
from pathlib import Path
from unittest.mock import patch

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / ".github/scripts/check_rc_python_compat.py"
spec = importlib.util.spec_from_file_location("check_rc_python_compat", SCRIPT)
compat = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compat)
MANIFEST = ROOT / "release/publish-artifacts.toml"
CANDIDATE = "a" * 40
PLATFORMS = ["ubuntu-latest", "ubuntu-24.04-arm", "macos-latest", "macos-15-intel", "windows-latest", "windows-11-arm"]


def evidence(candidate=CANDIDATE):
    return [{"name": f"python-compat ({os_name}, {candidate})", "conclusion": "success"} for os_name in PLATFORMS]


def check_jobs(jobs):
    def api(endpoint):
        if "/workflows/" in endpoint:
            return [{"workflow_runs": [{"id": 42, "conclusion": "success"}]}]
        return [{"jobs": jobs}]

    with patch.object(compat, "pages", side_effect=api):
        return compat.check(CANDIDATE, MANIFEST, "owner/repo")


def test_accepts_only_complete_success_for_candidate():
    assert check_jobs(evidence())
    assert not check_jobs(evidence("b" * 40))
    assert not check_jobs(evidence()[:-1])
    jobs = evidence()
    jobs[0]["conclusion"] = "failure"
    assert not check_jobs(jobs)


def test_missing_or_failed_run_and_api_failure_fail_closed():
    with patch.object(compat, "pages", return_value=[{"workflow_runs": [{"id": 42, "conclusion": "failure"}]}]):
        assert not compat.check(CANDIDATE, MANIFEST, "owner/repo")
    with patch.object(compat, "pages", side_effect=OSError("offline")):
        with patch.dict("os.environ", {"GITHUB_REPOSITORY": "owner/repo"}):
            with patch("sys.argv", [str(SCRIPT), CANDIDATE, str(MANIFEST)]):
                assert compat.main() == 1


def test_publish_gate_runs_before_tag_and_grants_read_permission():
    workflow = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())
    assert workflow["permissions"]["actions"] == "read"
    steps = workflow["jobs"]["gate-and-tag"]["steps"]
    gate = next(i for i, step in enumerate(steps) if step.get("name") == "Require release-candidate Python compatibility")
    tag = next(i for i, step in enumerate(steps) if step.get("id") == "release-ref")
    assert gate < tag
    assert '"${CANDIDATE_TAG}^{commit}"' in steps[gate]["run"]
    assert 'check_rc_python_compat.py "$candidate_sha"' in steps[gate]["run"]
    assert steps[gate]["env"]["GH_TOKEN"] == "${{ github.token }}"
