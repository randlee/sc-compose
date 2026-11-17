"""Controlled publication inventories; no network, sleeps, tags or uploads."""
import argparse
import base64
import os
import json
from pathlib import Path
import subprocess
import sys
import tomllib

import pytest
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from publication_continuity import ContinuityError, check, configuration, normalize_asset, planned_asset_patterns, write_summary
from publication_inventory import Inventory
from test_install import INSTALL, InstallValuesTests


def manifest():
    return {"publication_continuity": {}, "project": {"archive_prefix": "tool"},
            "release_binaries": [{"name": "tool"}],
            "release_targets": [{"target": "cpu-platform", "archive": "zip"}],
            "crates": [{"package": "tool-core", "publish": True}], "channels": {}}


def previous():
    return {"tag": "v1.2.3", "assets": ["tool_1.2.3_cpu-platform.zip", "checksums.txt"],
            "publications": ["crate:tool-core"]}


def approval(identifier):
    return {"identifier": identifier, "reason": "The user approved retiring this publication",
            "approved_by": "release-owner", "approval_ref": "https://example.test/team/repo/issues/12"}


def test_unchanged_surface_and_version_normalized_identifiers():
    result = check(manifest(), previous())
    assert result["status"] == "pass"
    assert "asset:tool_{version}_cpu-platform.zip" in result["published"]
    assert normalize_asset("tool_11.2.30_cpu.zip", "1.2.3") == "tool_11.2.30_cpu.zip"


def test_planted_dropped_asset_is_refused_and_exact_approved_deprecation_is_accepted():
    old = previous()
    old["assets"].append("binding-cpu-platform.zip")
    with pytest.raises(ContinuityError, match="asset:binding-cpu-platform.zip"):
        check(manifest(), old)
    values = manifest()
    values["publication_continuity"]["deprecations"] = [approval("asset:binding-cpu-platform.zip")]
    assert check(values, old)["deprecated"] == ["asset:binding-cpu-platform.zip"]


@pytest.mark.parametrize("field,value", [("reason", ""), ("approved_by", ""),
    ("approval_ref", "not an approval"), ("identifier", ""), ("identifier", "invented:item")])
def test_planted_malformed_deprecations_are_refused(field, value):
    row = approval("asset:binding.zip")
    row[field] = value
    values = manifest()
    values["publication_continuity"]["deprecations"] = [row]
    with pytest.raises(ContinuityError):
        check(values, previous())


@pytest.mark.parametrize("config", [None, False, "yes", [], {"enabled": False},
    {"deprecations": {}}, {"deprecations": [{}]}])
def test_bad_or_bypass_configuration_fails_closed(config):
    values = manifest()
    values["publication_continuity"] = config
    with pytest.raises(ContinuityError):
        configuration(values)


def test_stale_deprecations_are_notes_but_retained_items_are_refused():
    values = manifest()
    values["publication_continuity"]["deprecations"] = [approval("crate:never-published")]
    assert "Stale deprecation" in check(values, previous())["notes"][0]
    values["publication_continuity"]["deprecations"] = [approval("crate:tool-core")]
    with pytest.raises(ContinuityError, match="does not name a dropped"):
        check(values, previous())


@pytest.mark.parametrize("identifier", ["crate:removed", "pypi:removed", "npm:@scope/removed",
    "go:example.test/team/repo/binding", "homebrew:team/tap/Formula/removed.rb",
    "scoop:team/bucket/removed.json", "winget:Vendor.Removed"])
def test_every_published_channel_drop_is_blocking(identifier):
    old = previous()
    old["publications"].append(identifier)
    with pytest.raises(ContinuityError, match="would drop"):
        check(manifest(), old)


def test_non_adopter_is_unchanged_and_never_needs_inventory_or_repository(tmp_path):
    assert check({}, None) == {"status": "disabled"}
    path = tmp_path / "manifest.toml"
    path.write_text('schema_version = 1\n')
    script = Path(__file__).parents[1] / "publication_continuity.py"
    env = {key: value for key, value in os.environ.items() if key not in {"GITHUB_REPOSITORY", "GH_TOKEN"}}
    env["PATH"] = ""
    result = subprocess.run([sys.executable, str(script), "--manifest", str(path), "--version", "1.3.0"],
                            capture_output=True, text=True, timeout=30, env=env)
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout) == {"status": "disabled"}


