# Release under an epic

Copy this directory into your Beads workspace. Use production `bd` 1.3.1 and
`sc-compose` with the attach commands. In a fresh workspace, run:

```shell
bd init --non-interactive --skip-agents --skip-hooks
bd create "Release 1.6.1" --type epic --json
mkdir -p build
```

In `request.json`, set `working_directory` to this workspace's absolute path,
set `rendered_formula` to its absolute `build/release.formula.toml` path, and
replace `parent` with the epic id returned by `bd create`. Set `bd_executable`
if `bd` is not on `PATH`. Keep the template in the workspace.

```shell
sc-compose bead preview-attach --request request.json --json
sc-compose bead attach --request request.json --json
sc-compose bead attach --request request.json --json
```

Preview writes no beads. Attach creates `build`, `verify`, and `publish` directly
under the epic with `blocks` dependencies. The repeat reports the same ids and
`existing` actions, without changing any beads or edges. No `formulas/` registry
is needed. The installed manual has the same recipe in `sc-compose help bead`.
