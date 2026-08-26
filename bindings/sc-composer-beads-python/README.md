# sc-composer-beads Python bindings

`sc-composer-beads` exposes the versioned Beads formula-composition contract
from the Rust library to Python 3.11+ through Maturin and PyO3.

The binding invokes the Rust library directly. It does not shell out to
`sc-compose`, accept arbitrary commands, or bypass persistent-pour
authorization.
