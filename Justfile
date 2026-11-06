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

# Preserve the legacy fast checks alongside the released native analyzers.
# Identity literals remain skipped due to the legacy parser's unicode failure.
lint-ci-consumer: ensure-lint-runtime
    cargo fmt --all --check
    python3 .just/check_version_sync.py
    python3 .just/lint_manifests.py
    python3 .just/lint_codespell.py
    python3 .just/run_pytests.py
    python3 .just/lint_cargo_deny.py --root .
    cargo shear
    {{sc-compose}} lint --root . --target sc-boundary --json
    {{sc-compose}} lint --root . --target sc-portability --json
    {{sc-compose}} lint --root . --target line-counts --json
    @set -euo pipefail; report="$$(mktemp)"; trap 'rm -f "$$report"' EXIT; SC_COMPOSE_TEMPLATE_CONTRACTS_SCOPE=production {{sc-compose}} lint --root . --target template-contracts --json >"$$report"; jq -e '.payload.command_id == "template-contracts" and .payload.outcome == "pass" and .payload.raw_payload.data.scope == "production" and (.payload.raw_payload.data.templates_scanned > 0) and (.payload.findings_count == 0)' "$$report" >/dev/null
    @echo "sc-lint identity-literals skipped: v0.4.0 parser rejects Rust unicode escapes"

