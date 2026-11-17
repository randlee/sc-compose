#!/usr/bin/env python3
"""Build the candidate wheels or run their existing smoke tests."""

from pathlib import Path
import subprocess
import sys


def main() -> None:
    if sys.argv[1:] == ["build"]:
        for manifest in (
            "bindings/python/Cargo.toml",
            "bindings/sc-composer-beads-python/Cargo.toml",
            "bindings/sc-sha-python/Cargo.toml",
        ):
            subprocess.run(
                ["maturin", "build", "--release", "--manifest-path", manifest, "--out", "dist"],
                check=True,
            )
    elif sys.argv[1:] == ["smoke"]:
        wheels = sorted(Path("dist").glob("*.whl"))
        if len(wheels) != 3:
            raise SystemExit(f"expected three wheels, found {len(wheels)}")
        subprocess.run([sys.executable, "-m", "pip", "install", "pytest", *(str(w) for w in wheels)], check=True)
        subprocess.run(
            [
                sys.executable, "-m", "pytest",
                "bindings/python/tests/test_smoke.py::test_json_extraction_matches_the_shared_rust_contract",
                "bindings/sc-composer-beads-python/tests/test_contract.py::test_receipt_decode_errors_have_a_receipt_code_and_stage",
                "bindings/sc-sha-python/tests/test_compatibility.py::test_file_vector_and_result_shape",
            ],
            check=True,
        )
    else:
        raise SystemExit("expected build or smoke")


if __name__ == "__main__":
    main()
