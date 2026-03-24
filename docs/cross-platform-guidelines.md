# SC-Compose Cross-Platform Guidelines

## Required Rules

1. Do not hardcode `/tmp` in production code or tests.
2. Use `std::env::temp_dir()` or `tempfile::TempDir` for temporary paths.
3. Do not set `HOME` or `USERPROFILE` directly in tests unless the test is
   explicitly about home-directory resolution.
4. Prefer repo-specific env vars such as `SC_COMPOSE_*` over ambient OS-home
   mutation.
5. Use `PathBuf` and `.join()` for path construction.
6. Do not assume Unix-only socket, path, or executable behavior unless the code
   is cfg-gated accordingly.

## Test Rules

1. Tests must isolate filesystem state in temporary directories.
2. Tests must not rely on platform-specific default paths when explicit test
   paths can be provided.
3. Tests that spawn subprocesses must use bounded waits and explicit teardown.
