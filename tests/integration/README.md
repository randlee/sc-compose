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

## Optional per-run bd staging on atmbench

Here, **"install" means per-run atmbench-owned temporary bd staging**. It
persists nothing after the run and performs no persistent account installation.

The default `just integration` command uses installed executables and never
installs anything. To qualify another build on the local `atmbench` account,
use the explicit target command (requires SSH access to `atmbench@localhost`):

```sh
just integration-atmbench --mode server --install-bd-from /path/to/fork/bd
just integration-atmbench --mode embedded --install-bd-from /path/to/upstream/bd
```

`--install-bd-from` copies that executable into an atmbench-owned temporary
`bin/bd`, ahead of the target's PATH. It does not replace the account's normal
installation. Without the option, the target command uses the existing bd
and performs no installation. The target copies the built CLI, suite, examples
and report panels into its own temporary directory, runs integration and the
same two fuzz seeds serially, returns XHTML evidence, and removes its files
only after checking for live processes.

Use the stock upstream **v1.3.4 Darwin arm64** release for embedded mode. The
local randlee/beads fork used for server qualification is built without CGO
and cannot run embedded mode. Obtain the upstream archive and checksums from
the [v1.3.4 release](https://github.com/gastownhall/beads/releases/tag/v1.3.4),
verify the archive against `checksums.txt`, and extract its `bd` executable.
Both builds must report 1.3.4; the suite records the exact version and binary
SHA-256 rather than treating the fork and release as equivalent.

Each mode uses a fresh isolated home and database; embedded mode initializes
without server flags and starts no external Dolt server. Server mode retains
the w-5 owned-server setup. `~/.beads` and port 3308 are excluded in both
modes. Reports preserve separate panels for target, platform, exact bd version
and mode. Run the two commands sequentially, without parallel fan-out.

Stock bd v1.3.4 does not implement `bd sql` in embedded mode. That panel tests
the exact `BEADS_GRAPH_DIRTY_UNKNOWN` refusal, unchanged stored state, and
successful `--allow-uncommitted` override. It does not claim embedded
clean/dirty detection. The fork's server panel tests actual clean and dirty
database detection as in w-5.

The storage mode can also be selected without the target installer:

```sh
python3 scripts/integration_repour.py --mode embedded --fuzz
```
