#!/usr/bin/env python3
"""Consumer-declared verification of each staged CLI archive."""
from __future__ import annotations

import argparse
from pathlib import Path
from string import Formatter
import subprocess
import tomllib

FIELDS = {"stage_dir", "target", "manifest"}


def configuration(manifest):
    if "archive_verification" not in manifest:
        return None
    argv = manifest["archive_verification"]
    if (not isinstance(argv, list) or not argv or
            any(not isinstance(arg, str) or not arg or "\0" in arg for arg in argv)):
        raise ValueError("archive_verification must be a nonempty argv array")
    for arg in argv:
        for _, field, spec, conversion in Formatter().parse(arg):
            if field is not None and (field not in FIELDS or spec or conversion):
                raise ValueError("archive_verification supports only {stage_dir}, {target} and {manifest}")
    return argv


def verify(manifest_path, stage_dir, target):
    manifest_path = Path(manifest_path).resolve()
    argv = configuration(tomllib.loads(manifest_path.read_text(encoding="utf-8")))
    if argv is None:
        return
    stage_dir = Path(stage_dir).resolve()
    if not stage_dir.is_dir():
        raise ValueError("archive verification requires an existing staged directory")
    values = {"stage_dir": str(stage_dir), "target": target, "manifest": str(manifest_path)}
    subprocess.run([arg.format_map(values) for arg in argv], check=True, timeout=900)


CALL = '''          import sys
          subprocess.run(
              [sys.executable, ".github/scripts/archive_verification.py",
               "--manifest", os.environ["RELEASE_ARTIFACT_MANIFEST"],
               "--stage-dir", str(stage), "--target", target_name],
              check=True,
          )
'''


def extend_workflow(text, manifest):
    if configuration(manifest) is None:
        return text
    marker = '          archive = Path(f"{stage}.{target[\'archive\']}")\n'
    if text.count(marker) != 1:
        raise ValueError("cannot locate the staged archive verification extension point")
    return text.replace(marker, CALL + marker, 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--stage-dir", required=True)
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    try:
        verify(args.manifest, args.stage_dir, args.target)
        return 0
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Archive verification failed: {error}\n")


if __name__ == "__main__":
    raise SystemExit(main())
