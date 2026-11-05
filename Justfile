set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Prepare the verified release wheel before delegating to managed recipes.
python := if os_family() == "windows" { "python scripts/sc_lint_python.py" } else { "python3 scripts/sc_lint_python.py" }

sc-compose := "cargo run --quiet --bin sc-compose --"

# Regenerate committed UniFFI Go output. Use `just generate-sc-sha-go check`
# in CI to fail when the pinned generator would change it.
generate-sc-sha-go mode="write":
    case "{{mode}}" in write|check) ;; *) echo "mode must be write or check" >&2; exit 2 ;; esac
    uniffi-bindgen-go --out-dir bindings/sc-sha-go/go --config bindings/sc-sha-go/uniffi.toml bindings/sc-sha-go/src/sc_sha_go.udl
    if [ "{{mode}}" = "check" ]; then git diff --exit-code -- bindings/sc-sha-go/go; fi

# Regenerate or verify checked-in Rust scaffolding with uniffi-bindgen 0.31.2.
generate-sc-sha-rust mode="write":
    python_cmd="$(command -v python3 || command -v python)"; "$python_cmd" scripts/generate_sc_sha_scaffolding.py --mode "{{mode}}"

# Prepare the ignored, host-native static library required only to run the
# source-tree Go tests. Released consumers use a self-contained bundle instead.
[unix]
prepare-sc-sha-go-native:
    python_cmd="$(command -v python3 || command -v python)"; "$python_cmd" bindings/sc-sha-go/prepare_native.py

[windows]
prepare-sc-sha-go-native:
    {{python}} bindings/sc-sha-go/prepare_native.py

# Cross-target verification of the shipped Go native contract. REQUIRED for the
# Go contract: the gnu/gnullvm triples below, because native_loader.go ships
# windows/amd64 as x86_64-pc-windows-gnu and windows/arm64 as
# aarch64-pc-windows-gnullvm. The msvc cargo-xwin checks (sc-lint check.xwin /
# clippy.xwin) verify the Windows CLI contract only and do NOT stand in for
# these. Both are compile-time `cargo check`s; neither links or runs Go (see
# docs/architecture.md section 3.5, "Cross-target verification").
check-sc-sha-go-cross:
    cargo check -p sc-sha-go --target x86_64-pc-windows-gnu
    cargo check -p sc-sha-go --target aarch64-pc-windows-gnullvm

ensure-lint-runtime:
    python3 scripts/materialize_sc_lint_runtime.py --root .

reports-init:
    {{sc-compose}} reports init --root .

template-contracts:
    {{sc-compose}} lint --root . --target template-contracts --json

view target="findings":
    {{sc-compose}} lint --root . --target view-{{target}} --json

check target="native":
    PYO3_CROSS_PYTHON_VERSION="${PYO3_CROSS_PYTHON_VERSION:-3.11}" {{sc-compose}} lint --root . --target check-{{target}} --json

clippy target="native":
    PYO3_CROSS_PYTHON_VERSION="${PYO3_CROSS_PYTHON_VERSION:-3.11}" {{sc-compose}} lint --root . --target clippy-{{target}} --json

ci:
    {{sc-compose}} lint --root . --target ci-all --json

test:
    cargo test --workspace
    {{sc-compose}} report-render-many --root . --id test-evidence --glob 'reports/inputs/test/*.md' --template-family test --output-dir reports/latest/test-evidence/panels
    mkdir -p reports/latest/test-evidence
    {{sc-compose}} render --mode file --root . --file examples/report-evidence-summary.html.j2 --var-file reports/vars/test-evidence-summary.json --output reports/latest/test-evidence/index.html
    {{sc-compose}} reports finalize --root . --report-id test-evidence --kind test --entrypoint reports/latest/test-evidence/index.html --artifact reports/latest/test-evidence/index.html --artifact reports/latest/test-evidence/panels/manifest.json --artifact reports/latest/test-evidence/panels/reports/inputs/test/results.html --artifact reports/latest/test-evidence/panels/reports/inputs/test/matrix.html --archive

smoke:
    mkdir -p reports/latest/smoke
    {{sc-compose}} reports smoke --root . --fixture reports/smoke/reference-template.html.j2 --vars reports/smoke/sample-vars.json

state-diagrams:
    {{sc-compose}} report-render-many --root . --id state-diagrams --glob 'reports/specs/state-diagrams/*.toml' --template-family diagram --output-dir reports/latest/state-diagrams/panels
    mkdir -p reports/latest/state-diagrams
    {{sc-compose}} render --mode file --root . --file examples/report-evidence-summary.html.j2 --var-file reports/vars/state-diagrams-summary.json --output reports/latest/state-diagrams/index.html
    {{sc-compose}} reports finalize --root . --report-id state-diagrams --kind state_machine --entrypoint reports/latest/state-diagrams/index.html --artifact reports/latest/state-diagrams/index.html --artifact reports/latest/state-diagrams/panels/manifest.json --artifact reports/latest/state-diagrams/panels/reports/specs/state-diagrams/approval-flow.html --artifact reports/latest/state-diagrams/panels/reports/specs/state-diagrams/retry-loop.html --archive

sql-diagrams:
    {{sc-compose}} report-render-many --root . --id sql-diagrams --glob 'reports/specs/sql-diagrams/*.toml' --template-family diagram --output-dir reports/latest/sql-diagrams/panels
    mkdir -p reports/latest/sql-diagrams
    {{sc-compose}} render --mode file --root . --file examples/report-evidence-summary.html.j2 --var-file reports/vars/sql-diagrams-summary.json --output reports/latest/sql-diagrams/index.html
    {{sc-compose}} reports finalize --root . --report-id sql-diagrams --kind sql_query --entrypoint reports/latest/sql-diagrams/index.html --artifact reports/latest/sql-diagrams/index.html --artifact reports/latest/sql-diagrams/panels/manifest.json --artifact reports/latest/sql-diagrams/panels/reports/specs/sql-diagrams/publish-manifest.html --artifact reports/latest/sql-diagrams/panels/reports/specs/sql-diagrams/release-summary.html --archive

reports:
    mkdir -p reports/latest/report-evidence-summary
    {{sc-compose}} render --mode file --root . --file examples/report-evidence-summary.html.j2 --var-file examples/report-evidence-summary.sample-vars.json --output reports/latest/report-evidence-summary/index.html
    {{sc-compose}} reports finalize --root . --report-id report-evidence-summary --kind custom --entrypoint reports/latest/report-evidence-summary/index.html --artifact reports/latest/report-evidence-summary/index.html --archive
    {{sc-compose}} reports index --root .
    {{sc-compose}} reports publish-manifest --root .

reports-verify:
    {{sc-compose}} reports verify --root .

# >>> sc-lint managed integration >>>
set windows-shell := ["cmd.exe", "/c"]
import '.sc-lint/justfile'
# <<< sc-lint managed integration <<<
