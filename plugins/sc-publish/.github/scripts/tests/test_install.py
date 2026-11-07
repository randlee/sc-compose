"""Unit tests for the repository-neutral publish-kit installer."""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch


INSTALLER = next(path / "install.py" for path in Path(__file__).resolve().parents if (path / "install.py").is_file())
SPEC = importlib.util.spec_from_file_location("sc_publish_install", INSTALLER)
assert SPEC is not None and SPEC.loader is not None
INSTALL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALL)


class InstallValuesTests(unittest.TestCase):
    @staticmethod
    def valid_values() -> dict[str, object]:
        return {
            "schema_version": 1,
            "project": {
                "name": "example",
                "archive_prefix": "example",
                "description": "Example release package",
                "homepage": "https://example.test/example",
                "license": "MIT",
                "readme_dependency_crate": "example-core",
                "renderer_archive_path": "bin/example",
            },
            "release_targets": [
                {"target": "x86_64-unknown-linux-gnu", "os": "ubuntu-latest", "archive": "tar.gz"}
            ],
            "crates": [
                {
                    "artifact": "example-core",
                    "package": "example-core",
                    "cargo_toml": "crates/example-core/Cargo.toml",
                    "publish": True,
                    "publish_order": 1,
                    "wait_after_publish_seconds": 0,
                },
                {
                    "artifact": "example-python",
                    "package": "example-python",
                    "cargo_toml": "crates/example-python/Cargo.toml",
                    "publish": False,
                    "publish_order": 0,
                    "wait_after_publish_seconds": 0,
                },
            ],
            "release_binaries": [
                {
                    "name": "example",
                    "bundled_paths": [
                        {
                            "source": "docs",
                            "destination": "share/doc/example",
                            "homebrew_destination_components": ["pkgshare"],
                        }
                    ],
                }
            ],
            "python_packages": [
                {
                    "artifact": "example-wheel",
                    "package": "example",
                    "manifest": "python/pyproject.toml",
                    "module": "example",
                    "publish": "pypi",
                }
            ],
            "python_distributions": [
                {
                    "name": "example",
                    "source": "python",
                    "cargo_manifest": "crates/example-python/Cargo.toml",
                    "module_path": "python/example",
                    "sdist": True,
                    "wheels": ["ubuntu-latest", "macos-latest", "windows-latest"],
                },
                {
                    "name": "example-plugin",
                    "source": "plugin",
                    "build_system": "setuptools",
                    "module_path": "plugin/src/example_plugin",
                    "sdist": True,
                    "wheels": ["ubuntu-latest"],
                },
            ],
            "channels": {
                "pypi": {
                    "workflow": "pypi-publish.yml",
                    "dispatch_inputs": {"target": "production"},
                    "credential_rehearsal_inputs": {"target": "testpypi"},
                    "test_repository": "testpypi",
                    "production_repository": "pypi",
                },
                "homebrew": {
                    "workflow": "homebrew-publish.yml",
                    "dispatch_inputs": {},
                    "tap_repository": "example/tap",
                    "renderer_target": "x86_64-unknown-linux-gnu",
                    "formulas": [
                        {
                            "path": "Formula/example.rb",
                            "template": "release/homebrew/formula.rb.j2",
                            "class": "Example",
                            "binaries": ["example"],
                            "test_binary": "example",
                            "test_command": "--help",
                            "test_output": "Example release package",
                            "release_track": "stable",
                        }
                    ],
                    "assets": [
                        {"key": "macos_arm", "target": "aarch64-apple-darwin"},
                        {"key": "macos_intel", "target": "x86_64-apple-darwin"},
                        {"key": "linux", "target": "x86_64-unknown-linux-gnu"},
                    ],
                },
                "winget": {
                    "workflow": "winget-publish.yml",
                    "dispatch_inputs": {},
                    "identifier": "example.example",
                    "installer_target": "x86_64-pc-windows-msvc",
                },
                "scoop": {
                    "workflow": "scoop-publish.yml",
                    "dispatch_inputs": {},
                    "bucket_repository": "example/scoop-bucket",
                    "manifest_path": "bucket/example.json",
                    "manifest_template": "release/scoop/manifest.json.j2",
                    "installer_target": "x86_64-pc-windows-msvc",
                    "binary": "bin/example.exe",
                    "renderer_target": "x86_64-unknown-linux-gnu",
                },
            },
        }

    def test_optional_registry_visibility_round_trips_without_changing_default(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for declared in (None, False, True):
                values = self.valid_values()
                if declared is not None:
                    values["crates"][0]["verify_registry_visibility"] = declared
                source = root / "install.json"
                source.write_text(json.dumps(values))
                loaded = INSTALL.load_install_values(source)
                rendered = root / "artifacts.toml"
                INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), loaded, rendered)
                crate = tomllib.loads(rendered.read_text())["crates"][0]
                self.assertEqual(crate.get("verify_registry_visibility"), declared)
                self.assertEqual("verify_registry_visibility" in crate, declared is not None)

    def test_registry_visibility_rejects_nonboolean_install_input(self) -> None:
        for value in ("true", 1, None):
            with self.subTest(value=value), tempfile.TemporaryDirectory() as directory:
                values = self.valid_values()
                values["crates"][0]["verify_registry_visibility"] = value
                source = Path(directory) / "install.json"
                source.write_text(json.dumps(values))
                with self.assertRaisesRegex(Exception, "verify_registry_visibility"):
                    INSTALL.load_install_values(source)

    def test_help_explains_the_explicit_install_workflow(self) -> None:
        result = subprocess.run(
            [sys.executable, "-S", str(INSTALLER), "--help"],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--input INSTALL.json", result.stdout)
        self.assertIn("workflow:", result.stdout)
        self.assertIn("caller-owned complete JSON input", result.stdout)

    def test_load_install_values_accepts_the_complete_manifest_contract(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "input.json"
            values = self.valid_values()
            path.write_text(json.dumps(values), encoding="utf-8")
            loaded = INSTALL.load_install_values(path)
        expected = {**values, "preflight_checks": [], "docli": None, "preflight_workflows": []}
        self.assertEqual(loaded, expected)
        self.assertEqual(set(loaded["channels"]), set(INSTALL.CHANNEL_NAMES) - {"npm"})
        self.assertEqual(loaded["python_distributions"][1]["build_system"], "setuptools")

    def test_source_revision_rejects_floating_or_malformed_refs(self) -> None:
        for revision in ("main", "v0.6.0", "abc123", "a" * 39, "g" * 40, "a" * 40 + "\n", ""):
            with self.subTest(revision=revision), tempfile.TemporaryDirectory() as directory:
                values = self.valid_values()
                values["project"]["sc_lint_source_revision"] = revision
                path = Path(directory) / "input.json"
                path.write_text(json.dumps(values))
                with self.assertRaisesRegex(Exception, "sc_lint_source_revision"):
                    INSTALL.load_install_values(path)

    def test_optional_source_revision_round_trips_without_changing_default(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for revision in (None, "a" * 40):
                values = self.valid_values()
                if revision:
                    values["project"]["sc_lint_source_revision"] = revision
                source = root / "install.json"
                source.write_text(json.dumps(values))
                loaded = INSTALL.load_install_values(source)
                rendered = root / "artifacts.toml"
                INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), loaded, rendered)
                project = tomllib.loads(rendered.read_text())["project"]
                self.assertEqual(project.get("sc_lint_source_revision"), revision)

    def test_installed_consumer_resolves_its_rendered_immutable_source(self) -> None:
        if not (INSTALLER.parent / ".sc-publish-source-root").is_file():
            self.skipTest("source installer integration")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            consumer = root / "consumer"
            consumer.mkdir()
            values = self.valid_values()
            values["project"]["sc_lint_source_revision"] = "a" * 40
            contract = root / "install.json"
            contract.write_text(json.dumps(values))
            subprocess.run([sys.executable, INSTALLER, "--input", contract, consumer], check=True, capture_output=True)
            result = subprocess.run([sys.executable, INSTALLER, "--dry-run", "--input", contract, consumer], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            import os
            env = dict(os.environ, GITHUB_WORKSPACE=str(consumer), GITHUB_ENV=str(root / "env"), SC_LINT_SOURCE_REVISION_INPUT="")
            helper = consumer / ".github/scripts/setup_sc_lint_source.py"
            result = subprocess.run([sys.executable, helper, "resolve"], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((root / "env").read_text(), "SC_LINT_SOURCE_REVISION=" + "a" * 40 + "\n")

    def test_load_install_values_rejects_invalid_publish_orders(self) -> None:
        cases = {
            "boolean": (True, "integer"),
            "zero": (0, "positive when publish is true"),
            "negative": (-1, "non-negative integer"),
        }
        for name, (publish_order, message) in cases.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                values = self.valid_values()
                values["crates"][0]["publish_order"] = publish_order
                path = Path(directory) / "input.json"
                path.write_text(json.dumps(values), encoding="utf-8")
                with self.assertRaisesRegex(Exception, message):
                    INSTALL.load_install_values(path)
        with tempfile.TemporaryDirectory() as directory:
            values = self.valid_values()
            values["crates"][1].update(publish=True, publish_order=1)
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaisesRegex(Exception, "unique"):
                INSTALL.load_install_values(path)

    def test_load_install_values_rejects_missing_complete_contract_fields(self) -> None:
        values = self.valid_values()
        del values["project"]["license"]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaisesRegex(Exception, "project.license"):
                INSTALL.load_install_values(path)

    def test_load_install_values_accepts_a_channel_subset(self) -> None:
        values = self.valid_values()
        for name in ("homebrew", "winget", "scoop"):
            del values["channels"][name]
        # renderer_archive_path is only required for homebrew/scoop consumers.
        del values["project"]["renderer_archive_path"]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            loaded = INSTALL.load_install_values(path)
        self.assertEqual(set(loaded["channels"]), {"pypi"})
        template = INSTALL.template_values(loaded)
        self.assertTrue(template["has_channel_pypi"])
        for name in ("homebrew", "winget", "scoop"):
            self.assertFalse(template[f"has_channel_{name}"])
            self.assertIn(name, template["channels"])

    def test_load_install_values_rejects_unknown_channel_names(self) -> None:
        values = self.valid_values()
        values["channels"]["unsupported"] = {"workflow": "unsupported.yml", "dispatch_inputs": {}}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaisesRegex(Exception, "unsupported name"):
                INSTALL.load_install_values(path)

    def test_render_omits_undeclared_channel_tables(self) -> None:
        try:
            import sc_compose  # noqa: F401
        except ModuleNotFoundError:
            self.skipTest("sc-compose bindings are not provisioned in this environment")
        values = self.valid_values()
        for name in ("homebrew", "winget", "scoop"):
            del values["channels"][name]
        del values["project"]["renderer_archive_path"]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            loaded = INSTALL.load_install_values(path)
            output = Path(directory) / "publish-artifacts.toml"
            INSTALL.render_template(
                Path("release/publish-artifacts.toml.j2"), loaded, output
            )
            manifest = tomllib.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(set(manifest["channels"]), {"pypi"})
        self.assertEqual(manifest["channels"]["pypi"]["production_repository"], "pypi")
        self.assertEqual(
            manifest["python_distributions"][1]["build_system"], "setuptools"
        )
        self.assertNotIn("renderer_archive_path", manifest["project"])

    def test_render_round_trips_declared_prerelease_manifest(self) -> None:
        try:
            import sc_compose  # noqa: F401
        except ModuleNotFoundError:
            self.skipTest("sc-compose bindings are not provisioned in this environment")
        values = self.valid_values()
        values["prerelease"] = {
            "tag_prefix": "prerelease/v",
            "tag_script": ".just/prerelease_tag.py",
            "install_root": "~/.example-builds",
            "binaries": ["example"],
            "protected_branches": ["trunk", "release"],
            "selector_dir": {
                "darwin": "/opt/homebrew/bin",
                "linux": "~/.local/bin",
                "windows": r"%LOCALAPPDATA%\Programs\Example",
            },
            "post_install": "python3 scripts/activate.py --version {version} --stage {stage_dir}",
            "verify": "example --version",
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            loaded = INSTALL.load_install_values(path)
            template = INSTALL.template_values(loaded)
            output = Path(directory) / "publish-artifacts.toml"
            INSTALL.render_template(
                Path("release/publish-artifacts.toml.j2"), loaded, output
            )
            manifest = tomllib.loads(output.read_text(encoding="utf-8"))
        self.assertTrue(template["has_prerelease"])
        self.assertEqual(manifest["prerelease"], values["prerelease"])

    @staticmethod
    def wheels_only_prerelease(values: dict[str, object]) -> dict[str, object]:
        """The beads-cache shape: no release binaries, Python wheels and an sdist only."""
        values["release_binaries"] = []
        values["channels"] = {"pypi": values["channels"]["pypi"]}  # type: ignore[index]
        values["prerelease"] = {
            "tag_prefix": "prerelease/v",
            "tag_script": "scripts/prerelease_tag.py",
            "install_root": "~/.example-builds",
            "binaries": [],
            "protected_branches": ["develop", "main"],
            "selector_dir": {
                "darwin": "~/.example-builds/selectors/darwin",
                "linux": "~/.example-builds/selectors/linux",
                "windows": r"%LOCALAPPDATA%\Example\prerelease-selectors",
            },
            "post_install": "{python} -m pip install --force-reinstall {wheels}",
            "verify": "{python} -m pip show example",
        }
        return values

    def test_render_round_trips_a_wheels_only_prerelease_manifest(self) -> None:
        try:
            import sc_compose  # noqa: F401
        except ModuleNotFoundError:
            self.skipTest("sc-compose bindings are not provisioned in this environment")
        for omit_binaries in (False, True):
            with self.subTest(omit_binaries=omit_binaries):
                values = self.wheels_only_prerelease(self.valid_values())
                if omit_binaries:
                    del values["prerelease"]["binaries"]  # type: ignore[index]
                with tempfile.TemporaryDirectory() as directory:
                    path = Path(directory) / "input.json"
                    path.write_text(json.dumps(values), encoding="utf-8")
                    loaded = INSTALL.load_install_values(path)
                    output = Path(directory) / "publish-artifacts.toml"
                    INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), loaded, output)
                    manifest = tomllib.loads(output.read_text(encoding="utf-8"))
                self.assertEqual(manifest["prerelease"]["binaries"], [])
                self.assertEqual(
                    {key: value for key, value in manifest["prerelease"].items() if key != "binaries"},
                    {key: value for key, value in values["prerelease"].items() if key != "binaries"},  # type: ignore[union-attr]
                )
                self.assertNotIn("release_binaries", manifest)

    def test_load_install_values_rejects_inconsistent_prerelease_binaries(self) -> None:
        values = self.wheels_only_prerelease(self.valid_values())
        values["python_distributions"] = [
            {**entry, "wheels": []} for entry in values["python_distributions"]  # type: ignore[union-attr]
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaisesRegex(argparse.ArgumentTypeError, "prerelease installs nothing"):
                INSTALL.load_install_values(path)
            values["prerelease"]["binaries"] = ["ghost"]  # type: ignore[index]
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaisesRegex(argparse.ArgumentTypeError, "undeclared release binary\\(s\\): ghost"):
                INSTALL.load_install_values(path)

    def test_load_install_values_rejects_ambiguous_python_distribution(self) -> None:
        values = self.valid_values()
        values["python_distributions"][0]["build_system"] = "setuptools"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaisesRegex(Exception, "must not set both"):
                INSTALL.load_install_values(path)

    def test_load_install_values_accepts_preflight_checks_and_docli(self) -> None:
        values = self.valid_values()
        values["preflight_checks"] = [
            {
                "id": "example-gate",
                "description": "Example repository check",
                "run": "bash scripts/example-preflight.sh",
            }
        ]
        values["docli"] = {
            "enabled": True,
            "generate_script": "scripts/generate-cli-docs.sh",
            "drift_paths": ["fixtures/cli/model.json", "site/cli/"],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            loaded = INSTALL.load_install_values(path)
        self.assertEqual(len(loaded["preflight_checks"]), 1)
        self.assertEqual(loaded["docli"]["generate_script"], "scripts/generate-cli-docs.sh")
        template = INSTALL.template_values(loaded)
        self.assertTrue(template["has_docli"])
        self.assertEqual(len(template["preflight_checks"]), 1)

    def test_load_install_values_rejects_empty_docli_drift_paths(self) -> None:
        values = self.valid_values()
        values["docli"] = {
            "enabled": True,
            "generate_script": "scripts/generate-cli-docs.sh",
            "drift_paths": [],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaisesRegex(Exception, "must not be empty"):
                INSTALL.load_install_values(path)

    def test_render_publish_preflight_includes_docli_and_custom_checks(self) -> None:
        try:
            import sc_compose  # noqa: F401
        except ModuleNotFoundError:
            self.skipTest("sc-compose bindings are not provisioned in this environment")
        values = self.valid_values()
        for name in ("homebrew", "winget", "scoop"):
            del values["channels"][name]
        del values["project"]["renderer_archive_path"]
        values["preflight_checks"] = [
            {
                "id": "example-gate",
                "description": "Example repository check",
                "run": "bash scripts/example-preflight.sh",
            }
        ]
        values["docli"] = {
            "enabled": True,
            "generate_script": "scripts/generate-cli-docs.sh",
            "drift_paths": ["fixtures/cli/model.json"],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            loaded = INSTALL.load_install_values(path)
            output = Path(directory) / "publish-preflight.toml"
            INSTALL.render_template(Path("release/publish-preflight.toml.j2"), loaded, output)
            manifest = tomllib.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(manifest["schema_version"], 1)
        self.assertEqual(manifest["docli"]["generate_script"], "scripts/generate-cli-docs.sh")
        check_ids = [check["id"] for check in manifest["checks"]]
        self.assertEqual(check_ids, ["docli-cli-html", "example-gate"])

    def test_package_inventory_excludes_sprint_history_and_keeps_assets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("docs/sprints/nested/history.md", "docs/guide.md", "README.md"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name, encoding="utf-8")
            with patch.object(INSTALL, "PACKAGE_ROOT", root):
                destinations = [dest for _, dest in INSTALL.install_inventory(root / "consumer")]
            self.assertIn(Path("docs/guide.md"), destinations)
            self.assertIn(Path("README.sc-publish.md"), destinations)
            self.assertNotIn(Path("docs/sprints/nested/history.md"), destinations)

    def test_inventory_rejects_unsafe_and_symlink_destinations(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package, consumer = root / "package", root / "consumer"
            package.mkdir()
            consumer.mkdir()
            (package / "asset").write_text("asset", encoding="utf-8")
            with patch.object(INSTALL, "PACKAGE_ROOT", package):
                with patch.object(INSTALL, "RENAMED_FILES", {Path("asset"): Path("../escape")}):
                    with self.assertRaisesRegex(ValueError, "unsafe installer destination"):
                        INSTALL.install_inventory(consumer)
                with patch.object(INSTALL, "RENAMED_FILES", {Path("asset"): Path("C:\\escape")}):
                    with self.assertRaisesRegex(ValueError, "unsafe installer destination"):
                        INSTALL.install_inventory(consumer)
                (consumer / "asset").symlink_to(root / "outside")
                with self.assertRaisesRegex(ValueError, "symlink in installer destination"):
                    INSTALL.install_inventory(consumer)
                (package / "linked").symlink_to(package / "asset")
                with self.assertRaisesRegex(ValueError, "package contains symlink"):
                    INSTALL.install_inventory(consumer)

    def test_install_places_executable_release_helpers_at_every_workflow_path(self) -> None:
        """The byte-exact overlay keeps helpers under .github/scripts/.

        This exercises a real install into a temporary consumer. It prevents a
        parity-only check from accepting workflows that still call an obsolete
        consumer-local scripts/ path.
        """

        if not (INSTALLER.parent / ".sc-publish-source-root").is_file():
            self.skipTest("installed consumers do not re-run the package source installer")

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            consumer = root / "consumer"
            consumer.mkdir()
            input_path = root / "install.json"
            input_path.write_text(json.dumps(self.valid_values()), encoding="utf-8")

            def render_empty_toml(_template: Path, _values: dict[str, object], output: Path) -> None:
                output.write_text("schema_version = 1\n", encoding="utf-8")

            with (
                patch.object(INSTALL, "render_template", side_effect=render_empty_toml),
                patch.object(sys, "argv", [str(INSTALLER), "--input", str(input_path), str(consumer)]),
            ):
                self.assertEqual(INSTALL.main(), 0)

            artifacts = consumer / ".github" / "scripts" / "release_artifacts.py"
            gate = consumer / ".github" / "scripts" / "release_gate.sh"
            self.assertTrue(artifacts.is_file())
            self.assertTrue(gate.is_file())

            # The kit README installs under a kit-owned name and never
            # overwrites the consumer repository's own README.md.
            kit_readme = consumer / "README.sc-publish.md"
            self.assertTrue(kit_readme.is_file())
            self.assertFalse((consumer / "README.md").exists())
            self.assertEqual(
                kit_readme.read_bytes(), (INSTALLER.parent / "README.md").read_bytes()
            )
            readme_text = kit_readme.read_text(encoding="utf-8")
            self.assertIn("byte-for-byte", readme_text)
            self.assertIn("bootstrap_sc_compose.py --venv", readme_text)
            self.assertIn("--dry-run --input", readme_text)
            self.assertIn(".claude/agents/publisher.md", readme_text)
            self.assertIn(".cursor/", readme_text)
            self.assertIn("idempotent", readme_text)

            claude_skill = consumer / ".claude" / "skills" / "prerelease"
            codex_skill = consumer / ".codex" / "skills" / "prerelease" / "SKILL.md"
            self.assertTrue((claude_skill / "SKILL.md").is_file())
            self.assertTrue(codex_skill.is_file())
            self.assertIn(
                "../../../.claude/skills/prerelease/SKILL.md",
                codex_skill.read_text(encoding="utf-8"),
            )
            for mode in ("list.md", "install.md", "publish.md"):
                self.assertTrue((claude_skill / mode).is_file())

            workflows = (
                "release.yml",
                "release-preflight.yml",
                "prerelease-archive.yml",
                "pypi-publish.yml",
                "homebrew-publish.yml",
                "scoop-publish.yml",
                "winget-publish.yml",
            )
            for name in workflows:
                text = (consumer / ".github" / "workflows" / name).read_text(encoding="utf-8")
                self.assertIn(".github/scripts/release_artifacts.py", text, name)
                self.assertNotIn("python3 scripts/release_artifacts.py", text, name)
            release = (consumer / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
            release_candidate = (
                consumer / ".github" / "workflows" / "release-candidate.yml"
            ).read_text(encoding="utf-8")
            preflight = (consumer / ".github" / "workflows" / "release-preflight.yml").read_text(
                encoding="utf-8"
            )
            self.assertIn(".github/scripts/release_gate.sh", release)
            self.assertNotIn("run: scripts/release_gate.sh", release)
            self.assertIn('tag -a "${candidate_tag}" origin/develop', release_candidate)
            self.assertIn('-c user.name="github-actions[bot]"', release_candidate)
            self.assertIn(".github/scripts/release_artifacts.py validate-publish-order", preflight)
            self.assertNotIn("scripts/ci/validate_publish_order.sh", preflight)
            self.assertNotIn("docs/publishing-agent.md", preflight)
            packaged_runtime_files = (
                list((consumer / ".github" / "workflows").glob("*.yml"))
                + list((consumer / ".github" / "scripts").glob("*.py"))
                + list((consumer / ".github" / "scripts").glob("*.sh"))
                + list((consumer / ".github" / "actions").rglob("action.yml"))
            )
            for path in packaged_runtime_files:
                self.assertNotRegex(
                    path.read_text(encoding="utf-8"),
                    r"(?<![./\\w])scripts/",
                    path.relative_to(consumer).as_posix(),
                )

            pending = list((consumer / ".github" / "workflows").glob("*.yml"))
            seen: set[Path] = set()
            while pending:
                source = pending.pop()
                if source in seen:
                    continue
                seen.add(source)
                for action_name in re.findall(
                    r"uses:\s+\./\.github/actions/([^\s]+)", source.read_text(encoding="utf-8")
                ):
                    action = consumer / ".github" / "actions" / action_name / "action.yml"
                    self.assertTrue(action.is_file(), action.relative_to(consumer).as_posix())
                    pending.append(action)

            helper = subprocess.run(
                [sys.executable, str(artifacts), "validate-publish-order", "--help"],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(helper.returncode, 0, helper.stderr)
            (consumer / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/base", "crates/leaf"]\n', encoding="utf-8"
            )
            base = consumer / "crates" / "base"
            leaf = consumer / "crates" / "leaf"
            base.mkdir(parents=True)
            leaf.mkdir(parents=True)
            (base / "Cargo.toml").write_text(
                '[package]\nname = "base"\nversion = "1.0.0"\n', encoding="utf-8"
            )
            (leaf / "Cargo.toml").write_text(
                '[package]\nname = "leaf"\nversion = "1.0.0"\n\n'
                '[dependencies]\nbase = { path = "../base" }\n',
                encoding="utf-8",
            )
            manifest = consumer / "release" / "publish-artifacts.toml"
            manifest.write_text(
                "\n".join(
                    (
                        "schema_version = 1",
                        "",
                        "[[crates]]",
                        'artifact = "base"',
                        'package = "base"',
                        'cargo_toml = "crates/base/Cargo.toml"',
                        "publish = true",
                        "publish_order = 1",
                        "wait_after_publish_seconds = 0",
                        "",
                        "[[crates]]",
                        'artifact = "leaf"',
                        'package = "leaf"',
                        'cargo_toml = "crates/leaf/Cargo.toml"',
                        "publish = true",
                        "publish_order = 2",
                        "wait_after_publish_seconds = 0",
                        "",
                    )
                ),
                encoding="utf-8",
            )
            order = subprocess.run(
                [
                    sys.executable,
                    str(artifacts),
                    "validate-publish-order",
                    "--manifest",
                    str(manifest),
                    "--workspace-toml",
                    str(consumer / "Cargo.toml"),
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(order.returncode, 0, order.stderr)
            self.assertIn("matches the workspace dependency graph", order.stdout)
            gate_check = subprocess.run(
                ["bash", "-n", str(gate)],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(gate_check.returncode, 0, gate_check.stderr)



class PreflightWorkflowTests(unittest.TestCase):
    """Optional consumer-owned reusable workflows called by Release Preflight."""

    KIT_WORKFLOW = INSTALLER.parent / ".github" / "workflows" / "release-preflight.yml"
    DECLARED = [
        {
            "id": "install-matrix",
            "description": "Consumer install matrix",
            "uses": "./.github/workflows/install-matrix.yml",
            "with": {"profile": "release", "strict": True, "attempts": 2},
        },
        {
            "id": "artifact_verify",
            "description": "Consumer artifact verification",
            "uses": "./.github/workflows/artifact-verify.yml",
        },
    ]
    EXPECTED_JOBS = """
  consumer_workflow_install-matrix:
    name: "Consumer install matrix"
    uses: "./.github/workflows/install-matrix.yml"
    with:
      version: ${{ inputs.version }}
      candidate_sha: ${{ github.sha }}
      profile: "release"
      strict: true
      attempts: 2

  consumer_workflow_artifact_verify:
    name: "Consumer artifact verification"
    uses: "./.github/workflows/artifact-verify.yml"
    with:
      version: ${{ inputs.version }}
      candidate_sha: ${{ github.sha }}

  consumer_workflows:
    name: Record consumer preflight workflow outcomes
    if: ${{ always() }}
    needs: [consumer_workflow_install-matrix, consumer_workflow_artifact_verify]
    runs-on: ubuntu-latest
    steps:
      - name: Deny release unless every consumer preflight workflow succeeded
        shell: bash
        env:
          NEEDS: ${{ toJSON(needs) }}
        run: |
"""

    @staticmethod
    def load(values: dict[str, object]) -> dict[str, object]:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            return INSTALL.load_install_values(path)

    def install(self, values: dict[str, object]) -> bytes:
        """Run a real install into a temporary consumer and return its preflight workflow."""
        if not (INSTALLER.parent / ".sc-publish-source-root").is_file():
            self.skipTest("installed consumers do not re-run the package source installer")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            consumer = root / "consumer"
            consumer.mkdir()
            input_path = root / "install.json"
            input_path.write_text(json.dumps(values), encoding="utf-8")

            def render_empty_toml(_template: Path, _values: dict[str, object], output: Path) -> None:
                output.write_text("schema_version = 1\n", encoding="utf-8")

            with patch.object(INSTALL, "render_template", side_effect=render_empty_toml):
                with patch.object(sys, "argv", [str(INSTALLER), "--input", str(input_path), str(consumer)]):
                    self.assertEqual(INSTALL.main(), 0)
                with patch.object(
                    sys, "argv", [str(INSTALLER), "--dry-run", "--input", str(input_path), str(consumer)]
                ):
                    self.assertEqual(INSTALL.main(), 0, "a repeat install must be in sync")
            return (consumer / INSTALL.PREFLIGHT_WORKFLOW).read_bytes()

    def test_absent_or_empty_option_installs_the_kit_workflow_byte_for_byte(self) -> None:
        kit = self.KIT_WORKFLOW.read_bytes()
        values = InstallValuesTests.valid_values()
        self.assertNotIn("preflight_workflows", values)
        self.assertEqual(self.load(values)["preflight_workflows"], [])
        self.assertEqual(INSTALL.preflight_workflow_jobs([]), "")
        self.assertEqual(self.install(values), kit)
        values["preflight_workflows"] = []
        self.assertEqual(self.install(values), kit)

    def test_declared_workflows_append_exact_call_and_outcome_jobs(self) -> None:
        loaded = self.load({**InstallValuesTests.valid_values(), "preflight_workflows": self.DECLARED})
        jobs = INSTALL.preflight_workflow_jobs(loaded["preflight_workflows"])
        self.assertTrue(jobs.startswith(self.EXPECTED_JOBS), jobs)
        self.assertEqual(jobs, INSTALL.preflight_workflow_jobs(loaded["preflight_workflows"]))

        kit = self.KIT_WORKFLOW.read_bytes()
        installed = self.install({**InstallValuesTests.valid_values(), "preflight_workflows": self.DECLARED})
        self.assertEqual(installed, kit + jobs.encode("utf-8"))

        import yaml

        before = yaml.safe_load(kit)
        after = yaml.safe_load(installed)
        # Trigger, inputs, permissions and the kit's own preflight job are unchanged.
        self.assertEqual({key: value for key, value in after.items() if key != "jobs"},
                         {key: value for key, value in before.items() if key != "jobs"})
        self.assertEqual(after["permissions"], "read-all")
        self.assertEqual(after["jobs"]["preflight"], before["jobs"]["preflight"])
        self.assertEqual(
            list(after["jobs"]),
            ["preflight", "consumer_workflow_install-matrix", "consumer_workflow_artifact_verify",
             "consumer_workflows"],
        )
        matrix = after["jobs"]["consumer_workflow_install-matrix"]
        self.assertEqual(matrix["uses"], "./.github/workflows/install-matrix.yml")
        self.assertEqual(
            matrix["with"],
            {"version": "${{ inputs.version }}", "candidate_sha": "${{ github.sha }}",
             "profile": "release", "strict": True, "attempts": 2},
        )
        for job_id in ("consumer_workflow_install-matrix", "consumer_workflow_artifact_verify"):
            self.assertNotIn("secrets", after["jobs"][job_id])
            self.assertNotIn("permissions", after["jobs"][job_id])
        outcome = after["jobs"]["consumer_workflows"]
        self.assertEqual(outcome["if"], "${{ always() }}")
        self.assertEqual(outcome["needs"], ["consumer_workflow_install-matrix", "consumer_workflow_artifact_verify"])

    def test_outcome_job_denies_unless_every_declared_call_succeeded(self) -> None:
        import yaml

        loaded = self.load({**InstallValuesTests.valid_values(), "preflight_workflows": self.DECLARED})
        jobs = yaml.safe_load("jobs:\n" + INSTALL.preflight_workflow_jobs(loaded["preflight_workflows"]))
        script = jobs["jobs"]["consumer_workflows"]["steps"][0]["run"]
        cases = {
            "success": ({"a": "success", "b": "success"}, 0),
            "failure": ({"a": "success", "b": "failure"}, 1),
            "skipped": ({"a": "skipped", "b": "success"}, 1),
            "cancelled": ({"a": "cancelled", "b": "success"}, 1),
        }
        for label, (results, expected) in cases.items():
            with self.subTest(label), tempfile.TemporaryDirectory() as directory:
                summary = Path(directory) / "summary.md"
                needs = {job: {"result": result, "outputs": {}} for job, result in results.items()}
                completed = subprocess.run(
                    ["bash", "-c", script],
                    env={"PATH": "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin",
                         "NEEDS": json.dumps(needs), "GITHUB_STEP_SUMMARY": str(summary)},
                    capture_output=True, text=True, check=False,
                )
                self.assertEqual(completed.returncode, expected, completed.stderr)
                for job, result in results.items():
                    self.assertIn(f"| {job} | {result} |", summary.read_text(encoding="utf-8"))
                if expected:
                    failed = ", ".join(job for job, result in sorted(results.items()) if result != "success")
                    self.assertIn(f"did not succeed: {failed}", completed.stderr)

    def test_invalid_declarations_are_refused_before_install(self) -> None:
        base = dict(self.DECLARED[1])
        cases = {
            "must match": {**base, "id": "Bad Id"},
            "with.version must be": {**base, "with": {"version": "1.0.0"}},
            "with.candidate_sha must be": {**base, "with": {"candidate_sha": "abc"}},
            "with.bad name must be": {**base, "with": {"bad name": "x"}},
            "must be a string, boolean or integer": {**base, "with": {"items": ["x"]}},
            "unknown field": {**base, "secrets": "inherit"},
            "uses": {key: value for key, value in base.items() if key != "uses"},
        }
        for message, entry in cases.items():
            with self.subTest(message), self.assertRaisesRegex(argparse.ArgumentTypeError, message):
                self.load({**InstallValuesTests.valid_values(), "preflight_workflows": [entry]})
        with self.assertRaisesRegex(argparse.ArgumentTypeError, "ids must be unique"):
            self.load({**InstallValuesTests.valid_values(), "preflight_workflows": [base, base]})


if __name__ == "__main__":
    unittest.main()
