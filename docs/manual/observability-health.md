# sc-compose observability-health

When you need to know whether the CLI can write and maintain its logs, use
`observability-health`. It starts the same process-local logger configuration as
an ordinary `sc-compose` invocation, reads the logger's current health, and
reports the result without rendering templates or changing composition
behavior. The command does not contact a daemon or require a background
runtime.

## Basic usage

```console
sc-compose observability-health
```

The default text report includes the overall logger state, the active log path,
dropped-event and flush-error counters, query health, retained-log maintenance
state, and each configured sink's state. The active path is platform-specific;
on Windows it may include a drive letter.

## JSON output

Use `--json` when a health check is consumed by CI or another program:

```console
sc-compose observability-health --json
```

JSON is emitted as the standard versioned `DiagnosticEnvelope`:

```json
{
  "schema_version": "1",
  "payload": {
    "logging": {
      "state": "Healthy",
      "dropped_events_total": 0,
      "flush_errors_total": 0,
      "active_log_path": "<log_root>/logs/sc-compose.log.jsonl",
      "sink_statuses": [],
      "last_error": null,
      "last_writer_error": null,
      "maintenance": {
        "state": "Running",
        "last_error": null,
        "last_pass_at": null,
        "pruned_files_total": 0,
        "rotated_files_total": 0
      },
      "query": {
        "state": "Healthy",
        "last_error": null
      },
      "writer_state": "Running",
      "queue_capacity": 1024,
      "queue_depth": 0,
      "queue_full_drops_total": 0,
      "queue_high_water_mark": 0
    }
  },
  "diagnostics": []
}
```

The exact counters, sink list, and path vary with process state and logger
configuration. Treat `state`, `writer_state`, `maintenance.state`, and
`query.state` as the machine-readable status fields; do not parse the text
report.

## Options

| Option | Purpose |
|---|---|
| `--json` | Emit the health report in the standard diagnostic envelope. |
| `--help` | Show command-line usage. |

There are no template, root, or output-file arguments. The command observes the
logger configured for this process and does not mutate composition inputs.

## Common results and recovery

- `Healthy` means the logger, writer, query path, maintenance service, and
  reported sinks are operating normally at the time of the snapshot.
- `Degraded` or `Unavailable` means the snapshot contains the relevant error
  or state fields. Inspect `last_error`, `last_writer_error`, maintenance
  errors, and query errors in JSON before deciding whether to retry or repair
  the log root.
- A non-existent or unwritable log root can prevent normal logger startup. The
  CLI reports that configuration/startup failure through its standard
  diagnostic and exit-code behavior; fix the log-root permissions or
  configuration, then rerun the command.

`observability-health` itself returns success after it produces a health
snapshot, even when the snapshot records a degraded logger state. Use the
reported state fields as the health-check result and use `sc-compose help
exit-codes` for the general process-status contract.
