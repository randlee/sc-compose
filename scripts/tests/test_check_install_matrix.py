"""Deterministic tests for scripts/check_install_matrix.py.

Every case is built from controlled inputs in a temporary directory: fixture
contracts, staged files, the run's checksums.txt, a workflow run object and an
artifact listing shaped as GitHub returns them, and an injected clock. No
network, live API, randomness, sleeps or timing assertions.
"""
from __future__ import annotations

import copy
import hashlib
import importlib.util
import io
import json
from contextlib import redirect_stdout
from datetime import datetime, timezone
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "check_install_matrix.py"
SPEC = importlib.util.spec_from_file_location("check_install_matrix", SCRIPT)
cim = importlib.util.module_from_spec(SPEC)
sys.modules["check_install_matrix"] = cim
SPEC.loader.exec_module(cim)

HEAD = "a" * 40
OTHER_HEAD = "b" * 40
VERSION = "1.8.0"
REPO = "randlee/sc-compose"
RUN_ID = 4242
NOW = datetime(2026, 11, 6, 12, 0, tzinfo=timezone.utc)

RUST_TARGETS = [  # six coordinates plus the supplemental Windows gnu toolchain
    ("x86_64-unknown-linux-gnu", "tar.gz"),
    ("aarch64-unknown-linux-gnu", "tar.gz"),
    ("x86_64-apple-darwin", "tar.gz"),
    ("aarch64-apple-darwin", "tar.gz"),
    ("x86_64-pc-windows-msvc", "zip"),
    ("x86_64-pc-windows-gnu", "zip"),
    ("aarch64-pc-windows-msvc", "zip"),
]
GO_TARGETS = [
    ("x86_64-unknown-linux-gnu", "linux", "amd64"),
    ("aarch64-unknown-linux-gnu", "linux", "arm64"),
    ("x86_64-apple-darwin", "darwin", "amd64"),
    ("aarch64-apple-darwin", "darwin", "arm64"),
    ("x86_64-pc-windows-gnu", "windows", "amd64"),
    ("aarch64-pc-windows-gnullvm", "windows", "arm64"),
]
WHEEL_PLATFORMS = {
    ("linux", "x64"): ("cp39", "manylinux_2_28_x86_64"),
    ("linux", "arm64"): ("cp39", "manylinux_2_28_aarch64"),
    ("macos", "x64"): ("cp39", "macosx_10_12_x86_64"),
    ("macos", "arm64"): ("cp39", "macosx_11_0_arm64"),
    ("windows", "x64"): ("cp39", "win_amd64"),
    ("windows", "arm64"): ("cp311", "win_arm64"),  # FR-25.2 Windows ARM64 exception
}
DISTRIBUTIONS = ("sc-compose", "sc-sha", "sc-composer-beads")


def write_contracts(root: Path, rust_targets=RUST_TARGETS, go_targets=GO_TARGETS) -> tuple[Path, Path]:
    release = root / "release"
    release.mkdir(parents=True, exist_ok=True)
    lines = ['schema_version = 1', '', '[project]', 'name = "sc-compose"', 'archive_prefix = "sc-compose"', '']
    for target, archive in rust_targets:
        os_label = "ubuntu-latest" if "linux" in target else "macos-latest" if "darwin" in target else "windows-latest"
        lines += ['[[release_targets]]', f'target = "{target}"', f'os = "{os_label}"', f'archive = "{archive}"']
    for dist in DISTRIBUTIONS:
        lines += ['', '[[python_distributions]]', f'name = "{dist}"', f'source = "bindings/{dist}"', 'sdist = true',
                  'wheels = ["ubuntu-latest", "macos-latest", "windows-latest"]']
    manifest = release / "publish-artifacts.toml"
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8")
    go_config = release / "go-native-module.toml"
    go_config.write_text('schema_version = 1\nsource = "bindings/sc-sha-go"\ncargo_package = "sc-sha-go"\n'
                         'artifact_prefix = "sc-sha-go"\n', encoding="utf-8")
    native = root / "bindings" / "sc-sha-go" / "native"
    native.mkdir(parents=True, exist_ok=True)
    targets = ['schema_version = 1', '', '[contract]', 'linkage = "static"', 'generated_header = "go/sc_sha_go/sc_sha_go.h"',
               'generated_package = "go/sc_sha_go"', 'native_library = "sc_sha_go"']
    for target, goos, goarch in go_targets:
        targets += ['', '[[targets]]', f'rust_target = "{target}"', f'goos = "{goos}"', f'goarch = "{goarch}"',
                    'library = "libsc_sha_go.a"']
    (native / "targets.toml").write_text("\n".join(targets) + "\n", encoding="utf-8")
    return manifest, go_config


