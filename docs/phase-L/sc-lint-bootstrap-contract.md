# sc-lint Bootstrap Contract

## Status

Accepted for Phase L.1 on `sprint/l-1-sc-lint-bootstrap`. The installed
consumer contract is pinned to sc-lint `0.4.0` and is verified through its
machine-readable `version --json` response.

## Ownership boundary

`sc-lint` owns analyzer rules, target dispatch, findings, and the stable JSON
command envelope. `sc-compose` owns only invocation, exit-status handling,
normalization into its report model, and XHTML/HTML artifact materialization.
Neither Rust crate embeds sc-lint as a Cargo dependency, and no analyzer rule
or Python runner is copied into this repository. `sc-composer` remains a pure
rendering library.

The repository configuration records the supported tool version and report
locations in [`sc-lint.toml`](../../sc-lint.toml) and the cargo-deny policy in
[`deny.toml`](../../deny.toml). The setup action installs
the release archive containing `sc-lint`, `sc-lint-boundary`,
`sc-lint-portability`, and `sc-lint-runtime`, then verifies the version before
an analyzer target is used.

## Boundary inventory

The canonical inventory under [`boundaries/`](../../boundaries/) models the
actual workspace edges:

| Owner | Dependents | Allowed dependencies |
| --- | --- | --- |
| `sc-composer` | `sc-compose`, `sc-compose-py` | none |
| `sc-compose` | none | `sc-composer` |
| `sc-compose-py` | none | `sc-composer` |

This is an ownership contract, not a copy of sc-lint's own inventory. Future
feature sprints may add narrow boundary records only when ownership changes;
they must not widen the dependency direction.

## Machine error taxonomy

The sc-compose integration uses these stable classes when normalizing sc-lint
outcomes:

- `CLI.CONFIG_ERROR`: repository root, boundary inventory, `sc-lint.toml`,
  release/version, or required utility is missing or malformed. The caller
  should fix configuration or installation before retrying.
- `CLI.CAPABILITY_ERROR`: the repository is configured correctly, but the host
  lacks a required capability such as a target toolchain or Windows-only
  utility. The caller should install/enable the capability or mark the target
  unsupported.

No unnamed “capability error” string is part of the contract. L.2 will encode
these classes in the closed result enum and preserve the original sc-lint
diagnostic details.

## Python utility availability

sc-lint 0.4.0 has Python-backed adapters for line counts, identity literals,
view findings, and related workflow helpers. The adapter currently resolves
consumer-relative `.just/` paths. The CI setup action therefore downloads the
pinned sc-lint source archive and materializes those utilities into the
runner workspace; no utility is copied into or maintained in sc-compose.

This packaging gap is tracked by [sc-lint issue #83](https://github.com/randlee/sc-lint/issues/83), which requests a pip-installable/maturin-backed
distribution or equivalent embedded-resource/module entrypoint. L.17 will
inventory the duplicate surface and propose the concrete shared package
shape, linked to #83.

## Required smoke commands

The setup action verifies the release first. Repository bootstrap validation
then uses:

```text
sc-lint version --json
sc-lint --json --root . lint sc-boundary
```

Both commands must return the v1 JSON envelope. The second command proves root
discovery and boundary inventory loading; its findings are the analyzer's
authority and are not reimplemented in a local script.
