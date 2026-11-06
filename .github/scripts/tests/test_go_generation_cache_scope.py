"""Workflow caches must never restore over tracked Go source.

CI job 127724611523 restored a cached bindings/sc-sha-go/go directory holding an
old three-target native_loader.go over the current six-target source, so the
generated-output diff failed. These checks use controlled workflow definitions
and the tracked tree only: no network, no timing.
"""

import posixpath
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
WORKFLOWS = (".github/workflows/ci.yml", ".github/workflows/go-native-preflight.yml")
GO_DIR = "bindings/sc-sha-go/go"
HANDWRITTEN = (f"{GO_DIR}/sc_sha_go/native_loader.go", f"{GO_DIR}/sc_sha_go/conformance_test.go")

# The exact cache step that caused the failure, as a controlled fixture.
STALE_STEP = {"uses": "actions/cache@v6", "with": {"path": GO_DIR, "key": "k"}}


def cache_paths(step):
    if not str(step.get("uses", "")).startswith("actions/cache@"):
        return []
    return [line.strip() for line in str(step["with"]["path"]).splitlines() if line.strip()]


def covers(cache_path, repo_file):
    cache_path = posixpath.normpath(cache_path)
    return repo_file == cache_path or repo_file.startswith(cache_path + "/")


def tracked_go_files():
    return sorted(p.relative_to(ROOT).as_posix() for p in (ROOT / GO_DIR).rglob("*") if p.is_file())


def go_steps(workflow):
    data = yaml.safe_load((ROOT / workflow).read_text())
    for job in data["jobs"].values():
        steps = job.get("steps", [])
        if any("uniffi-bindgen-go --out-dir" in str(s.get("run", "")) for s in steps):
            return steps
    raise AssertionError(f"no Go generation job in {workflow}")


class GenerationCacheScopeTests(unittest.TestCase):
    def test_old_cache_definition_would_overwrite_handwritten_source(self):
        for path in HANDWRITTEN:
            self.assertTrue((ROOT / path).is_file(), path)
            self.assertTrue(any(covers(p, path) for p in cache_paths(STALE_STEP)), path)

    def test_no_cache_path_covers_any_tracked_go_file(self):
        files = tracked_go_files()
        self.assertTrue(set(HANDWRITTEN) <= set(files))
        for workflow in WORKFLOWS:
            for step in go_steps(workflow):
                for cache_path in cache_paths(step):
                    for repo_file in files:
                        self.assertFalse(covers(cache_path, repo_file), f"{workflow}: {cache_path} -> {repo_file}")

    def test_generation_runs_unconditionally_before_the_diff_check(self):
        for workflow in WORKFLOWS:
            steps = go_steps(workflow)
            gen = next(i for i, s in enumerate(steps) if "uniffi-bindgen-go --out-dir" in str(s.get("run", "")))
            diff = next(i for i, s in enumerate(steps) if "git diff --exit-code -- bindings/sc-sha-go/go" in str(s.get("run", "")))
            self.assertNotIn("if", steps[gen], workflow)
            self.assertLess(gen, diff, workflow)

    def test_generator_binary_cache_is_pinned_by_tag_and_platform(self):
        for workflow in WORKFLOWS:
            keys = [
                str(s["with"]["key"]) for s in go_steps(workflow)
                if any("uniffi-bindgen-go" in p for p in cache_paths(s))
            ]
            self.assertEqual(len(keys), 1, workflow)
            self.assertIn("v0.7.1-v0.31.0", keys[0])
            self.assertIn("runner.os", keys[0])
            self.assertIn("runner.arch", keys[0])
            self.assertIn("install_uniffi_bindgen_go.py", keys[0])


if __name__ == "__main__":
    unittest.main()
