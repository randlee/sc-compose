#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  build)
    maturin build --release --manifest-path bindings/python/Cargo.toml --out dist
    maturin build --release --manifest-path bindings/sc-composer-beads-python/Cargo.toml --out dist
    maturin build --release --manifest-path bindings/sc-sha-python/Cargo.toml --out dist
    ;;
  smoke)
    python -m pip install pytest dist/*.whl
    python -m pytest \
      bindings/python/tests/test_smoke.py::test_json_extraction_matches_the_shared_rust_contract \
      bindings/sc-composer-beads-python/tests/test_contract.py::test_receipt_decode_errors_have_a_receipt_code_and_stage \
      bindings/sc-sha-python/tests/test_compatibility.py::test_file_vector_and_result_shape
    ;;
  *) echo 'expected build or smoke' >&2; exit 1 ;;
esac