def test_first_release_and_go_tags_without_a_github_release():
    assert check(manifest(), {"tag": None, "assets": [], "publications": []})["status"] == "pass"
    with pytest.raises(ContinuityError, match="go:"):
        check(manifest(), {"tag": None, "assets": [], "publications": ["go:example.test/old"]})


def test_manifest_and_installer_round_trip_only_opt_in_values(tmp_path):
    values = InstallValuesTests.valid_values()
    path = tmp_path / "input.json"
    path.write_text(json.dumps(values))
    plain = INSTALL.load_install_values(path)
    assert INSTALL.template_values(plain)["has_continuity"] is False
    values["publication_continuity"] = {"deprecations": [approval("crate:old")]}
    path.write_text(json.dumps(values))
    parsed = INSTALL.load_install_values(path)
    rendered = INSTALL.template_values(parsed)
    assert rendered["has_continuity"] is True
    assert tomllib.loads("config=" + rendered["publication_continuity"])["config"] == values["publication_continuity"]
    values["publication_continuity"]["deprecations"][0]["approved_by"] = ""
    path.write_text(json.dumps(values))
    with pytest.raises(argparse.ArgumentTypeError):
        INSTALL.load_install_values(path)


def encoded(text):
    return {"encoding": "base64", "content": base64.b64encode(text.encode()).decode()}


def test_actual_previous_inventory_excludes_current_retry_and_queries_old_declared_package():
    calls, probes = [], []
    old_manifest = '[[crates]]\npackage="old-core"\npublish=true\n'
    responses = {
        "repos/team/repo/releases?per_page=100&page=1": [
            {"id": 9, "tag_name": "v1.3.0", "draft": False, "prerelease": False},
            {"id": 8, "tag_name": "v1.2.3", "draft": False, "prerelease": False}],
        "repos/team/repo/releases/8/assets?per_page=100&page=1": [{"name": "old-1.2.3.zip", "state": "uploaded"}],
        "repos/team/repo/contents/release/publish-artifacts.toml?ref=v1.2.3": encoded(old_manifest),
        "repos/team/repo/tags?per_page=100&page=1": [{"name": "binding/v1.2.3"}, {"name": "binding/v1.3.0"}],
        "repos/team/repo/contents/binding/go.mod?ref=binding%2Fv1.2.3": encoded("module example.test/team/repo/binding\n"),
    }
    def query(path):
        calls.append(path)
        assert path in responses, path
        return 200, responses[path]
    def probe(url):
        probes.append(url)
        return "published"
    old = Inventory("team/repo", query=query, probe=probe).previous("1.3.0", "release/publish-artifacts.toml")
    assert old == {"tag": "v1.2.3", "assets": ["old-1.2.3.zip"],
                   "publications": ["crate:old-core", "go:example.test/team/repo/binding"], "notes": []}
    assert probes == ["https://crates.io/api/v1/crates/old-core/1.2.3"]
    assert not any("releases/9/" in path for path in calls)


@pytest.mark.parametrize("status,value", [(403, None), (500, None), (200, {}),
    (200, [{"id": 1, "tag_name": "v1.2.3"}])])
def test_incomplete_previous_listing_is_not_treated_as_first_release(status, value):
    with pytest.raises(ContinuityError):
        Inventory("team/repo", query=lambda _: (status, value)).previous("1.3.0", "manifest.toml")


def test_unknown_registry_state_fails_closed():
    inventory = Inventory("team/repo", query=lambda _: pytest.fail("no GitHub query"), probe=lambda _: "unknown")
    with pytest.raises(ContinuityError, match="indeterminate"):
        inventory.registries({"crates": [{"package": "old", "publish": True}]}, "1.2.3")


def test_missing_historical_manifest_does_not_guess_a_registry_surface():
    def query(path):
        if "/releases?" in path:
            return 200, [{"id": 8, "tag_name": "v1.2.3", "draft": False, "prerelease": False}]
        if "/assets?" in path:
            return 200, []
        if "/contents/" in path:
            return 404, None
        pytest.fail(path)
    with pytest.raises(ContinuityError, match="inventory cannot be established"):
        Inventory("team/repo", query=query).previous("1.3.0", "manifest.toml")


