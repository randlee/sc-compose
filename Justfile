set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

sc-compose := "cargo run --quiet --bin sc-compose --"

reports-init:
    {{sc-compose}} reports init --root .

lint:
    {{sc-compose}} report-render-many --root . --id sc-lint --glob 'reports/inputs/lint/*.md' --template-family lint --output-dir reports/latest/sc-lint/panels
    mkdir -p reports/latest/sc-lint
    {{sc-compose}} render --mode file --root . --file examples/report-evidence-summary.html.j2 --var-file reports/vars/sc-lint-summary.json --output reports/latest/sc-lint/index.html
    {{sc-compose}} reports finalize --root . --report-id sc-lint --kind lint --entrypoint reports/latest/sc-lint/index.html --artifact reports/latest/sc-lint/index.html --artifact reports/latest/sc-lint/panels/manifest.json --artifact reports/latest/sc-lint/panels/reports/inputs/lint/summary.html --artifact reports/latest/sc-lint/panels/reports/inputs/lint/whitespace.html --archive

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