def go_zip(path: Path, target: str, version: str) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(f"sc-sha-go-{target}/VERSION", f"{version}\n")
        archive.writestr(f"sc-sha-go-{target}/go.mod", "module example.com/sc-sha-go\n")
        archive.writestr(f"sc-sha-go-{target}/native/{target}/libsc_sha_go.a", target)


def stage_files(staging: Path, version: str = VERSION, wheel_version: str = VERSION) -> None:
    staging.mkdir(parents=True, exist_ok=True)
    for target, archive in RUST_TARGETS:
        (staging / f"sc-compose_{version}_{target}.{archive}").write_bytes(f"cli {target}".encode())
    for dist in DISTRIBUTIONS:
        stem = dist.replace("-", "_")
        for python_tag, platform_tag in WHEEL_PLATFORMS.values():
            (staging / f"{stem}-{wheel_version}-{python_tag}-abi3-{platform_tag}.whl").write_bytes(f"{dist} {platform_tag}".encode())
        (staging / f"{stem}-{wheel_version}.tar.gz").write_bytes(f"sdist {dist}".encode())
    for target, _, _ in GO_TARGETS:
        go_zip(staging / f"sc-sha-go-{target}.zip", target, version)


def checksums_for(staging: Path) -> str:
    return "".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in sorted(staging.iterdir()))


def run_object(**overrides) -> dict:
    run = {"id": RUN_ID, "head_sha": HEAD, "status": "completed", "conclusion": "success", "event": "workflow_dispatch",
           "path": ".github/workflows/release-preflight.yml", "updated_at": "2026-11-06T10:00:00Z",
           "repository": {"full_name": REPO}}
    run.update(overrides)
    return run


def artifact(name: str, number: int = 1, **overrides) -> dict:
    entry = {"id": number, "name": name, "expired": False, "expires_at": "2026-12-06T10:00:00Z",
             "workflow_run": {"id": RUN_ID, "head_sha": HEAD}}
    entry.update(overrides)
    return entry


def listing_of(*entries: dict) -> dict:
    return {"total_count": len(entries), "artifacts": list(entries)}


def artifact_listing(**overrides) -> dict:
    """The run's two required artifacts; overrides apply to both."""
    return listing_of(artifact(cim.CHECKSUMS_ARTIFACT, 1, **overrides), artifact(cim.FILES_ARTIFACT, 2, **overrides))


class Fixture:
    """One complete, valid candidate; tests change one input and re-verify."""

    def __init__(self, root: Path, rust_targets=RUST_TARGETS, go_targets=GO_TARGETS):
        self.root = root
        self.manifest, self.go_config = write_contracts(root, rust_targets, go_targets)
        self.staging = root / "staging"
        stage_files(self.staging)
        self.checksums = checksums_for(self.staging)
        self.run = run_object()
        self.artifacts = artifact_listing()

    def verify(self, **overrides) -> dict:
        arguments = dict(contracts=cim.load_contracts(self.manifest, self.go_config), run=self.run, artifacts=self.artifacts,
                         checksums_text=self.checksums, staging=self.staging, repository=REPO,
                         workflow=cim.DEFAULT_WORKFLOW, head=HEAD, version=VERSION, now=NOW)
        arguments.update(overrides)
        return cim.verify(**arguments)


def codes(result: dict) -> set[str]:
    return {error["code"] for error in result["errors"]}


class CompleteCandidateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.fixture = Fixture(Path(self.directory.name))

    def tearDown(self) -> None:
        self.directory.cleanup()

    def test_complete_candidate_verifies_all_thirty_cells(self) -> None:
        result = self.fixture.verify()
        self.assertEqual(result["errors"], [])
        self.assertTrue(result["ok"])
        self.assertEqual((result["required_cells"], result["verified_cells"]), (30, 30))
        self.assertEqual(len(result["cells"]), 30)
        families = {cell["family"] for cell in result["cells"]}
        self.assertEqual(families, set(cim.FAMILIES))

    def test_duplicate_windows_toolchain_counts_one_coordinate(self) -> None:
        result = self.fixture.verify()
        windows_x64 = [c for c in result["cells"] if (c["family"], c["os"], c["arch"]) == ("rust-cli", "windows", "x64")]
        self.assertEqual(len(windows_x64), 1)
        self.assertEqual(len(windows_x64[0]["files"]), 2)  # msvc and gnu, one coordinate
        self.assertEqual(sum(1 for c in result["cells"] if c["family"] == "rust-cli"), 6)

    def test_command_line_passes_and_reports_json(self) -> None:
        paths = {name: self.fixture.root / f"{name}.json" for name in ("run", "artifacts")}
        paths["run"].write_text(json.dumps(self.fixture.run), encoding="utf-8")
        paths["artifacts"].write_text(json.dumps(self.fixture.artifacts), encoding="utf-8")
        sums = self.fixture.root / "checksums.txt"
        sums.write_text(self.fixture.checksums, encoding="utf-8")
        argv = ["--manifest", str(self.fixture.manifest), "--go-config", str(self.fixture.go_config), "--run", str(paths["run"]),
                "--artifacts", str(paths["artifacts"]), "--checksums", str(sums), "--staging", str(self.fixture.staging),
                "--repository", REPO, "--candidate-sha", HEAD, "--version", VERSION, "--now", "2026-11-06T12:00:00Z"]
        result = subprocess.run([sys.executable, str(SCRIPT), *argv], capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["verified_cells"], 30)
        stale = subprocess.run([sys.executable, str(SCRIPT), *argv[:-6], "--candidate-sha", OTHER_HEAD, "--version", VERSION,
                                "--now", "2026-11-06T12:00:00Z"], capture_output=True, text=True, check=False)
        self.assertEqual(stale.returncode, 1)
        self.assertFalse(json.loads(stale.stdout)["ok"])

    def test_unreadable_input_is_exit_two_not_a_pass(self) -> None:
        out = io.StringIO()
        with redirect_stdout(out):
            code = cim.main(["--manifest", str(self.fixture.manifest), "--go-config", str(self.fixture.go_config),
                             "--run", str(self.fixture.root / "missing.json"), "--artifacts", str(self.fixture.root / "missing.json"),
                             "--checksums", str(self.fixture.root / "missing.txt"), "--staging", str(self.fixture.staging),
                             "--repository", REPO, "--candidate-sha", HEAD, "--version", VERSION, "--now", "2026-11-06T12:00:00Z"])
        self.assertEqual(code, 2)
        self.assertFalse(json.loads(out.getvalue())["ok"])

    def test_repository_contracts_parse(self) -> None:
        contracts = cim.load_contracts(ROOT / "release" / "publish-artifacts.toml", ROOT / "release" / "go-native-module.toml")
        self.assertEqual(contracts.archive_prefix, "sc-compose")
        self.assertEqual(set(contracts.wheel_names), set(DISTRIBUTIONS))
        self.assertTrue(contracts.rust_targets and contracts.go_targets)


class RunEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.fixture = Fixture(Path(self.directory.name))

    def tearDown(self) -> None:
        self.directory.cleanup()

    def assertRejected(self, code: str, **overrides) -> None:
        result = self.fixture.verify(**overrides)
        self.assertFalse(result["ok"])
        self.assertIn(code, codes(result), result["errors"])

    def test_missing_run(self) -> None:
        self.assertRejected("RUN_MISSING", run={})
        self.assertRejected("RUN_MISSING", run=None)

    def test_wrong_run(self) -> None:
        cases = {
            "repository": run_object(repository={"full_name": "someone/fork"}),
            "workflow": run_object(path=".github/workflows/ci.yml"),
            "event": run_object(event="push"),
            "failed": run_object(conclusion="failure"),
            "running": run_object(status="in_progress", conclusion=None),
            "future": run_object(updated_at="2026-11-07T00:00:00Z"),
            "no time": run_object(updated_at=None),
        }
        for label, run in cases.items():
            with self.subTest(label):
                self.assertRejected("RUN_WRONG", run=run)

    def test_workflow_ref_suffix_is_the_same_workflow(self) -> None:
        result = self.fixture.verify(run=run_object(path=".github/workflows/release-preflight.yml@refs/heads/develop"))
        self.assertTrue(result["ok"], result["errors"])

    def test_stale_run_for_another_head(self) -> None:
        self.assertRejected("RUN_STALE", run=run_object(head_sha=OTHER_HEAD))
        self.assertRejected("RUN_STALE", head=OTHER_HEAD)

    def test_expired_evidence(self) -> None:
        cases = {
            "flag": artifact_listing(expired=True),
            "past": artifact_listing(expires_at="2026-11-06T11:59:59Z"),
            "at now": artifact_listing(expires_at="2026-11-06T12:00:00Z"),
            "no expiry": artifact_listing(expires_at=None),
            "none retained": {"total_count": 0, "artifacts": []},
            "no listing": None,
        }
        for label, listing in cases.items():
            with self.subTest(label):
                self.assertRejected("EVIDENCE_EXPIRED", artifacts=listing)

    def test_artifact_from_another_run(self) -> None:
        self.assertRejected("ARTIFACT_WRONG_RUN", artifacts=artifact_listing(workflow_run={"id": RUN_ID + 1, "head_sha": HEAD}))
        self.assertRejected("ARTIFACT_WRONG_RUN", artifacts=artifact_listing(workflow_run={"id": RUN_ID, "head_sha": OTHER_HEAD}))

    def test_injected_clock_decides_expiry(self) -> None:
        later = datetime(2026, 12, 7, tzinfo=timezone.utc)
        self.assertTrue(self.fixture.verify()["ok"])
        self.assertRejected("EVIDENCE_EXPIRED", now=later)

    def test_invalid_candidate(self) -> None:
        self.assertRejected("CANDIDATE_INVALID", head="abc123")
        self.assertRejected("CANDIDATE_INVALID", version="v1.8")


class RequiredArtifactTests(unittest.TestCase):
    """qa-f1: the listing must name the checksums and release-files artifacts; unrelated evidence never stands in."""

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.fixture = Fixture(Path(self.directory.name))

    def tearDown(self) -> None:
        self.directory.cleanup()

    def assertRejected(self, code: str, listing: dict) -> dict:
        result = self.fixture.verify(artifacts=listing)
        self.assertFalse(result["ok"])
        self.assertIn(code, codes(result), result["errors"])
        return result

    def test_unrelated_live_artifact_does_not_satisfy_required_evidence(self) -> None:
        result = self.assertRejected("EVIDENCE_MISSING", listing_of(artifact("coverage-report")))
        missing = [e["message"] for e in result["errors"] if e["code"] == "EVIDENCE_MISSING"]
        self.assertEqual(len(missing), 2)

    def test_each_required_artifact_missing(self) -> None:
        for required, other in ((cim.CHECKSUMS_ARTIFACT, cim.FILES_ARTIFACT), (cim.FILES_ARTIFACT, cim.CHECKSUMS_ARTIFACT)):
            with self.subTest(missing=required):
                result = self.assertRejected("EVIDENCE_MISSING", listing_of(artifact(other), artifact("coverage-report", 3)))
                self.assertEqual([e["message"] for e in result["errors"]], [f"run {RUN_ID} retains no {required!r} artifact"])

    def test_each_required_artifact_expired(self) -> None:
        for required, other in ((cim.CHECKSUMS_ARTIFACT, cim.FILES_ARTIFACT), (cim.FILES_ARTIFACT, cim.CHECKSUMS_ARTIFACT)):
            for label, stale in (("flag", {"expired": True}), ("past", {"expires_at": "2026-11-06T11:00:00Z"})):
                with self.subTest(expired=required, how=label):
                    listing = listing_of(artifact(required, 1, **stale), artifact(other, 2), artifact("coverage-report", 3))
                    self.assertRejected("EVIDENCE_EXPIRED", listing)

    def test_required_artifact_listed_twice_is_ambiguous(self) -> None:
        listing = listing_of(artifact(cim.CHECKSUMS_ARTIFACT, 1), artifact(cim.CHECKSUMS_ARTIFACT, 2), artifact(cim.FILES_ARTIFACT, 3))
        self.assertRejected("EVIDENCE_AMBIGUOUS", listing)

    def test_required_artifact_from_another_run(self) -> None:
        foreign = artifact(cim.FILES_ARTIFACT, 2, workflow_run={"id": RUN_ID + 1, "head_sha": HEAD})
        self.assertRejected("ARTIFACT_WRONG_RUN", listing_of(artifact(cim.CHECKSUMS_ARTIFACT), foreign))

    def test_required_artifacts_plus_unrelated_live_artifact_verify(self) -> None:
        listing = listing_of(artifact(cim.CHECKSUMS_ARTIFACT, 1), artifact(cim.FILES_ARTIFACT, 2), artifact("coverage-report", 3))
        result = self.fixture.verify(artifacts=listing)
        self.assertTrue(result["ok"], result["errors"])
        self.assertEqual(result["verified_cells"], 30)

    def test_artifact_names_are_explicit_inputs(self) -> None:
        listing = listing_of(artifact("matrix-sums", 1), artifact("matrix-files", 2))
        self.assertIn("EVIDENCE_MISSING", codes(self.fixture.verify(artifacts=listing)))
        result = self.fixture.verify(artifacts=listing, checksums_artifact="matrix-sums", files_artifact="matrix-files")
        self.assertTrue(result["ok"], result["errors"])


class FileEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.fixture = Fixture(Path(self.directory.name))
        self.staging = self.fixture.staging

    def tearDown(self) -> None:
        self.directory.cleanup()

    def assertRejected(self, code: str, **overrides) -> dict:
        result = self.fixture.verify(**overrides)
        self.assertFalse(result["ok"])
        self.assertIn(code, codes(result), result["errors"])
        return result

    def test_changed_bytes_are_a_digest_mismatch(self) -> None:
        (self.staging / "sc-compose_1.8.0_x86_64-unknown-linux-gnu.tar.gz").write_bytes(b"rebuilt after the gate")
        result = self.assertRejected("DIGEST_MISMATCH")
        self.assertIn("CELL_MISSING", codes(result))  # a mismatched file covers no cell

    def test_staged_file_missing(self) -> None:
        (self.staging / "sc_sha-1.8.0-cp311-abi3-win_arm64.whl").unlink()
        result = self.assertRejected("FILE_MISSING")
        self.assertIn("CELL_MISSING", codes(result))
        self.assertEqual(result["verified_cells"], 29)

    def test_file_added_after_the_gate(self) -> None:
        (self.staging / "sc-compose_1.8.0_riscv64gc-unknown-linux-gnu.tar.gz").write_bytes(b"extra")
        self.assertRejected("FILE_UNEXPECTED")

    def test_checksums_must_list_every_required_cell(self) -> None:
        kept = [line for line in self.fixture.checksums.splitlines() if "win_arm64" not in line or "sc_compose-" not in line]
        (self.staging / "sc_compose-1.8.0-cp311-abi3-win_arm64.whl").unlink()
        result = self.assertRejected("CELL_MISSING", checksums_text="\n".join(kept) + "\n")
        self.assertEqual(result["verified_cells"], 29)

    def test_malformed_or_duplicate_checksums(self) -> None:
        self.assertRejected("CHECKSUMS_INVALID", checksums_text="not a checksum line\n")
        self.assertRejected("CHECKSUMS_INVALID", checksums_text="")
        first = self.fixture.checksums.splitlines()[0]
        self.assertRejected("CHECKSUMS_INVALID", checksums_text=self.fixture.checksums + first + "\n")
        self.assertRejected("CHECKSUMS_INVALID", checksums_text=f"{'0' * 64}  ../escape.whl\n")

    def test_recorded_digest_differs_from_staged_bytes(self) -> None:
        name = "sc-sha-go-x86_64-unknown-linux-gnu.zip"
        lines = [f"{'0' * 64}  {name}" if line.endswith(name) else line for line in self.fixture.checksums.splitlines()]
        self.assertRejected("DIGEST_MISMATCH", checksums_text="\n".join(lines) + "\n")

    def test_wrong_candidate_version(self) -> None:
        result = self.assertRejected("VERSION_MISMATCH", version="1.8.1")
        self.assertEqual(result["verified_cells"], 0)

    def test_go_module_with_wrong_version_file(self) -> None:
        name = "sc-sha-go-aarch64-apple-darwin.zip"
        go_zip(self.staging / name, "aarch64-apple-darwin", "1.7.0")
        result = self.assertRejected("VERSION_MISMATCH", checksums_text=checksums_for(self.staging))
        self.assertIn("CELL_MISSING", codes(result))

    def test_wheel_tags(self) -> None:
        bad = self.staging / "sc_sha-1.8.0-cp311-abi3-manylinux_2_28_x86_64.whl"  # the 3.11 exception is Windows ARM64 only
        (self.staging / "sc_sha-1.8.0-cp39-abi3-manylinux_2_28_x86_64.whl").rename(bad)
        self.assertRejected("WHEEL_TAG_INVALID", checksums_text=checksums_for(self.staging))

    def test_non_abi3_wheel_is_rejected(self) -> None:
        old = self.staging / "sc_compose-1.8.0-cp39-abi3-win_amd64.whl"
        old.rename(self.staging / "sc_compose-1.8.0-cp39-cp39-win_amd64.whl")
        self.assertRejected("WHEEL_TAG_INVALID", checksums_text=checksums_for(self.staging))

    def test_unrecognized_file(self) -> None:
        (self.staging / "notes.txt").write_bytes(b"x")
        self.assertRejected("FILE_UNRECOGNIZED", checksums_text=checksums_for(self.staging))

    def test_two_wheels_for_one_cell(self) -> None:
        (self.staging / "sc_sha-1.8.0-cp39-abi3-musllinux_1_2_x86_64.whl").write_bytes(b"second linux x64 wheel")
        self.assertRejected("CELL_DUPLICATE", checksums_text=checksums_for(self.staging))


