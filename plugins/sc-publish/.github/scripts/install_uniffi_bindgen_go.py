#!/usr/bin/env python3
"""Install the pinned UniFFI Go generator with bounded retries."""

from __future__ import annotations

import subprocess
import shutil
import time
import os
import platform
from pathlib import Path


COMMAND = [
    "cargo",
    "install",
    "uniffi-bindgen-go",
    "--git",
    "https://github.com/NordSecurity/uniffi-bindgen-go",
    "--tag",
    "v0.7.1+v0.31.0",
    "--locked",
]
ATTEMPTS = 2
TIMEOUT_SECONDS = 1800
DARWIN_TIMEOUT_SECONDS = 2700
TARGET_DIR = Path.home() / ".cargo" / "target" / "uniffi-bindgen-go"


def timeout_seconds() -> int:
    """Use the longer budget only for the slow macOS generator build."""
    return DARWIN_TIMEOUT_SECONDS if platform.system().lower() == "darwin" else TIMEOUT_SECONDS


def main() -> int:
    if shutil.which("uniffi-bindgen-go"):
        print("using cached uniffi-bindgen-go")
        return 0
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(TARGET_DIR))
    timeout = timeout_seconds()
    for attempt in range(1, ATTEMPTS + 1):
        try:
            subprocess.run(COMMAND, check=True, timeout=timeout, env=environment)
            return 0
        except subprocess.TimeoutExpired:
            print(
                f"generator install timed out after {timeout}s "
                f"(attempt {attempt}/{ATTEMPTS})"
            )
        except subprocess.CalledProcessError as error:
            print(
                f"generator install failed with {error.returncode} "
                f"(attempt {attempt}/{ATTEMPTS})"
            )
        if attempt < ATTEMPTS:
            time.sleep(10)
    raise SystemExit(
        f"unable to install pinned uniffi-bindgen-go after {ATTEMPTS} "
        "bounded attempts"
    )


if __name__ == "__main__":
    raise SystemExit(main())
