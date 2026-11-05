"""Wrapper tests for scripts/generate_cli_reference.py (real docli, fixture model)."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "generate_cli_reference.py"
MODEL = ROOT / "tests" / "fixtures" / "cli-reference" / "reference.json"


def run(out: Path, mode: str, model: Path = MODEL, docli: str = "docli"):
    return subprocess.run(
        [sys.executable, str(SCRIPT), "--output-dir", str(out), mode, "--model-json", str(model), "--docli", docli],
        capture_output=True,
        text=True,
        check=False,
    )


class GenerateCliReferenceTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.out = Path(self._tmp.name) / "out"

    def test_write_produces_html_and_markdown_then_check_passes(self):
        done = run(self.out, "--write")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertTrue((self.out / "html" / "index.html").is_file())
        self.assertIn("sc-compose", (self.out / "cli-reference.md").read_text(encoding="utf-8"))
        self.assertEqual(run(self.out, "--check").returncode, 0)

    def test_check_refuses_stale_markdown_and_write_recovers(self):
        self.assertEqual(run(self.out, "--write").returncode, 0)
        md = self.out / "cli-reference.md"
        md.write_text(md.read_text(encoding="utf-8") + "stale\n", encoding="utf-8")
        stale = run(self.out, "--check")
        self.assertEqual(stale.returncode, 1)
        self.assertIn("cli-reference.md", stale.stderr)
        self.assertEqual(run(self.out, "--write").returncode, 0)
        self.assertEqual(run(self.out, "--check").returncode, 0)

    def test_check_refuses_stale_html_missing_and_extra_files(self):
        self.assertEqual(run(self.out, "--write").returncode, 0)
        index = self.out / "html" / "index.html"
        original = index.read_text(encoding="utf-8")
        index.write_text(original + "<!-- stale -->", encoding="utf-8")
        self.assertEqual(run(self.out, "--check").returncode, 1)
        index.write_text(original, encoding="utf-8")
        (self.out / "html" / "extra.txt").write_text("x", encoding="utf-8")
        self.assertEqual(run(self.out, "--check").returncode, 1)
        (self.out / "html" / "extra.txt").unlink()
        index.unlink()
        self.assertEqual(run(self.out, "--check").returncode, 1)

    def test_check_never_rewrites_expected_files(self):
        self.assertEqual(run(self.out, "--write").returncode, 0)
        md = self.out / "cli-reference.md"
        md.write_text("tampered\n", encoding="utf-8")
        self.assertEqual(run(self.out, "--check").returncode, 1)
        self.assertEqual(md.read_text(encoding="utf-8"), "tampered\n")

    def test_missing_outputs_fail_check(self):
        self.assertEqual(run(self.out, "--check").returncode, 1)

    def test_model_without_payload_model_is_rejected(self):
        bad = Path(self._tmp.name) / "bad.json"
        bad.write_text('{"payload": {}}', encoding="utf-8")
        done = run(self.out, "--write", model=bad)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("payload.model", done.stderr)

    def test_missing_docli_names_pinned_install(self):
        done = run(self.out, "--write", docli="docli-does-not-exist")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("sc-docli --version 0.2.0", done.stderr)


if __name__ == "__main__":
    unittest.main()
