#!/usr/bin/env python3
"""Install the pinned UniFFI Go generator with bounded retries."""

from __future__ import annotations

import subprocess
import time


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
ATTEMPTS = 3
TIMEOUT_SECONDS = 900


def main() -> int:
    for attempt in range(1, ATTEMPTS + 1):
        try:
            subprocess.run(COMMAND, check=True, timeout=TIMEOUT_SECONDS)
            return 0
        except subprocess.TimeoutExpired:
            print(
                f"generator install timed out after {TIMEOUT_SECONDS}s "
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