def test_workflows_block_on_the_same_checker_before_tag_and_in_preflight_summary():
    workflows = Path(__file__).parents[2] / "workflows"
    release = yaml.safe_load((workflows / "release.yml").read_text())
    steps = release["jobs"]["gate-and-tag"]["steps"]
    admission = next(i for i, step in enumerate(steps) if "publication_continuity.py" in step.get("run", ""))
    mutation = next(i for i, step in enumerate(steps) if "git push" in step.get("run", ""))
    assert admission < mutation
    assert "--source-ref" in steps[admission]["run"]
    assert not steps[admission].get("continue-on-error", False)
    preflight = yaml.safe_load((workflows / "release-preflight.yml").read_text())
    steps = preflight["jobs"]["preflight"]["steps"]
    assert any(step.get("id") == "publication_continuity" and "publication_continuity.py" in step["run"] for step in steps)
    gate = next(step for step in steps if step.get("name") == "Deny release after complete preflight summary")
    assert gate["env"]["PUBLICATION_CONTINUITY"] == "${{ steps.publication_continuity.outcome }}"
    assert 'record publication-continuity "${PUBLICATION_CONTINUITY}"' in gate["run"]


def test_release_after_approved_drop_needs_no_new_deprecation():
    values = manifest()
    values["publication_continuity"]["deprecations"] = [approval("go:example.test/retired")]
    before = previous()
    before["publications"].append("go:example.test/retired")
    assert check(values, before)["deprecated"] == ["go:example.test/retired"]
    after = previous()
    after["tag"] = "v1.3.0"
    after["assets"] = []
    result = check(values, after)
    assert result["deprecated"] == []
    assert "go:example.test/retired" in result["notes"][0]


def test_old_go_tag_does_not_reappear_in_later_baseline():
    inventory = Inventory("team/repo", query=lambda path: (200, [{"name": "binding/v1.1.0"}])
                          if "/tags?" in path else pytest.fail(path))
    assert inventory.go_publications((1, 2, 3)) == set()


@pytest.mark.parametrize("extra", [False, True])
def test_duplicate_and_extra_field_deprecations_fail(extra):
    values = manifest()
    row = approval("crate:old")
    values["publication_continuity"]["deprecations"] = [dict(row, unexpected="value")] if extra else [row, row]
    with pytest.raises(ContinuityError, match="exactly|duplicate"):
        configuration(values)


def test_skips_drafts_prereleases_and_nonsemver_tags_with_notes():
    releases = [{"id": i, "tag_name": tag, "draft": draft, "prerelease": pre}
                for i, (tag, draft, pre) in enumerate([
                    ("v9.0.0", True, False), ("v8.0.0", False, True),
                    ("notes-only", False, False), ("v1.3.0-rc.1", False, False)])]
    inventory = Inventory("team/repo", query=lambda path: (200, releases)
                          if "/releases?" in path else pytest.fail(path))
    result = inventory.previous("1.3.0", "manifest.toml")
    assert result["tag"] is None
    assert result["publications"] == []
    assert result["notes"] == ["Skipped non-stable-semver release tag: notes-only",
                               "Skipped non-stable-semver release tag: v1.3.0-rc.1"]


def test_unuploaded_asset_is_not_a_complete_inventory():
    def query(path):
        if "/releases?" in path:
            return 200, [{"id": 1, "tag_name": "v1.2.3", "draft": False, "prerelease": False}]
        if "/assets?" in path:
            return 200, [{"name": "broken.zip", "state": "new"}]
        pytest.fail(path)
    with pytest.raises(ContinuityError, match="incomplete asset"):
        Inventory("team/repo", query=query).previous("1.3.0", "manifest.toml")


def test_pagination_is_bounded_and_never_silently_truncates():
    calls = []
    def query(path):
        calls.append(path)
        return 200, [{}] * 100
    with pytest.raises(ContinuityError, match="pagination limit"):
        list(Inventory("team/repo", query=query).pages("repos/team/repo/releases"))
    assert len(calls) == 100
    assert calls[-1].endswith("page=100")


