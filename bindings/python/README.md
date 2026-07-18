# sc-compose Python bindings

`sc-compose` provides Python bindings for the standalone template-composition
engine from the `sc-compose` project.

The package targets Python 3.11+ and exposes the composition APIs as a native
extension built with `maturin` and `pyo3`.

## Install

```bash
pip install sc-compose
```

For release rehearsals, pre-release artifacts are published to TestPyPI:

```bash
pip install -i https://test.pypi.org/simple/ sc-compose
```

## Example

```python
from sc_compose import render_template

result = render_template("Hello {{ name }}", {"name": "world"})
print(result.output)
```

## Project

- Repository: <https://github.com/randlee/sc-compose>
- Documentation: see the repository `README.md` and `docs/`
