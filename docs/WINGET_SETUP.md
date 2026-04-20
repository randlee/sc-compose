# Windows Package Manager (`winget`) Setup

This document explains the `winget` path for `sc-compose` published from the
`sc-compose` repo.

## Package Identity

- Package identifier: `randlee.sc-compose`
- Installed binary: `sc-compose`
- Release source repo: `https://github.com/randlee/sc-compose`

## Release Model

- The first `winget` release requires a one-time manual manifest submission to
  `microsoft/winget-pkgs`.
- After that bootstrap submission, later releases are automated by the release
  workflow via `vedantmgoyal2009/winget-releaser@v2`.
- No `winget`-specific repository secret is required; the default
  `GITHUB_TOKEN` is sufficient for the workflow step.

## Installer Source

The workflow submits the Windows ZIP asset from the GitHub Release:

- `sc-compose_<VERSION>_x86_64-pc-windows-msvc.zip`

That archive contains:

- `bin/sc-compose.exe`
- `share/sc-compose/examples/...`

## Review Lag

Microsoft review normally introduces a 1-2 day lag between submission and
public `winget install` visibility. Release verification for `sc-compose`
therefore checks submission success, not same-day installability.

## First Release Bootstrap

For the initial submission:

1. Build and publish the GitHub Release as usual.
2. Update the template manifest under `.winget/`.
3. Prepare the initial three-file manifest set for `microsoft/winget-pkgs`:
   - version manifest
   - installer manifest
   - locale/default-locale manifest
4. Submit that initial manifest set to `microsoft/winget-pkgs`.
5. After that first package exists, keep using the automated workflow step for
   later releases.