def test_duplicate_normalized_asset_families_fail():
    old = previous()
    old["assets"] += ["tool_v1.2.3_cpu-platform.zip"]
    with pytest.raises(ContinuityError, match="duplicate asset family"):
        check(manifest(), old)


@pytest.mark.parametrize("status,expected", [(200, True), (404, False), (403, None)])
def test_other_channel_inventory_uses_declared_providers(status, expected):
    channels = {"homebrew": {"tap_repository": "team/tap", "formulas": [{"path": "Formula/tool.rb"}]},
                "scoop": {"bucket_repository": "team/bucket", "manifest_path": "tool.json"},
                "winget": {"identifier": "Vendor.Tool"}}
    calls = []
    def query(path):
        calls.append(path)
        value = [{"name": "1.2.3"}] if "winget-pkgs" in path else encoded("published")
        return status, value
    inventory = Inventory("team/repo", query=query)
    if expected is None:
        with pytest.raises(ContinuityError, match="HTTP 403"):
            inventory.other_channels({"channels": channels})
    else:
        result = inventory.other_channels({"channels": channels})
        assert result == ({"homebrew:team/tap/Formula/tool.rb", "scoop:team/bucket/tool.json",
                           "winget:Vendor.Tool"} if expected else set())
        assert calls == ["repos/team/tap/contents/Formula/tool.rb?ref=HEAD",
                         "repos/team/bucket/contents/tool.json?ref=HEAD",
                         "repos/microsoft/winget-pkgs/contents/manifests/v/Vendor/Tool"]


def test_previous_release_fixture_preserves_assets_and_all_published_channels():
    fixtures = Path(__file__).parent / "fixtures/continuity"
    values = tomllib.loads((fixtures / "previous-manifest.toml").read_text())
    values["publication_continuity"] = {}
    old = json.loads((fixtures / "previous-inventory.json").read_text())
    assert check(values, old)["status"] == "pass"
    values["crates"] = [row for row in values["crates"] if row["package"] != "example-core"]
    with pytest.raises(ContinuityError, match="crate:example-core"):
        check(values, old)


def test_wheel_npm_and_go_asset_patterns_require_the_declared_platforms():
    fixtures = Path(__file__).parent / "fixtures/continuity"
    values = tomllib.loads((fixtures / "previous-manifest.toml").read_text())
    values["npm_packages"] = [{"name": "@team/tool"}]
    values["go_modules"] = [{"module": "example.test/tool", "artifact_prefix": "binding",
                              "targets": [{"target": "aarch64-apple-darwin"}]}]
    patterns = planned_asset_patterns(values)
    matches = lambda name: any(pattern.fullmatch(name) for pattern in patterns)
    assert matches("team-tool-1.9.0.tgz")
    assert matches("binding-aarch64-apple-darwin.zip")
    assert not matches("binding-x86_64-apple-darwin.zip")
    assert matches("example_python-1.9.0-cp39-abi3-macosx_11_0_arm64.whl")
    assert not matches("example_python-1.9.0-cp39-abi3-win32.whl")
    values["python_distributions"][0]["wheels"] = ["macos-latest"]
    with pytest.raises(ContinuityError, match="continuity requires explicit platform wheel declarations"):
        planned_asset_patterns(values)


def test_summary_appends_labeled_approved_drop_and_notes(tmp_path):
    summary = tmp_path / "summary.md"
    summary.write_text("Existing summary\n")
    row = approval("crate:old")
    row["reason"] = "Retire <unsafe>"
    write_summary({"status": "pass", "deprecated": ["crate:old"], "notes": ["Stale entry"]},
                  {"crate:old": row}, summary)
    text = summary.read_text()
    assert text.startswith("Existing summary\n")
    assert "User-approved deprecated publications" in text
    assert "crate:old" in text and "Retire &lt;unsafe&gt;" in text
    assert row["approval_ref"] in text and "Stale entry" in text