class CoordinateContractTests(unittest.TestCase):
    def test_supplemental_toolchain_never_stands_in_for_windows_arm64(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            without_arm64 = [t for t in RUST_TARGETS if t[0] != "aarch64-pc-windows-msvc"]
            fixture = Fixture(Path(directory), rust_targets=without_arm64)
            (fixture.staging / "sc-compose_1.8.0_aarch64-pc-windows-msvc.zip").unlink()
            result = fixture.verify(checksums_text=checksums_for(fixture.staging))
            self.assertFalse(result["ok"])
            self.assertIn({"code": "CONTRACT_MISSING_COORDINATE", "message": "rust-cli declares no windows/arm64 target"},
                          result["errors"])
            self.assertEqual(result["verified_cells"], 29)

    def test_missing_go_coordinate_is_a_contract_gap(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(Path(directory), go_targets=[t for t in GO_TARGETS if t[1:] != ("windows", "arm64")])
            (fixture.staging / "sc-sha-go-aarch64-pc-windows-gnullvm.zip").unlink()
            result = fixture.verify(checksums_text=checksums_for(fixture.staging))
            self.assertIn("CONTRACT_MISSING_COORDINATE", codes(result))
            self.assertFalse(result["ok"])

    def test_declared_supplemental_target_must_still_ship(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(Path(directory))
            (fixture.staging / "sc-compose_1.8.0_x86_64-pc-windows-gnu.zip").unlink()
            result = fixture.verify(checksums_text=checksums_for(fixture.staging))
            self.assertIn("FILE_MISSING", codes(result))
            self.assertEqual(result["verified_cells"], 30)  # the coordinate is covered; the declared file is still required

    def test_prerelease_version_maps_to_pep440_for_wheels(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = Fixture(root)
            for path in fixture.staging.iterdir():
                path.unlink()
            stage_files(fixture.staging, version="1.8.0-rc.1", wheel_version="1.8.0rc1")
            result = fixture.verify(version="1.8.0-rc.1", checksums_text=checksums_for(fixture.staging))
            self.assertTrue(result["ok"], result["errors"])
            self.assertEqual(result["verified_cells"], 30)


class DeterminismTests(unittest.TestCase):
    def test_same_inputs_give_the_same_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(Path(directory))
            first = fixture.verify(run=copy.deepcopy(fixture.run))
            second = fixture.verify(run=copy.deepcopy(fixture.run))
            self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
