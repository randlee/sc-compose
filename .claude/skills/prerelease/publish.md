# Publish a prerelease

Publishing pushes a tag and causes `prerelease-archive.yml` to publish GitHub
Release assets. Treat it as an external state change. Proceed beyond the dry
run only when the operator explicitly requested `--publish` (or its
compatibility alias `--create`) in writing and the working branch is clean,
non-protected, and attached.

The prerelease carries every manifest-declared asset a real release builds:
the binary archives (only when the manifest declares release binaries), every
declared Python wheel for every platform, each declared sdist, and
`checksums.txt`. It never publishes to crates.io, PyPI, Homebrew, Winget,
Scoop, npm, or another consumer-facing channel. The dry run prints the asset
counts it will verify.

Always invoke the consumer's manifest-declared tag script. That script owns
the repository's version-selection and tag policy: it may use the current
workspace version only when that version has never been published; otherwise
it must patch-bump before tagging. Reject an explicit version. The script and
workflow must fail closed rather than overwrite or republish a version.

```bash
set -euo pipefail
python3 .claude/skills/prerelease/scripts/prerelease.py --publish --dry-run
python3 .claude/skills/prerelease/scripts/prerelease.py --publish --authorized
```

The helper waits up to 20 minutes for the archive workflow for the newly tagged
commit, polling once per minute, and verifies every manifest-declared asset and
checksum (and that `checksums.txt` covers every Release asset) before
reporting success.
