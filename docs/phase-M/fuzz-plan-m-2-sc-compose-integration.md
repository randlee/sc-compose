# Fuzz Plan: Sprint M.2 (sc-compose / sc-sha integration)

## Targets
- `crates/sc-composer/src/include/expansion.rs` — `@<path>` include graph discovery (conditional-candidate enumeration, cycles, depth limits, path confinement)
- `crates/sc-compose` composition-hash path — `calculate_hash` / `calculate_composition_hash` call sites
- `bindings/sc-sha-python` — maturin adapter surface (result/error mapping)

## Approach
- Use `sc-adversarial-fuzz-coordinator` against the include-graph resolver: malformed/nested/dynamic `@<path>` directives, cycles, missing files, confinement escapes, unresolved `{{ }}`-containing targets.
- Use a second pass against the Python adapter boundary: malformed inputs across the FFI edge, error-shape drift vs Rust.
- No separate plan-review round needed — this is a fuzz test, not a sprint deliverable. If a probe/worker fails or times out, just re-run it.

## Output
- Findings filed as GitHub issues (existing `Fuzz: ...` convention) or routed straight into the M.2 fix cycle if blocking.
- Fuzz run report goes to `site/reports/<run-id>-fuzz-report(.html)`, per the standard `docs/sprint-fuzz-run-report-template.md` artifact layout.
