from __future__ import annotations

import hashlib
import importlib.util
import io
import re
import shlex
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[3] / ".claude" / "skills" / "prerelease" / "scripts" / "prerelease.py"
SPEC = importlib.util.spec_from_file_location("sc_publish_prerelease", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
PRERELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PRERELEASE)


def manifest(root: Path) -> dict[str, object]:
    return {
        "project": {"archive_prefix": "fixture"},
        "release_targets": [
            {"target": "x86_64-unknown-linux-gnu", "archive": "tar.gz"},
        ],
        "release_binaries": [{"name": "fixture"}],
        "prerelease": {
            "tag_prefix": "prerelease/v",
            "tag_script": ".just/prerelease_tag.py",
            "install_root": str(root / "builds"),
            "binaries": ["fixture"],
            "protected_branches": ["trunk", "release"],
            "selector_dir": {"darwin": str(root / "darwin"), "linux": str(root / "selector"), "windows": str(root / "windows")},
            "post_install": "true",
            "verify": "echo 1.5.11",
        },
    }


def write_manifest(root: Path) -> None:
    (root / "release").mkdir()
    (root / ".just").mkdir()
    (root / ".just" / "prerelease_tag.py").write_text(
        'print("would create tag: prerelease/v1.5.11")\n', encoding="utf-8"
    )
    (root / "release" / "publish-artifacts.toml").write_text(
        "[prerelease]\n"
        'tag_prefix = "prerelease/v"\n'
        'tag_script = ".just/prerelease_tag.py"\n'
        'install_root = "~/.fixture-builds"\n'
        'binaries = ["fixture"]\n'
        'protected_branches = ["trunk", "release"]\n'
        'selector_dir = { darwin = "/tmp", linux = "/tmp", windows = "C:\\\\tmp" }\n'
        'post_install = "true"\n'
        'verify = "echo 1.5.11"\n',
        encoding="utf-8",
    )


def fixture_archive() -> bytes:
    contents = io.BytesIO()
    with zipfile.ZipFile(contents, "w") as archive:
        archive.writestr("fixture_1.5.11_x86_64-unknown-linux-gnu/bin/fixture", "fixture")
    return contents.getvalue()


def fixture_tarball() -> bytes:
    contents = io.BytesIO()
    with tarfile.open(fileobj=contents, mode="w:gz") as archive:
        payload = b"fixture"
        member = tarfile.TarInfo("fixture_1.5.11_x86_64-unknown-linux-gnu/bin/fixture")
        member.size = len(payload)
        member.mode = 0o755
        archive.addfile(member, io.BytesIO(payload))
    return contents.getvalue()


