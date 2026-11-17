# Publication continuity

Add `"publication_continuity": {}` to the consumer install JSON and rerun
the installer. Declaration enables a blocking, read-only check in both Release
Preflight and Release. Omitting the field preserves existing behavior, with no
additional service access. There is no runtime bypass once declared.

The check selects the greatest published, non-draft, stable semantic release
below the requested version. A retry excludes the current version. It reads
that release's actual GitHub assets, its historical artifact manifest, and
public registry version lookups for the historical crate, PyPI and npm names.
It also inventories Go module tags for exactly the previous release version.
Older module tags do not reappear in later baselines. Independently versioned
Go submodules are not covered by this release-version baseline. Non-semver release tags
(including prerelease-looking tags marked stable) are skipped with a note. Formula, bucket and Winget publications
are read through their declared GitHub repositories. Registry errors, incomplete
listings and unavailable historical metadata fail closed. A first GitHub
release has no previous publication baseline.

The baseline is never reconstructed from the new manifest. In particular,
removing a package from the new manifest cannot remove it from the comparison.
A historical GitHub release without an artifact manifest is indeterminate and
is refused; the check does not guess an external publication surface from
arbitrary source files.

Asset identifiers have the form `asset:<filename>` with the release version
replaced by `{version}`, for example
`asset:example_{version}_x86_64-unknown-linux-gnu.tar.gz`. The new manifest must
still produce that artifact family for the declared target. Platform wheels require explicit platform declarations exactly matching the
shipped wheel platform tags. Legacy runner labels are refused. A changed platform
tag drops the old asset family and requires a user-approved deprecation.
Other identifiers are `crate:<package>`, `pypi:<normalized-name>`,
`npm:<package>`, `go:<module>`, `homebrew:<tap>/<formula-path>`,
`scoop:<bucket>/<manifest-path>` and `winget:<identifier>`.

## User-approved retirement

Each dropped item needs its own reviewed manifest record:

```json
"publication_continuity": {
  "deprecations": [{
    "identifier": "crate:retired-library",
    "reason": "The user approved retiring the obsolete library",
    "approved_by": "release-owner",
    "approval_ref": "https://github.com/example/project/issues/42"
  }]
}
```

The approval reference must identify the issue or PR containing user approval;
the fields are documentary evidence for human review, not an automatic grant
of approval. Empty, malformed, duplicate or still-retained entries are refused. Approval is
required on the release that drops an item. On subsequent releases, an entry
absent from both the previous baseline and current plan produces a nonfatal
stale-entry note. The existing Actions step summary labels approved drops with
the identifier, reason and approval reference. No ledger or additional release
asset is created.

## Ordering and extension

Release checks the manifest at its resolved release commit before the first
tag push. Preflight records the same check in its final blocking summary and
per-channel results. Both paths call `publication_continuity.py`; its pure
comparison accepts an injected previous inventory, and `publication_inventory`
owns read-only service access. Tests use controlled listings and never publish.
Consumer declaration is the only switch.
