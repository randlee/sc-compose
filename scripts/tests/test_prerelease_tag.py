"""Deterministic tests for scripts/prerelease_tag.py.

Every test drives the script with controlled inputs: a throwaway git repository
whose `origin` is a local bare repository (no network), and an injected
registry lookup. No test touches a real remote, tag or registry, and none
depends on timing or scheduling.
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("prerelease_tag", ROOT / "scripts" / "prerelease_tag.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

IDENTITY_VARIABLES = ("GIT_AUTHOR_NAME", "GIT_AUTHOR_EMAIL", "GIT_COMMITTER_NAME", "GIT_COMMITTER_EMAIL", "EMAIL")
HOSTILE_GIT_ENV = {
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_SYSTEM": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_CONFIG_COUNT": "1",
    "GIT_CONFIG_KEY_0": "user.useConfigOnly",
    "GIT_CONFIG_VALUE_0": "true",
}
MANIFEST = """\
[prerelease]
tag_prefix = "prerelease/v"
tag_script = "scripts/prerelease_tag.py"
protected_branches = ["develop", "main"]
"""
CARGO = """\
[workspace]
members = ["crates/a"]

[workspace.package]
version = "{v}"
edition = "2024"

[workspace.dependencies]
serde = {{ version = "1" }}
sc-sha = {{ path = "crates/sc-sha", version = "{v}" }}
sc-composer = {{ path = "crates/sc-composer", version = "{v}" }}
"""
LOCK = """\
version = 4

