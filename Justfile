set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

sc-compose := "cargo run --quiet --bin sc-compose --"

# Regenerate committed UniFFI Go output. Use `just generate-sc-sha-go check`
# in CI to fail when the pinned generator would change it.
generate-sc-sha-go mode="write":
    case "{{mode}}" in write|check) ;; *) echo "mode must be write or check" >&2; exit 2 ;; esac
    uniffi-bindgen-go --out-dir bindings/sc-sha-go/go --config bindings/sc-sha-go/uniffi.toml bindings/sc-sha-go/src/sc_sha_go.udl
    if [ "{{mode}}" = "check" ]; then git diff --exit-code -- bindings/sc-sha-go/go; fi

# Prepare the ignored, host-native static library required only to run the
# source-tree Go tests. Released consumers use a self-contained bundle instead.
prepare-sc-sha-go-native:
    host="$(rustc -vV | awk '/^host:/ { print $2 }')"; \
    case "$host" in \
      x86_64-unknown-linux-gnu|aarch64-apple-darwin|x86_64-pc-windows-gnu) library="target/debug/libsc_sha_go.a" ;; \
      *) echo "unsupported sc-sha-go host target: $host" >&2; exit 2 ;; \
    esac; \
    cargo build -p sc-sha-go; \
    python3 scripts/release_artifacts.py install-go-native-library \
      --manifest release/publish-artifacts.toml --target "$host" --native-library "$library"

ensure-lint-runtime:
    python3 scripts/materialize_sc_lint_runtime.py --root .

reports-init:
    {{sc-compose}} reports init --root .

lint target="full": ensure-lint-runtime
    target="{{target}}"; target="${target#target=}"; {{sc-compose}} lint --root . --target "$target" --json
    target="{{target}}"; target="${target#target=}"; if [ "$target" = "full" ]; then {{sc-compose}} lint --root . --target template-contracts --json; fi

template-contracts:
    {{sc-compose}} lint --root . --target template-contracts --json

# Temporary consumer profile while sc-lint's released full/ci profile is broken
# (sc-lint#84). Restore `lint full` after its profile fix; identity-literals is
# also skipped because v0.4.0 crashes on valid Rust unicode escapes.
lint-ci-consumer: ensure-lint-runtime
    {{sc-compose}} lint --root . --target fast --json
    python3 .just/lint_cargo_deny.py --root .
    cargo shear
    {{sc-compose}} lint --root . --target sc-boundary --json
    {{sc-compose}} lint --root . --target sc-portability --json
    {{sc-compose}} lint --root . --target line-counts --json
    @set -euo pipefail; report="$$(mktemp)"; trap 'rm -f "$$report"' EXIT; SC_COMPOSE_TEMPLATE_CONTRACTS_SCOPE=production {{sc-compose}} lint --root . --target template-contracts --json >"$$report"; jq -e '.payload.command_id == "template-contracts" and .payload.outcome == "pass" and .payload.raw_payload.data.scope == "production" and (.payload.raw_payload.data.templates_scanned > 0) and (.payload.findings_count == 0)' "$$report" >/dev/null
    @echo "sc-lint identity-literals skipped: v0.4.0 parser rejects Rust unicode escapes"

view target="findings":
    {{sc-compose}} lint --root . --target view-{{target}} --json

check target="native":
    {{sc-compose}} lint --root . --target check-{{target}} --json

clippy target="native":
    {{sc-compose}} lint --root . --target clippy-{{target}} --json

ci:
    {{sc-compose}} lint --root . --target ci-all --json

test:
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
