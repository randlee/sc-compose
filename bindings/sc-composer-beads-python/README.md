# sc-composer-beads Python bindings

`sc-composer-beads` exposes the versioned Beads formula-composition contract
from the Rust library to Python 3.9+ through Maturin and PyO3.

The binding invokes the Rust library directly. It does not shell out to
`sc-compose`, accept arbitrary commands, or bypass persistent-pour
authorization.

Revising an existing graph uses `repour(request, dry_run=True)` or
`preview_repour(request)` to inspect the planned changes without writing
beads. Pass `allow_uncommitted=True` only when deliberately overriding a
refusal for a dirty or unknown database state. A request can name its existing
pour root with `parent`, or callers can supply `root=`. For an attached graph,
keep the request's `parent` and `ref`. Use `pour` or `preview_pour` with an
existing target to check whether a changed graph requires repour. Revision
receipts expose `graph["changes"]`; typed `BeadComposeError` values expose
structured `details` and `recovery` for refusals and partial failures.
