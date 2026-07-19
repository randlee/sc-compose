# Prototype Multipass Reference

This directory is the executable reference implementation for the Phase D
nested-template design.

## Local Setup

- Python: `3.13`
- `PYTHONPATH`: `bindings/python/python`
- Install dependencies:

```bash
python3.13 -m pip install -r prototype/multipass/requirements.txt
```

## Test Commands

Run the focused pytest coverage:

```bash
PYTHONPATH=bindings/python/python python3.13 -m pytest prototype/multipass/test_multipass.py prototype/multipass/test_integration.py -q
```

Run the lightweight harness:

```bash
PYTHONPATH=bindings/python/python python3.13 prototype/multipass/run_tests.py
```

The integration test file imports the maturin-backed `sc_compose` package from
`bindings/python/python`, so that path must stay on `PYTHONPATH` when running
the prototype outside a virtualenv.
