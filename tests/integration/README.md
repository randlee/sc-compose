# Local repour integration and bounded fuzz cases

Build the CLI, then run the existing integration suite followed by two seeded
cases in the **same disposable database**:

```sh
cargo build -j 2 -p sc-compose
python3 scripts/integration_repour.py --fuzz
```

The installed `bd` 1.3.4 fork and `dolt` are prerequisites. The w-5 runner owns
the temporary home, workspace, database, server, readiness signal and teardown.
It uses `bd init --server`, refuses port 3308, and does not use `~/.beads`.
All integration scenarios must pass before any generated case runs. There is
one server and one case at a time; this is not a load or throughput test.

Default seeds are **24034** and **24035**. Each generates exactly three plans
of three active steps, with four distinct step identities in total. Revisions
remove and restore steps and vary acyclic dependencies, titles, descriptions
and priorities. Readback checks IDs, runtime fields, user labels, removed-node
labels and cleanup recommendations, exact internal edges, preserved external
edges in both directions, dry-run/apply changes, refusal before writes for a
changed pour, and replay without stored-state changes. Text includes quotes,
equals signs, newlines and Unicode.

Replay one recorded seed (the integration preflight still runs):

```sh
python3 scripts/integration_repour.py --fuzz --fuzz-seed 24034
```

At most two `--fuzz-seed` options are accepted. There is no repeat count,
parallel mode, duration-based loop, or timing assertion. Command and cleanup
deadlines only detect failure to finish.

The dated `site/reports/*-repour-fuzz.xhtml` report preserves a panel per target
and bd version. It records bd version/hash, Dolt version, CLI hash, generated
inputs, seed, observed change receipts, failures and teardown. A failure stops
the run and marks subsequent cases as not run. Reproduction data appears in
the failure report even if a case does not finish.

Controlled generator/oracle checks require no database:

```sh
python3 -m unittest discover -s tests/integration -v
python3 -m unittest discover -s scripts/tests -p test_integration_repour.py -v
```
