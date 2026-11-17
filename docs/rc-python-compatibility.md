# Release-candidate Python compatibility

Declare `rc_python_compatibility` in the consumer install JSON and rerun the
installer. It adds a compatibility job to the existing Release Candidate
workflow and a required evidence check before publication in Release. Only
the Release gate job receives Actions read permission for that evidence lookup. Without
this declaration, both workflow files and their permissions remain unchanged.
No additional workflow, evidence artifact, ledger or secret is introduced.

```json
"rc_python_compatibility": {
  "versions": ["3.9", "3.11", "3.12", "3.13", "3.14", "3.15"],
  "build_python": "3.11",
  "setup": ["python", "-m", "pip", "install", "build==1.3.0"],
  "build": ["python", "scripts/build-compatibility-wheels.py"],
  "smoke": ["python", "scripts/smoke-compatibility-wheels.py"],
  "runner_overrides": {"ubuntu-latest": "ubuntu-24.04"},
  "exclusions": {"windows-11-arm": ["3.9"]}
}
```

Platforms come from the declared Python wheel runners, including both legacy
runner strings and explicit wheel objects' `os` fields. Each platform runs one
candidate job, checking out exactly the candidate commit. It builds the wheels
once, switches interpreters, and smoke-tests those same wheels on each declared
version except that platform's exclusions. Every smoke must succeed. An
exclusion cannot remove every interpreter from a platform. The runner override
changes the execution host while preserving the declared platform identity.

`build` and `smoke` are required consumer-owned argv arrays; `setup` is optional.
Hooks run without a shell from the candidate checkout, using the active Python
interpreter's PATH, with a 900-second timeout. An optional `rust_toolchain`
selects a native Rust compiler for consumers whose wheel builders need it.
Consumers remain responsible for testing their real public API in the smoke
hook. A candidate lacking the declaration cannot pass the generated job.

Release reads the policy at its resolved release commit and resolves the
candidate tag to its exact SHA. It requires a successful Release Candidate
workflow run containing successful `python-compat (<platform>, <candidate SHA>)`
jobs for every declared platform. Missing, failed, skipped or cancelled platform
jobs do not qualify; API or Git errors refuse release. This is job-level
success evidence, with no per-interpreter evidence protocol. Compatibility runs
only in Release Candidate, never again in Release or Release Preflight.