class PrereleaseTests(unittest.TestCase):
    def test_publish_dry_run_never_needs_network_or_authorization(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_manifest(root)
            result = subprocess.run([sys.executable, str(SCRIPT), "--manifest", "release/publish-artifacts.toml", "--publish", "--dry-run"], cwd=root, text=True, capture_output=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("would create tag: prerelease/v1.5.11", result.stdout)
        self.assertIn("would wait for prerelease-archive.yml", result.stdout)

    def test_publish_refuses_without_written_authorization(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_manifest(root)
            result = subprocess.run([sys.executable, str(SCRIPT), "--manifest", "release/publish-artifacts.toml", "--publish"], cwd=root, text=True, capture_output=True, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("authorization", result.stderr)

    def test_create_remains_an_alias_for_publish(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_manifest(root)
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--manifest",
                    "release/publish-artifacts.toml",
                    "--create",
                    "--dry-run",
                ],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("would create tag: prerelease/v1.5.11", result.stdout)

    def test_publish_tags_waits_and_verifies_release_assets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            values = manifest(root)
            archive = fixture_archive()
            name = "fixture_1.5.11_x86_64-unknown-linux-gnu.tar.gz"
            digest = hashlib.sha256(archive).hexdigest()
            release = {"url": "https://example.test/release", "assets": [{"name": "checksums.txt"}, {"name": name}]}

            def download(_tag: str, asset: str, destination: Path, **_kwargs: object) -> Path:
                path = destination / asset
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(f"{digest}  {name}\n".encode() if asset == "checksums.txt" else archive)
                return path

            tag_result = subprocess.CompletedProcess(
                [], 0, stdout="created and pushed prerelease/v1.5.11 from fixture\n"
            )
            with (
                mock.patch.object(PRERELEASE, "require_publish_preconditions"),
                mock.patch.object(
                    PRERELEASE,
                    "command",
                    side_effect=[
                        tag_result,
                        subprocess.CompletedProcess([], 0, stdout="fixture-sha\n"),
                    ],
                ),
                mock.patch.object(PRERELEASE, "wait_for_archive") as wait,
                mock.patch.object(PRERELEASE, "release_for_tag", return_value=release),
                mock.patch.object(PRERELEASE, "download_asset", side_effect=download),
            ):
                tag, url = PRERELEASE.publish(values)
        self.assertEqual((tag, url), ("prerelease/v1.5.11", "https://example.test/release"))
        wait.assert_called_once_with("prerelease/v1.5.11", "fixture-sha")

    def test_publish_rejects_an_operator_supplied_version(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_manifest(root)
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--publish", "1.5.11"],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unrecognized arguments: 1.5.11", result.stderr)

    def test_install_stages_repoints_and_reuses_a_local_pair(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            values = manifest(root)
            archive = root / "fixture.tar.gz"
            archive.write_bytes(fixture_tarball())
            config = values["prerelease"]
            assert isinstance(config, dict)
            stage = root / "builds" / "v1.5.11"
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            name = "fixture_1.5.11_x86_64-unknown-linux-gnu.tar.gz"
            release = {"assets": [{"name": "checksums.txt"}, {"name": name}]}

            def fetch(_tag: str, asset: str, destination: Path, **_kwargs: object) -> Path:
                path = destination / asset
                path.write_bytes(f"{digest}  {name}\n".encode() if asset == "checksums.txt" else archive.read_bytes())
                return path

            with (
                mock.patch.object(PRERELEASE, "resolve_release", return_value=("1.5.11", release, None)),
                mock.patch.object(PRERELEASE, "download_asset", side_effect=fetch) as download,
                mock.patch.object(PRERELEASE.platform, "system", return_value="Linux"),
                mock.patch.object(PRERELEASE.platform, "machine", return_value="x86_64"),
            ):
                version, installed = PRERELEASE.install(values, "1.5.11")
                PRERELEASE.install(values, "1.5.11")
                self.assertEqual((version, installed), ("1.5.11", stage))
                self.assertTrue((stage / "bin" / "fixture").is_file())
                self.assertTrue((root / "selector" / "fixture").is_symlink())
                # checksums.txt plus the host archive, once: the second install reuses the stage.
                self.assertEqual([call.args[1] for call in download.call_args_list], ["checksums.txt", name])

    def test_checksum_verification_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "fixture.zip"
            archive.write_bytes(b"wrong archive")
            with self.assertRaisesRegex(SystemExit, "sha256 mismatch"):
                PRERELEASE.verify_checksum(archive, "0" * 64)

    def test_safe_extract_rejects_zip_path_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "unsafe.zip"
            with zipfile.ZipFile(archive, "w") as bundle:
                bundle.writestr("../escaped", "unsafe")
            with self.assertRaisesRegex(SystemExit, "unsafe path"):
                PRERELEASE.safe_extract(archive, root / "extract")
            self.assertFalse((root / "escaped").exists())

    def test_prerelease_workflow_converges_without_clobbering_a_release(self) -> None:
        workflow = (SCRIPT.parents[4] / ".github" / "workflows" / "prerelease-archive.yml").read_text(encoding="utf-8")
        self.assertIn('gh release create "$tag" --prerelease', workflow)
        self.assertIn('gh release view "$tag" --json isDraft,isPrerelease,assets', workflow)
        self.assertIn('shasum -a 256 "${assets_to_publish[@]}" > checksums.txt', workflow)
        self.assertIn('gh release create "$tag" --prerelease --title "$tag" --generate-notes "${assets_to_publish[@]}" checksums.txt', workflow)
        self.assertIn('cmp checksums.txt existing-release/checksums.txt', workflow)
        self.assertIn("concurrent run converged", workflow)
        self.assertLess(
            workflow.index('gh release create "$tag" --prerelease'),
            workflow.index('gh release view "$tag" --json isDraft,isPrerelease,assets'),
        )
        self.assertNotIn("gh release upload \"$tag\" --clobber", workflow)

    def test_prerelease_workflow_uses_manifest_build_contract(self) -> None:
        workflow = (SCRIPT.parents[4] / ".github" / "workflows" / "prerelease-archive.yml").read_text(encoding="utf-8")
        self.assertIn("toolchain: ${{ needs.plan.outputs.rust_toolchain }}", workflow)
        self.assertIn("uses: ./.github/actions/install-linux-native-deps", workflow)
        self.assertIn("ref: ${{ needs.plan.outputs.source_sha }}", workflow)
        self.assertIn('matrix="$(python3 .github/scripts/release_artifacts.py release-target-matrix --manifest "$RELEASE_ARTIFACT_MANIFEST")"', workflow)
        self.assertNotIn('--manifest \\"$RELEASE_ARTIFACT_MANIFEST\\"', workflow)
        self.assertIn('for bundled_path in binary.get("bundled_paths", []):', workflow)
        self.assertIn("path: ${{ env.ARCHIVE }}", workflow)
        self.assertIn("permissions:\n  contents: read", workflow)
        self.assertIn("    permissions:\n      contents: write", workflow)
        self.assertIn(
            "    steps:\n      - uses: actions/checkout@v4\n"
            "      - uses: actions/download-artifact@v4",
            workflow,
        )
        self.assertIn("timeout-minutes: 45", workflow)
        self.assertNotRegex(workflow, r":\s*\{[^\n]*\$\{\{")

    def test_prerelease_packager_matches_release_packager(self) -> None:
        workflows = SCRIPT.parents[4] / ".github" / "workflows"

        def package_script(path: Path, step_name: str) -> str:
            workflow = path.read_text(encoding="utf-8")
            step = workflow.split(f"      - name: {step_name}\n", 1)[1]
            return step.split("          python3 - <<'PY'\n", 1)[1].split(
                "\n          PY", 1
            )[0]

        prerelease = package_script(
            workflows / "prerelease-archive.yml",
            "Package manifest-declared prerelease archive",
        ).replace(
            'version = "${{ needs.plan.outputs.version }}"',
            'version = "${VERSION}"',
        )
        release = package_script(
            workflows / "release.yml",
            "Package manifest-declared release archive",
        ).replace(
            'version = "${{ needs.gate-and-tag.outputs.release_version }}"',
            'version = "${VERSION}"',
        )
        # A downstream consumer's release workflow may already carry this
        # correction. It invokes the helper with the interpreter running the
        # packaging heredoc; the kit's production release template has not
        # adopted that correction.
        release = release.replace(
            "          import shutil\n          import subprocess",
            "          import shutil\n          import sys\n          import subprocess",
        ).replace(
            '                      "python3",\n'
            '                      ".github/scripts/release_artifacts.py",',
            '                      sys.executable,\n'
            '                      ".github/scripts/release_artifacts.py",',
        )
        self.assertEqual(prerelease, release)

    def test_wait_for_archive_ignores_a_stale_run_for_the_reused_tag(self) -> None:
        stale = {"headSha": "old-sha", "status": "completed", "conclusion": "failure"}
        current = {"headSha": "new-sha", "status": "completed", "conclusion": "success"}
        with (
            mock.patch.object(PRERELEASE, "gh_json", side_effect=[[stale], [stale, current]]),
            mock.patch.object(PRERELEASE.time, "sleep"),
        ):
            PRERELEASE.wait_for_archive("prerelease/v1.5.11", "new-sha")

    def test_wait_for_archive_polls_once_per_minute_until_success(self) -> None:
        clock = [0.0]
        sleeps: list[float] = []

        def sleep(seconds: float) -> None:
            sleeps.append(seconds)
            clock[0] += seconds

        success = {"headSha": "new-sha", "status": "completed", "conclusion": "success"}
        with (
            mock.patch.object(PRERELEASE, "gh_json", side_effect=[[], [], [success]]) as gh_json,
            mock.patch.object(PRERELEASE.time, "monotonic", side_effect=lambda: clock[0]),
            mock.patch.object(PRERELEASE.time, "sleep", side_effect=sleep),
            mock.patch("sys.stdout", new_callable=io.StringIO) as stdout,
        ):
            PRERELEASE.wait_for_archive("prerelease/v1.5.11", "new-sha")
        self.assertEqual(gh_json.call_count, 3)
        self.assertEqual(sleeps, [60, 60])
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "waiting for prerelease-archive.yml (0.0 min elapsed)",
                "waiting for prerelease-archive.yml (1.0 min elapsed)",
                "waiting for prerelease-archive.yml (2.0 min elapsed)",
            ],
        )

    def test_wait_for_archive_times_out_after_twenty_minutes(self) -> None:
        clock = [0.0]
        sleeps: list[float] = []

        def sleep(seconds: float) -> None:
            sleeps.append(seconds)
            clock[0] += seconds

        with (
            mock.patch.object(PRERELEASE, "gh_json", return_value=[]),
            mock.patch.object(PRERELEASE.time, "monotonic", side_effect=lambda: clock[0]),
            mock.patch.object(PRERELEASE.time, "sleep", side_effect=sleep),
            mock.patch("sys.stdout", new_callable=io.StringIO),
        ):
            with self.assertRaisesRegex(
                SystemExit,
                "timed out waiting for prerelease-archive.yml for prerelease/v1.5.11 after 20.0 minutes",
            ):
                PRERELEASE.wait_for_archive("prerelease/v1.5.11", "new-sha")
        self.assertEqual(clock, [20 * 60])
        self.assertEqual(sleeps, [60] * 20)

    def test_wait_for_archive_clamps_the_final_sleep_to_the_remaining_time(self) -> None:
        clock = [0.0]
        sleeps: list[float] = []

        def sleep(seconds: float) -> None:
            sleeps.append(seconds)
            clock[0] += seconds

        with (
            mock.patch.object(PRERELEASE, "ARCHIVE_WAIT_SECONDS", 150),
            mock.patch.object(PRERELEASE, "gh_json", return_value=[]),
            mock.patch.object(PRERELEASE.time, "monotonic", side_effect=lambda: clock[0]),
            mock.patch.object(PRERELEASE.time, "sleep", side_effect=sleep),
            mock.patch("sys.stdout", new_callable=io.StringIO),
        ):
            with self.assertRaisesRegex(SystemExit, "after 2.5 minutes"):
                PRERELEASE.wait_for_archive("prerelease/v1.5.11", "new-sha")
        self.assertEqual(sleeps, [60, 60, 30])

    def test_help_documents_install_and_publish(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--help"],
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--publish", result.stdout)
        self.assertIn("install X.Y.Z", result.stdout)


VERSION = "1.5.11"
ARCHIVE = f"fixture_{VERSION}_x86_64-unknown-linux-gnu.tar.gz"
NATIVE_WHEELS = {
    "linux-x86_64": f"fixture_native-{VERSION}-cp311-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl",
    "linux-aarch64": f"fixture_native-{VERSION}-cp311-abi3-manylinux_2_17_aarch64.manylinux2014_aarch64.whl",
    "musl-x86_64": f"fixture_native-{VERSION}-cp311-abi3-musllinux_1_2_x86_64.whl",
    "macos-arm64": f"fixture_native-{VERSION}-cp311-abi3-macosx_11_0_arm64.whl",
    "windows-amd64": f"fixture_native-{VERSION}-cp311-abi3-win_amd64.whl",
}
NATIVE_SDIST = f"fixture_native-{VERSION}.tar.gz"
PURE_WHEEL = f"fixture_pure-{VERSION}-py3-none-any.whl"


def wheel_manifest(root: Path) -> dict[str, object]:
    """A consumer with binaries, one native wheel per platform plus an sdist, and one pure wheel."""
    values = manifest(root)
    values["python_distributions"] = [
        {
            "name": "fixture-native",
            "sdist": True,
            "wheels": ["ubuntu-latest", "ubuntu-24.04-arm", "ubuntu-musl", "macos-latest", "windows-latest"],
        },
        {"name": "fixture-pure", "sdist": False, "wheels": ["ubuntu-latest"]},
    ]
    return values


def wheels_only_manifest(root: Path) -> dict[str, object]:
    """The beads-cache shape: no release binaries, one maturin distribution with wheels and an sdist."""
    values = manifest(root)
    values["release_binaries"] = []
    values["python_distributions"] = [
        {"name": "fixture-bindings", "sdist": True, "wheels": ["macos-14", "ubuntu-latest"]},
    ]
    config = values["prerelease"]
    assert isinstance(config, dict)
    config["binaries"] = []
    return values


WHEELS_ONLY_ASSETS = {
    "linux": f"fixture_bindings-{VERSION}-cp312-cp312-manylinux_2_34_x86_64.whl",
    "macos": f"fixture_bindings-{VERSION}-cp312-cp312-macosx_11_0_arm64.whl",
    "sdist": f"fixture_bindings-{VERSION}.tar.gz",
}


def release_of(*names: str) -> dict[str, object]:
    return {"url": "https://example.test/release", "assets": [{"name": name} for name in ("checksums.txt", *names)]}


def fake_downloads(payloads: dict[str, bytes], *, corrupt: str | None = None, uncovered: str | None = None):
    """Serve checksums.txt for the payloads, optionally corrupting or omitting one asset's digest."""
    digests = "".join(
        f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in sorted(payloads.items()) if name != uncovered
    )
    requested: list[str] = []

    def download(_tag: str, asset: str, destination: Path, **_kwargs: object) -> Path:
        requested.append(asset)
        destination.mkdir(parents=True, exist_ok=True)
        path = destination / asset
        data = digests.encode() if asset == "checksums.txt" else payloads[asset]
        path.write_bytes(b"tampered" if asset == corrupt else data)
        return path

    return download, requested


def published(values: dict[str, object], release: dict[str, object], download) -> tuple[str, str]:
    tag_result = subprocess.CompletedProcess([], 0, stdout=f"created and pushed prerelease/v{VERSION} from fixture\n")
    with (
        mock.patch.object(PRERELEASE, "require_publish_preconditions"),
        mock.patch.object(
            PRERELEASE,
            "command",
            side_effect=[tag_result, subprocess.CompletedProcess([], 0, stdout="fixture-sha\n")],
        ),
        mock.patch.object(PRERELEASE, "wait_for_archive"),
        mock.patch.object(PRERELEASE, "release_for_tag", return_value=release),
        mock.patch.object(PRERELEASE, "download_asset", side_effect=download),
    ):
        return PRERELEASE.publish(values)


class PrereleaseWheelTests(unittest.TestCase):
    def test_asset_set_requires_every_declared_wheel_for_every_platform_and_sdist(self) -> None:
        values = wheel_manifest(Path("/unused"))
        everything = {ARCHIVE, *NATIVE_WHEELS.values(), NATIVE_SDIST, PURE_WHEEL}
        self.assertEqual(PRERELEASE.require_release_assets(release_of(*everything), values, VERSION), everything)

        without_windows = everything - {NATIVE_WHEELS["windows-amd64"]}
        with self.assertRaisesRegex(SystemExit, "4 wheel\\(s\\) for fixture-native 1.5.11; the manifest declares 5"):
            PRERELEASE.require_release_assets(release_of(*without_windows), values, VERSION)
        with self.assertRaisesRegex(SystemExit, "0 sdist\\(s\\) for fixture-native"):
            PRERELEASE.require_release_assets(release_of(*(everything - {NATIVE_SDIST})), values, VERSION)
        # A wheel of another version never satisfies the declared set.
        stale = f"fixture_pure-1.5.10-py3-none-any.whl"
        with self.assertRaisesRegex(SystemExit, "0 wheel\\(s\\) for fixture-pure"):
            PRERELEASE.require_release_assets(release_of(*(everything - {PURE_WHEEL}), stale), values, VERSION)
        with self.assertRaisesRegex(SystemExit, "missing required assets: " + re.escape(ARCHIVE)):
            PRERELEASE.require_release_assets(release_of(*(everything - {ARCHIVE})), values, VERSION)

    def test_explicit_wheel_platforms_must_each_be_present(self) -> None:
        values = wheel_manifest(Path("/unused"))
        values["python_distributions"] = [
            {
                "name": "fixture-native",
                "sdist": False,
                "wheels": [
                    {"id": "linux", "os": "ubuntu-latest", "target": "x86_64-unknown-linux-gnu", "platform": "manylinux_2_28_x86_64"},
                    {"id": "mac", "os": "macos-14", "target": "aarch64-apple-darwin", "platform": "macosx_11_0_arm64"},
                ],
            }
        ]
        linux = f"fixture_native-{VERSION}-cp311-abi3-manylinux_2_28_x86_64.whl"
        mac = f"fixture_native-{VERSION}-cp311-abi3-macosx_11_0_arm64.whl"
        self.assertEqual(PRERELEASE.require_release_assets(release_of(ARCHIVE, linux, mac), values, VERSION), {ARCHIVE, linux, mac})
        # Right count, wrong platform: a second Linux build cannot stand in for the macOS wheel.
        other_linux = f"fixture_native-{VERSION}-cp311-abi3-manylinux_2_17_x86_64.whl"
        with self.assertRaisesRegex(SystemExit, "no unique macosx_11_0_arm64 wheel"):
            PRERELEASE.require_release_assets(release_of(ARCHIVE, linux, other_linux), values, VERSION)

    def test_publish_checksum_verifies_every_wheel_and_sdist(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            values = wheel_manifest(Path(directory))
            names = {ARCHIVE, *NATIVE_WHEELS.values(), NATIVE_SDIST, PURE_WHEEL}
            payloads = {name: name.encode() for name in names}
            download, requested = fake_downloads(payloads)
            tag, url = published(values, release_of(*names), download)
            self.assertEqual((tag, url), (f"prerelease/v{VERSION}", "https://example.test/release"))
            self.assertEqual(set(requested), {"checksums.txt", *names})

            corrupt, _ = fake_downloads(payloads, corrupt=NATIVE_WHEELS["macos-arm64"])
            with self.assertRaisesRegex(SystemExit, "sha256 mismatch for " + re.escape(NATIVE_WHEELS["macos-arm64"])):
                published(values, release_of(*names), corrupt)
            extra = "unexpected-notes.txt"
            uncovered, _ = fake_downloads({**payloads, extra: b"x"}, uncovered=extra)
            with self.assertRaisesRegex(SystemExit, "checksums.txt does not cover Release assets: unexpected-notes.txt"):
                published(values, release_of(*names, extra), uncovered)

    def test_host_wheel_selection_is_deterministic_for_each_platform(self) -> None:
        values = wheel_manifest(Path("/unused"))
        names = [*NATIVE_WHEELS.values(), NATIVE_SDIST, PURE_WHEEL, ARCHIVE]
        hosts = {
            "linux-x86_64": ("Linux", "x86_64", "glibc"),
            "linux-aarch64": ("Linux", "aarch64", "glibc"),
            "musl-x86_64": ("Linux", "x86_64", "musl"),
            "macos-arm64": ("Darwin", "arm64", ""),
            "windows-amd64": ("Windows", "AMD64", ""),
        }
        for key, (system, machine, libc) in hosts.items():
            with self.subTest(host=key):
                for ordering in (names, list(reversed(names)), sorted(names)):
                    selected = PRERELEASE.host_wheel_names(
                        ordering, values, VERSION, system=system, machine=machine, libc=libc,
                        python=(3, 12), implementation="cpython",
                    )
                    self.assertEqual(selected, [NATIVE_WHEELS[key], PURE_WHEEL])
        # Planted negatives: no wheel for the host, and an old interpreter below the abi3 floor.
        with self.assertRaisesRegex(SystemExit, "no Darwin x86_64 wheel for fixture-native"):
            PRERELEASE.host_wheel_names(names, values, VERSION, system="Darwin", machine="x86_64", python=(3, 12), implementation="cpython")
        with self.assertRaisesRegex(SystemExit, "no Linux x86_64 wheel for fixture-native"):
            PRERELEASE.host_wheel_names(names, values, VERSION, system="Linux", machine="x86_64", libc="glibc", python=(3, 10), implementation="cpython")

    def test_host_wheel_selection_prefers_exact_abi_and_rejects_ties(self) -> None:
        values = {"python_distributions": [{"name": "fixture-native", "wheels": ["a", "b", "c"]}]}
        abi3 = f"fixture_native-{VERSION}-cp311-abi3-manylinux_2_17_x86_64.whl"
        exact = f"fixture_native-{VERSION}-cp312-cp312-manylinux_2_17_x86_64.whl"
        other_python = f"fixture_native-{VERSION}-cp313-cp313-manylinux_2_17_x86_64.whl"
        host = {"system": "Linux", "machine": "x86_64", "libc": "glibc", "implementation": "cpython"}
        self.assertEqual(PRERELEASE.host_wheel_names([abi3, exact, other_python], values, VERSION, python=(3, 12), **host), [exact])
        self.assertEqual(PRERELEASE.host_wheel_names([abi3, exact, other_python], values, VERSION, python=(3, 14), **host), [abi3])
        tie = f"fixture_native-{VERSION}-cp311-abi3-manylinux_2_28_x86_64.whl"
        with self.assertRaisesRegex(SystemExit, "ambiguous host wheels for fixture-native: " + re.escape(f"{abi3}, {tie}")):
            PRERELEASE.host_wheel_names([tie, abi3], values, VERSION, python=(3, 12), **host)

    def test_wheels_only_consumer_dry_run_publish_and_install(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_manifest(root)
            manifest_path = root / "release" / "publish-artifacts.toml"
            manifest_path.write_text(
                manifest_path.read_text(encoding="utf-8").replace('binaries = ["fixture"]', "binaries = []")
                + '[[release_targets]]\ntarget = "x86_64-unknown-linux-gnu"\narchive = "tar.gz"\n'
                + '[[python_distributions]]\nname = "fixture-bindings"\nsdist = true\nwheels = ["macos-14", "ubuntu-latest"]\n',
                encoding="utf-8",
            )
            dry_run = subprocess.run(
                [sys.executable, str(SCRIPT), "--publish", "--dry-run"], cwd=root, text=True, capture_output=True, check=False
            )
            self.assertEqual(dry_run.returncode, 0, dry_run.stderr)
            self.assertIn("would verify 0 binary archive(s), 2 wheel(s), 1 sdist(s), and checksums.txt", dry_run.stdout)

            values = wheels_only_manifest(root)
            names = set(WHEELS_ONLY_ASSETS.values())
            payloads = {name: name.encode() for name in names}
            download, requested = fake_downloads(payloads)
            self.assertEqual(published(values, release_of(*names), download)[0], f"prerelease/v{VERSION}")
            self.assertEqual(set(requested), {"checksums.txt", *names})
            with self.assertRaisesRegex(SystemExit, "0 sdist\\(s\\) for fixture-bindings"):
                published(values, release_of(*(names - {WHEELS_ONLY_ASSETS["sdist"]})), download)

            marker = root / "installed.txt"
            config = values["prerelease"]
            assert isinstance(config, dict)
            config["post_install"] = (
                "{python} -c \"import sys; open(sys.argv[1], 'w').write(sys.argv[2])\" "
                + str(marker) + " {wheels}"
            )
            stage = root / "builds" / f"v{VERSION}"
            host = {"system": "Linux", "machine": "x86_64", "libc": "glibc", "python": (3, 12), "implementation": "cpython"}
            select = PRERELEASE.host_wheel_names

            def install_with(fetch):
                with (
                    mock.patch.object(PRERELEASE, "resolve_release", return_value=(VERSION, release_of(*names), None)),
                    mock.patch.object(PRERELEASE, "download_asset", side_effect=fetch),
                    mock.patch.object(PRERELEASE, "host_wheel_names", side_effect=lambda *a, **k: select(*a, **host)),
                    mock.patch.object(PRERELEASE.platform, "system", return_value="Linux"),
                ):
                    return PRERELEASE.install(values, VERSION)

            corrupt, _ = fake_downloads(payloads, corrupt=WHEELS_ONLY_ASSETS["linux"])
            with self.assertRaisesRegex(SystemExit, "sha256 mismatch"):
                install_with(corrupt)
            self.assertFalse(stage.exists(), "a failed download must not leave a partial stage")

            fetch, fetched = fake_downloads(payloads)
            self.assertEqual(install_with(fetch), (VERSION, stage))
            staged = stage / "wheels" / WHEELS_ONLY_ASSETS["linux"]
            self.assertEqual(sorted(path.name for path in stage.rglob("*") if path.is_file()), [WHEELS_ONLY_ASSETS["linux"]])
            self.assertEqual(marker.read_text(encoding="utf-8"), str(staged))
            self.assertEqual(fetched, ["checksums.txt", WHEELS_ONLY_ASSETS["linux"]])
            self.assertFalse((root / "selector").exists(), "no binaries means no selector")

    def test_headless_install_reads_a_public_release_without_gh(self) -> None:
        values = wheels_only_manifest(Path("/unused"))
        project = values["project"]
        assert isinstance(project, dict)
        project["homepage"] = "https://github.com/example/fixture"
        config = values["prerelease"]
        assert isinstance(config, dict)
        public = {
            "html_url": "https://github.com/example/fixture/releases/tag/prerelease/v1.5.11",
            "prerelease": True,
            "draft": False,
            "assets": [{"name": "checksums.txt"}, *({"name": name} for name in WHEELS_ONLY_ASSETS.values())],
        }
        with (
            mock.patch.object(PRERELEASE, "gh_available", return_value=False),
            mock.patch.object(PRERELEASE, "public_json", return_value=public) as public_json,
        ):
            version, release, base = PRERELEASE.resolve_release(values, config, VERSION)
        self.assertEqual(version, VERSION)
        self.assertEqual(base, "https://github.com/example/fixture/releases/download/prerelease/v1.5.11")
        public_json.assert_called_once_with("https://api.github.com/repos/example/fixture/releases/tags/prerelease/v1.5.11")
        self.assertEqual(PRERELEASE.require_release_assets(release, values, VERSION), set(WHEELS_ONLY_ASSETS.values()))

        project["homepage"] = "https://example.test/fixture"
        with mock.patch.object(PRERELEASE, "gh_available", return_value=False):
            with self.assertRaisesRegex(SystemExit, "PRERELEASE.GH_UNAVAILABLE"):
                PRERELEASE.resolve_release(values, config, VERSION)
        project["homepage"] = "https://github.com/example/fixture"
        with (
            mock.patch.object(PRERELEASE, "gh_available", return_value=False),
            mock.patch.object(PRERELEASE, "public_json", return_value={**public, "prerelease": False}),
        ):
            with self.assertRaisesRegex(SystemExit, "is not a published GitHub prerelease"):
                PRERELEASE.resolve_release(values, config, VERSION)

    def test_command_placeholders_expand_once_and_quote_paths(self) -> None:
        config = {"selector_dir": {"darwin": "/sel dir", "linux": "/sel dir", "windows": "C:/sel dir"}}
        stage = Path("/stage/{version}")
        wheel = Path("/stage/wheels/a b.whl")
        with mock.patch.object(PRERELEASE.os, "name", "posix"):
            expanded = PRERELEASE.replace_command(
                "{python} -m pip install {wheels} && ls {stage_dir} {selector_dir} {version} {unknown}",
                VERSION, stage, config, [wheel],
            )
        self.assertEqual(
            expanded,
            f"{shlex.quote(sys.executable)} -m pip install '/stage/wheels/a b.whl' && ls /stage/{{version}} '/sel dir' 1.5.11 {{unknown}}",
        )

    def test_prerelease_requires_python_311_with_typed_remediation(self) -> None:
        PRERELEASE.require_supported_python()
        with mock.patch.object(PRERELEASE.sys, "version_info", (3, 9, 18)):
            with self.assertRaisesRegex(SystemExit, r"PRERELEASE\.PYTHON_VERSION_UNSUPPORTED: Python 3\.11 or newer"):
                PRERELEASE.require_supported_python()

    def test_prerelease_workflow_builds_every_declared_python_asset(self) -> None:
        workflow = (SCRIPT.parents[4] / ".github" / "workflows" / "prerelease-archive.yml").read_text(encoding="utf-8")
        self.assertIn("    if: ${{ needs.plan.outputs.has_release_binaries == 'true' }}\n    needs: plan\n", workflow)
        self.assertIn("matrix: ${{ fromJSON(needs.plan.outputs.python_wheel_matrix) }}", workflow)
        self.assertIn("matrix: ${{ fromJSON(needs.plan.outputs.python_sdist_matrix) }}", workflow)
        self.assertIn("python-wheel-matrix --manifest", workflow)
        self.assertIn("needs: [plan, build, build-python-wheels, build-python-sdists]", workflow)
        self.assertIn("!contains(needs.*.result, 'failure')", workflow)
        self.assertIn("-name '*.whl'", workflow)
        self.assertIn("verify-python-release-assets --manifest \"$RELEASE_ARTIFACT_MANIFEST\" --asset-dir prerelease-assets", workflow)
        self.assertLess(workflow.index("verify-python-release-assets"), workflow.index("shasum -a 256"))
        self.assertNotIn("${archives[@]}", workflow)
        # No repository-specific artifact names or paths: everything comes from the manifest.
        self.assertNotRegex(workflow, r"assets/[a-z][a-z0-9_-]*-wheels")

    def test_prerelease_wheel_build_matches_release_wheel_build(self) -> None:
        workflows = SCRIPT.parents[4] / ".github" / "workflows"

        def wheel_job(path: Path, outputs: str, ref: str, version: str) -> str:
            text = path.read_text(encoding="utf-8")
            job = text.split("\n  build-python-wheels:\n", 1)[1].split("\n  build-python-sdists:\n", 1)[0]
            job = job.split("      - name: Build wheels (maturin)\n", 1)[1].rstrip()
            return (
                job.replace(ref, "REF").replace(version, "VERSION").replace(outputs, "PLAN")
                .replace("actions/upload-artifact@v6", "actions/upload-artifact@v4")
            )

        prerelease = wheel_job(workflows / "prerelease-archive.yml", "needs.plan.outputs", "needs.plan.outputs.source_sha", "needs.plan.outputs.version")
        release = wheel_job(workflows / "release.yml", "needs.release-plan.outputs", "needs.gate-and-tag.outputs.build_ref", "needs.gate-and-tag.outputs.release_version")
        self.assertIn("maturin build", prerelease)
        self.assertEqual(prerelease, release)



if __name__ == "__main__":
    unittest.main()
