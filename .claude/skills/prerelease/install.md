# Install a prerelease

Install the requested version, or the latest matching published prerelease
when no version was supplied. The helper requires every manifest-declared
asset on the Release, then checksum-verifies and stages under
`<install_root>/v<version>`:

- the host binary archive's `[prerelease].binaries` under `bin/`, when the
  manifest declares binaries, and repoints the platform selector to them;
- exactly one host wheel per declared Python distribution under `wheels/`.
  Selection is deterministic: the running interpreter's exact CPython ABI
  first, then the highest compatible `abi3` wheel, then pure Python, and a
  platform-specific wheel before `any`. Linux matches `manylinux` on glibc and
  `musllinux` on musl. A tie or no compatible wheel fails closed.

Staging is atomic, so an interrupted download leaves no partial version. It
then runs the manifest's `post_install` and `verify` commands with these
placeholders expanded: `{version}`, `{stage_dir}`, and the shell-quoted
`{python}` (this interpreter), `{selector_dir}`, and `{wheels}` (the staged host
wheel paths). For example, a wheels-only consumer may declare
`post_install = "{python} -m pip install --force-reinstall {wheels}"`.

```bash
set -euo pipefail
version="${VERSION:-latest}"
python3 .claude/skills/prerelease/scripts/prerelease.py --install "$version" --dry-run
python3 .claude/skills/prerelease/scripts/prerelease.py --install "$version"
```

Set `VERSION` only from the optional `--install X.Y.Z` argument. A staged
version may skip downloading, but selector repair and manifest-declared
post-install verification still run.
