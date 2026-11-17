# Native Go module artifacts

Declare `go_modules` in the install JSON, then rerun the installer. Omission
preserves the existing artifact plan and never sets up Go or a native compiler.
The installer renders the same declarations into `release/publish-artifacts.toml`.
For example:

```json
"go_modules": [{
  "module": "example.test/team/project/binding",
  "source": "binding",
  "artifact_prefix": "project-binding",
  "targets": [{
    "target": "aarch64-apple-darwin",
    "os": "macos-15",
    "goos": "darwin",
    "goarch": "arm64"
  }],
  "build": ["python3", "scripts/build-native.py", "{target}"],
  "stage": ["python3", "scripts/stage-module.py", "{target}", "{stage}", "{version}"],
  "consumer_test": "tests/go-consumer"
}]
```

`prepare`, `build` and `stage` are optional consumer-owned argv arrays, executed
without a shell from the repository root. Supported substitutions are the module
and target fields plus `{root}`, absolute `{source}`, `{version}`, and, for the
stage hook, `{stage}`. A prepare hook runs in a separate Actions step so it can
configure native tools through `GITHUB_PATH` or `GITHUB_ENV`. `rust_toolchain`
optionally selects the Rust compiler and target for native adapters. There is no
Rust requirement for other Go modules. Without a stage hook, the module source
is copied, excluding `.git`. Sources must remain within the repository.

Both existing Release Preflight and Release workflows build on each declared
native runner, check the actual GOOS/GOARCH, and run `go test ./...` on both the
staged module and its zipped/unpacked copy. When `consumer_test` is declared,
each copy is also imported by that independent consumer module using a local
module replacement; its tests and executable must pass. Build hooks are
responsible for supplying their real native library, and consumer tests should
exercise the native API.

Every declared target produces exactly `<artifact_prefix>-<target>.zip` with
`go.mod` at the archive root. Verification refuses missing, empty, duplicate,
wrong-module or extra Go bundles. The production release tag and all publication
jobs wait for Go verification. Non-Go consumers retain the existing tag reuse
and rehearsal behavior; Go consumers require an existing release tag to point
at exactly the verified build commit.

Production release attaches the verified bundles alongside other manifest
artifacts. Before creating the GitHub Release it creates or reuses immutable
`<source>/v<version>` module tags at the verified release commit. An existing tag
at another commit is refused. Each module must then resolve through
`GOPROXY=https://proxy.golang.org go list -m <module>@v<version>`; a lookup error
stops release. Rehearsals and preflight never push these tags. A successful tag
push remains immutable if a later proxy lookup fails; retry reuses the same tag.
The publication procedure follows the [Go module publishing guide](https://go.dev/doc/modules/publishing).

Continuity treats the Go module path and each native ZIP as publication
identities. Its Go tag baseline covers exactly the previous GitHub release
version, so independently versioned submodules are outside that baseline.


Go preflight jobs run outside the existing channel summary/deny step. Their
individual job results and the `go-verified` job must pass for the entire
preflight workflow to pass; the channel summary alone is not Go qualification.
The root-module source `.` is currently rejected: this artifact kind supports
repository-relative subdirectories and their `<source>/v<version>` tags only.
Each command has a 900-second timeout. Independent consumer qualification runs
`go mod tidy` and can require network access to resolve its dependencies.

Exact compound wheel matching and additional manylinux policies apply only
when publication continuity is declared. Other consumers retain the existing
manylinux 2_28-only policy and compound-tag subset matching.