[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "sc-sha"
version = "{v}"

[[package]]
name = "sc-composer"
version = "{v}"
dependencies = [
 "serde",
]
"""
IDENTITY_NAME = "Fixture Tagger"
IDENTITY_EMAIL = "fixture@example.invalid"
PYPROJECT = '[project]\nname = "{n}"\nversion = "{v}"\n'
PYPROJECTS = {
    "bindings/sc-sha-python/pyproject.toml": "sc-sha",
    "bindings/python/pyproject.toml": "sc-compose",
    "bindings/sc-composer-beads-python/pyproject.toml": "sc-composer-beads",
}


def git(cwd: Path, *args: str) -> str:
    # No identity is injected here: fixture repositories carry their own local
    # user.name/user.email, so the script under test and this helper see the same
    # deterministic identity and neither depends on machine or global config.
    done = subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True)
    return done.stdout


class Fixture:
    def __init__(self, directory: Path, version: str = "1.8.0", branch: str = "feature/prep") -> None:
        self.origin = directory / "origin.git"
        self.work = directory / "work"
        subprocess.run(["git", "init", "--bare", "-q", str(self.origin)], check=True)
        self.work.mkdir()
        git(self.work, "init", "-q", "-b", "develop")
        git(self.work, "config", "user.name", IDENTITY_NAME)
        git(self.work, "config", "user.email", IDENTITY_EMAIL)
        git(self.work, "config", "commit.gpgsign", "false")
        git(self.work, "config", "tag.gpgsign", "false")
        git(self.work, "remote", "add", "origin", str(self.origin))
        self.write("release/publish-artifacts.toml", MANIFEST)
        self.write("Cargo.toml", CARGO.format(v=version))
        self.write("Cargo.lock", LOCK.format(v=version))
        for path, name in PYPROJECTS.items():
            self.write(path, PYPROJECT.format(n=name, v=version))
        git(self.work, "add", "-A")
        git(self.work, "commit", "-q", "-m", "base")
        if branch != "develop":
            git(self.work, "checkout", "-q", "-b", branch)

    def write(self, relative: str, text: str) -> None:
        path = self.work / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def snapshot(self) -> tuple[str, str, str]:
        return (
            git(self.work, "rev-parse", "HEAD"),
            git(self.work, "tag", "--list") + git(self.work, "status", "--porcelain"),
            git(self.origin, "for-each-ref"),
        )

    def run(self, *argv: str, published: set[str] | None = None) -> tuple[int | str, str]:
        out = io.StringIO()
        gone = published or set()
        with contextlib.redirect_stdout(out):
            try:
                code = MODULE.main(list(argv), root=self.work, registry_published=lambda v: v in gone)
            except SystemExit as error:
                code = str(error.code)
        return code, out.getvalue()


class PrereleaseTag(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        # Hostile identity environment: no identity variables, null global and
        # system config, and no automatic identity detection. Only the
        # fixture-local config can satisfy `git commit` and `git tag -a`.
        patcher = mock.patch.dict(os.environ, HOSTILE_GIT_ENV)
        patcher.start()
        self.addCleanup(patcher.stop)
        for name in IDENTITY_VARIABLES:
            os.environ.pop(name, None)
        self.fx = Fixture(Path(self.tmp.name))

    # --- dry-run -----------------------------------------------------------
    def test_dry_run_uses_unused_version_and_mutates_nothing(self) -> None:
        before = self.fx.snapshot()
        code, out = self.fx.run("--dry-run")
        self.assertEqual(code, 0)
        self.assertIn("would create tag prerelease/v1.8.0", out)
        self.assertIn("local-only", out)
        self.assertEqual(self.fx.snapshot(), before)

    def test_dry_run_names_the_patch_bump_and_never_a_minor_bump(self) -> None:
        git(self.fx.work, "tag", "prerelease/v1.8.0")
        git(self.fx.work, "tag", "v1.8.1")
        before = self.fx.snapshot()
        code, out = self.fx.run("--dry-run")
        self.assertEqual(code, 0)
        self.assertIn("PATCH-BUMP 1.8.0 -> 1.8.2", out)
        self.assertNotIn("1.9.0", out)
        self.assertIn("do not merge", out)
        self.assertEqual(self.fx.snapshot(), before)

    def test_dry_run_makes_no_remote_or_registry_call(self) -> None:
        def forbidden(*_a: object) -> object:
            raise AssertionError("dry-run must not reach origin or a registry")

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = MODULE.main(
                ["--dry-run"], root=self.fx.work, registry_published=forbidden, remote_tag_lister=forbidden
            )
        self.assertEqual(code, 0)

    # --- refusals ----------------------------------------------------------
    def test_explicit_version_is_rejected(self) -> None:
        before = self.fx.snapshot()
        with contextlib.redirect_stderr(io.StringIO()) as err:
            code, _ = self.fx.run("1.8.0")
        self.assertEqual(str(code), "2")
        self.assertIn("unrecognized arguments", err.getvalue())
        self.assertEqual(self.fx.snapshot(), before)

    def test_protected_branch_is_refused(self) -> None:
        git(self.fx.work, "checkout", "-q", "develop")
        for flags in (["--dry-run"], []):
            code, _ = self.fx.run(*flags)
            self.assertIn("PROTECTED_BRANCH", str(code))

    def test_detached_head_is_refused(self) -> None:
        git(self.fx.work, "checkout", "-q", "--detach")
        code, _ = self.fx.run("--dry-run")
        self.assertIn("DETACHED_HEAD", str(code))

    def test_dirty_tracked_tree_is_refused(self) -> None:
        self.fx.write("Cargo.toml", CARGO.format(v="1.8.0") + "# edit\n")
        code, _ = self.fx.run("--dry-run")
        self.assertIn("DIRTY_TREE", str(code))

    def test_missing_prerelease_table_is_refused(self) -> None:
        self.fx.write("release/publish-artifacts.toml", "[project]\nname = 'x'\n")
        git(self.fx.work, "commit", "-q", "-am", "no table")
        code, _ = self.fx.run("--dry-run")
        self.assertIn("MANIFEST_INVALID", str(code))

    def test_invalid_workspace_version_is_refused(self) -> None:
        self.fx.write("Cargo.toml", CARGO.format(v="1.8"))
        git(self.fx.work, "commit", "-q", "-am", "bad")
        code, _ = self.fx.run("--dry-run")
        self.assertIn("VERSION_INVALID", str(code))

    def test_exhausted_patch_range_fails_closed(self) -> None:
        tags = {f"prerelease/v1.8.{n}" for n in range(MODULE.MAX_PATCH_STEPS + 1)}
        with self.assertRaises(SystemExit) as caught:
            MODULE.select_version("1.8.0", "prerelease/v", tags, None)
        self.assertIn("NO_FREE_VERSION", str(caught.exception))

    def test_selection_only_ever_changes_the_patch(self) -> None:
        tags = {f"prerelease/v1.8.{n}" for n in range(MODULE.MAX_PATCH_STEPS - 1)}
        chosen = MODULE.select_version("1.8.0", "prerelease/v", tags, None)
        self.assertEqual(chosen, f"1.8.{MODULE.MAX_PATCH_STEPS - 1}")

    # --- identity independence ---------------------------------------------
    def test_real_run_uses_only_the_fixture_local_identity(self) -> None:
        for name in IDENTITY_VARIABLES:
            self.assertNotIn(name, os.environ)
        probe = subprocess.run(
            ["git", "-C", str(Path(self.tmp.name) / "no-repo"), "var", "GIT_COMMITTER_IDENT"],
            capture_output=True, text=True, check=False,
        )
        self.assertNotEqual(probe.returncode, 0, "the hostile environment must offer no ambient identity")
        code, out = self.fx.run(published={"1.8.0"})
        self.assertEqual(code, 0)
        self.assertIn("prerelease/v1.8.1", out)
        who = git(self.fx.work, "log", "-1", "--format=%an <%ae>|%cn <%ce>").strip()
        expected = f"{IDENTITY_NAME} <{IDENTITY_EMAIL}>"
        self.assertEqual(who, f"{expected}|{expected}")
        tagger = git(self.fx.work, "for-each-ref", "--format=%(taggername) <%(taggeremail)>", "refs/tags/prerelease/v1.8.1").strip()
        self.assertEqual(tagger, f"{IDENTITY_NAME} <<{IDENTITY_EMAIL}>>")

    # --- real run (local bare origin) -------------------------------------
    def test_real_run_tags_unused_workspace_version_without_a_commit(self) -> None:
        head = git(self.fx.work, "rev-parse", "HEAD").strip()
        code, out = self.fx.run()
        self.assertEqual(code, 0)
        self.assertEqual(out.strip(), f"created and pushed prerelease/v1.8.0 from {head}")
        self.assertEqual(git(self.fx.work, "rev-parse", "HEAD").strip(), head)
        self.assertIn("refs/tags/prerelease/v1.8.0", git(self.fx.origin, "for-each-ref"))
        self.assertIn("refs/heads/feature/prep", git(self.fx.origin, "for-each-ref"))

    def test_real_run_never_overwrites_an_existing_tag(self) -> None:
        self.assertEqual(self.fx.run()[0], 0)
        old = git(self.fx.origin, "rev-parse", "refs/tags/prerelease/v1.8.0").strip()
        git(self.fx.work, "tag", "-d", "prerelease/v1.8.0")  # remote still holds it
        code, out = self.fx.run()
        self.assertEqual(code, 0)
        self.assertIn("created and pushed prerelease/v1.8.1 from ", out)
        self.assertEqual(git(self.fx.origin, "rev-parse", "refs/tags/prerelease/v1.8.0").strip(), old)

    def test_real_run_bumps_every_lockstep_file_by_patch_only(self) -> None:
        code, out = self.fx.run(published={"1.8.0"})
        self.assertEqual(code, 0)
        self.assertIn("created and pushed prerelease/v1.8.1 from ", out)
        cargo = (self.fx.work / "Cargo.toml").read_text(encoding="utf-8")
        self.assertEqual(cargo.count('"1.8.1"'), 3)
        self.assertNotIn("1.8.0", cargo)
        self.assertIn('serde = { version = "1" }', cargo)
        for path in PYPROJECTS:
            self.assertIn('version = "1.8.1"', (self.fx.work / path).read_text(encoding="utf-8"))
        lock = (self.fx.work / "Cargo.lock").read_text(encoding="utf-8")
        self.assertEqual(lock.count('version = "1.8.1"'), 2)
        self.assertIn('name = "serde"\nversion = "1.0.0"', lock)
        self.assertEqual(git(self.fx.work, "status", "--porcelain").strip(), "")
        files = git(self.fx.work, "show", "--name-only", "--format=", "HEAD").split()
        self.assertEqual(
            sorted(files),
            sorted(["Cargo.toml", "Cargo.lock", *PYPROJECTS]),
        )

    def test_registry_collision_skips_to_next_patch(self) -> None:
        code, out = self.fx.run(published={"1.8.0", "1.8.1"})
        self.assertEqual(code, 0)
        self.assertIn("prerelease/v1.8.2", out)

    def test_lookup_failure_aborts_before_any_mutation(self) -> None:
        def failing(_version: str) -> bool:
            raise SystemExit("registry lookup failed")

        before = self.fx.snapshot()
        with contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(SystemExit):
                MODULE.main([], root=self.fx.work, registry_published=failing)
        self.assertEqual(self.fx.snapshot(), before)


if __name__ == "__main__":
    unittest.main()
