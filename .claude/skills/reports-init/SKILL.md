# Reports Init

Use this skill to scaffold the shared reporting layout in a consumer repo
without inventing a repo-local contract.

## What This Skill Owns

- run `sc-compose reports init --root .`
- append the shared `Justfile` stubs from `assets/Justfile.append.j2`
- keep producer bodies repo-owned:
  - `just lint`
  - `just test`
  - `just smoke`
- keep wrapper-only helpers such as `just reports-open` out of the shared
  contract

## Expected Outputs

- `reports/catalog/reports.toml`
- `reports/latest/`
- `reports/archive/`
- `reports/templates/`
- `reports/smoke/reference-template.html.j2`
- `reports/smoke/sample-vars.json`
- `Justfile` stubs for `lint`, `test`, `smoke`, `reports`, and
  `reports-verify`

## Notes

- `sc-compose` owns the scaffold shape and the shared smoke harness contract.
- The consumer repo owns the real producer bodies and any repo-specific
  report templates.
- `just reports` and `just reports-verify` stay reserved for the shared
  aggregator and verifier; Sprint B5 wires the real implementation.
