---
name: prerelease
description: List, install, or publish manifest-declared GitHub prerelease assets (binary archives and Python wheels). Use for /prerelease --list, --install, or --publish operations.
---

# Prerelease

Operate only on GitHub prerelease assets declared by
`release/publish-artifacts.toml`. Never publish Homebrew, crates.io, PyPI,
Winget, Scoop, or another package channel from this skill.

`--publish` creates one GitHub prerelease carrying the same manifest-declared
assets a real release builds: one binary archive per release target when the
manifest declares release binaries, every declared Python wheel for every
platform, each declared sdist, and `checksums.txt`. A consumer with no release
binaries (`release_binaries = []`) gets a prerelease of its wheels and sdists.

## Step 1 — Verify dependencies

```bash
which python3 && python3 --version
python3 -c 'import sys; sys.exit("Python 3.11+ is required") if sys.version_info < (3, 11) else None'
which git && git --version
which gh && gh --version
gh auth status
```

`--install X.Y.Z` (or `--install` for the latest) also works on a headless
host where `gh` is absent or unauthenticated, provided the repository is public
and `[project].homepage` is its `https://github.com/OWNER/REPO` URL: the helper
then reads the release through the public GitHub API and downloads assets
from public Release URLs, with the same asset and checksum checks. `--list`
and `--publish` always require an authenticated `gh`.

If a dependency is absent from `PATH`, check common installation locations as
described in
[references/installation-and-troubleshooting.md](references/installation-and-troubleshooting.md).
Stop if a required command or GitHub authentication is unavailable, except
that headless `--install` needs only Python 3.11+ as described above.

## Route the requested mode

Read and follow exactly one mode file; do not load the other mode procedures.

- `--list`: read [list.md](list.md).
- `--install [X.Y.Z]`: read [install.md](install.md). Omit the version to
  select the latest matching prerelease.
- `--publish` (`--create` is a compatibility alias): read
  [publish.md](publish.md). This mode always invokes the
  manifest-declared version-selection and tag command. Reject a version
  argument; prerelease creation may never reuse a published version.

Require exactly one mode. Run from the consumer repository root, fail closed
when `[prerelease]` is absent, and take all repository-specific values from the
manifest. Do not hardcode project names, tag prefixes, binaries, targets,
installation paths, selectors, or verification commands.
