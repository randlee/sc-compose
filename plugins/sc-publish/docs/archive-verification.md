# Staged archive verification

Declare an optional `archive_verification` argv array in the consumer install
JSON, then rerun the installer. The installer renders the declaration into the
artifact manifest and adds the call to the existing Release and Prerelease
Archive packaging steps. Without the declaration, workflow bytes and the
rendered manifest remain unchanged.

```json
"archive_verification": [
  "python3", "scripts/verify-archive.py",
  "{stage_dir}", "{target}", "{manifest}"
]
```

The command runs once for each staged CLI archive, after binaries and bundled
paths are copied and before the archive is created or uploaded. A nonzero exit,
missing command or 900-second timeout fails packaging and prevents its upload.
The kit helper uses the packaging interpreter. The declared command runs
from the repository root without a shell; metacharacters are
ordinary argv data. `{stage_dir}` and `{manifest}` expand to absolute paths,
and `{target}` is the declared release target. Only these placeholders are
supported; use `{{` and `}}` for literal braces. The staged directory must exist.

Verification is consumer-owned: declare the checks appropriate to the archive.
The kit adds no verification requirement when the array is absent. Python
wheel/sdist, npm and Go module build paths are unaffected; Go module bundles
retain their separate declared build and stage hooks.