@pytest.mark.parametrize("outcome", ["success", "failure", "cancelled", "skipped", "unknown", ""])
def test_preflight_executes_continuity_denial_for_every_non_success(outcome):
    path = Path(__file__).parents[2] / "workflows/release-preflight.yml"
    steps = yaml.safe_load(path.read_text())["jobs"]["preflight"]["steps"]
    gate = next(step for step in steps if step.get("name") == "Deny release after complete preflight summary")
    env = dict(os.environ, **{key: "success" for key in gate["env"]})
    env.update(HAS_CRATES="false", PUBLICATION_CONTINUITY=outcome)
    result = subprocess.run(["bash", "-c", gate["run"]], env=env, capture_output=True, text=True, timeout=10)
    assert result.returncode == (0 if outcome == "success" else 1), result.stdout + result.stderr


def test_explicit_wheel_platform_is_exact():
    values = {"python_distributions": [{"name": "native-tool", "wheels": [{
        "id": "win-arm", "os": "windows-11-arm", "target": "aarch64-pc-windows-msvc",
        "platform": "win_arm64"}]}]}
    patterns = planned_asset_patterns(values)
    assert any(pattern.fullmatch("native_tool-1.3.0-cp39-abi3-win_arm64.whl") for pattern in patterns)
    assert not any(pattern.fullmatch("native_tool-1.3.0-cp39-abi3-win_amd64.whl") for pattern in patterns)


def test_changed_wheel_platform_is_a_drop():
    fixtures = Path(__file__).parent / "fixtures/continuity"
    values = tomllib.loads((fixtures / "previous-manifest.toml").read_text())
    values["publication_continuity"] = {}
    values["python_distributions"][0]["wheels"][0].update(platform="macosx_15_0_arm64", deployment_target="15.0")
    old = json.loads((fixtures / "previous-inventory.json").read_text())
    with pytest.raises(ContinuityError, match="would drop.*macosx_11_0_arm64"):
        check(values, old)


@pytest.mark.parametrize("case", ["nonadopter", "adopter", "invalid-ref", "missing-at-commit"])
def test_source_ref_uses_committed_policy_and_ref_errors_fail_closed(tmp_path, case):
    def git(*args):
        return subprocess.run(["git", *args], cwd=tmp_path, check=True,
                              capture_output=True, text=True).stdout.strip()
    git("init")
    git("config", "user.email", "fixture@example.test")
    git("config", "user.name", "Fixture")
    path = tmp_path / "manifest.toml"
    path.write_text("schema_version=1\n" + ("[publication_continuity]\n" if case == "adopter" else ""))
    git("add", "manifest.toml")
    git("commit", "-m", "fixture")
    commit = git("rev-parse", "HEAD")
    # The checkout intentionally disagrees with the committed declaration.
    path.write_text("schema_version=1\n")
    result = subprocess.run([sys.executable, str(Path(__file__).parents[1] / "publication_continuity.py"),
                             "--manifest", "absent.toml" if case == "missing-at-commit" else "manifest.toml",
                             "--version", "1.3.0", "--source-ref", "invalid-ref" if case == "invalid-ref" else commit,
                             "--repository", ""],
                            cwd=tmp_path, text=True, capture_output=True, timeout=10)
    if case in {"nonadopter", "missing-at-commit"}:
        assert result.returncode == 0, result.stderr
        assert json.loads(result.stdout) == {"status": "disabled"}
    else:
        assert result.returncode == 1
        assert "disabled" not in result.stdout
        assert ("repository must be OWNER/REPO" if case == "adopter" else "rev-parse") in result.stderr



def test_nonadopter_install_does_not_import_continuity(tmp_path, monkeypatch):
    path = tmp_path / "input.json"
    path.write_text(json.dumps(InstallValuesTests.valid_values()))
    monkeypatch.setitem(sys.modules, "publication_continuity", None)
    assert "publication_continuity" not in INSTALL.load_install_values(path)


def test_source_ref_refuses_absolute_manifest_path(tmp_path):
    result = subprocess.run([sys.executable, str(Path(__file__).parents[1] / "publication_continuity.py"),
                             "--manifest", str(tmp_path / "manifest.toml"), "--version", "1.3.0",
                             "--source-ref", "HEAD"], capture_output=True, text=True, timeout=10)
    assert result.returncode == 1
    assert "repository-relative" in result.stderr
