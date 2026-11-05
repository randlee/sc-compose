# sc-lint adoption kit

From the installed Claude Code plugin, run its installer with:

```bash
python3 "$CLAUDE_PLUGIN_ROOT/install.py" .
```

Use `--dry-run` to report drift without writing.

The marketplace plugin root is supplied by Claude Code. The repository path
`plugins/sc-lint/install.py` is the consumer layout created by an install; it
is not the marketplace entry point.

The first install persists its validated input at `.sc-lint/install.json`.
Use `--input <path>` to override it or bootstrap a new input document.

Kit-rendered steps name only shipped binaries or `sc_lint` modules; profile and
test-layer commands are consumer-owned and rendered verbatim.
