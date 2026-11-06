"""Construct the stable-ABI exception classes of the native sc-compose module.

The Rust adapter loads this file with ``include_str!`` and reads the six
classes below; their names, ``builtins`` module, constructor signature and
attributes are the public Python contract.
"""

import inspect as _inspect
from collections.abc import Sequence as _Sequence


def _sc_compose_error_init(self, *args, **kwargs):
    if kwargs:
        raise TypeError(f"{type(self).__name__}() takes no keyword arguments")
    if not 1 <= len(args) <= 6:
        raise TypeError(f"{type(self).__name__}() takes from 1 to 6 positional arguments")
    message = args[0]
    code = args[1] if len(args) > 1 else None
    recovery_hints = args[2] if len(args) > 2 else None
    diagnostic_kind = args[3] if len(args) > 3 else None
    diagnostic_message = args[4] if len(args) > 4 else None
    diagnostic_occurrence = args[5] if len(args) > 5 else None
    if not isinstance(message, str) or (code is not None and not isinstance(code, str)):
        raise TypeError("message and code must be strings")
    if recovery_hints is not None and (
        isinstance(recovery_hints, str)
        or not isinstance(recovery_hints, _Sequence)
        or not all(isinstance(hint, str) for hint in recovery_hints)
    ):
        raise TypeError("recovery_hints must be a sequence of strings or None")
    if any(
        value is not None and not isinstance(value, str)
        for value in (diagnostic_kind, diagnostic_message)
    ):
        raise TypeError("diagnostic fields must be strings or None")
    if diagnostic_occurrence is not None and (
        not isinstance(diagnostic_occurrence, int) or diagnostic_occurrence < 0
    ):
        raise TypeError("diagnostic_occurrence must be a non-negative integer or None")
    Exception.__init__(self, *args)
    self.message = message
    self.code = code
    self._recovery_hints = list(recovery_hints) if recovery_hints is not None else []
    self.diagnostic_kind = diagnostic_kind
    self.diagnostic_message = diagnostic_message
    self.diagnostic_occurrence = diagnostic_occurrence


_sc_compose_error_init.__signature__ = _inspect.Signature(
    [
        _inspect.Parameter("self", _inspect.Parameter.POSITIONAL_OR_KEYWORD),
        _inspect.Parameter("message", _inspect.Parameter.POSITIONAL_OR_KEYWORD),
        *[
            _inspect.Parameter(name, _inspect.Parameter.POSITIONAL_OR_KEYWORD, default=None)
            for name in (
                "code",
                "recovery_hints",
                "diagnostic_kind",
                "diagnostic_message",
                "diagnostic_occurrence",
            )
        ],
    ]
)

ScComposeError = type(
    "ScComposeError",
    (Exception,),
    {
        "__module__": "builtins",
        "__init__": _sc_compose_error_init,
        "recovery_hints": property(lambda self: list(self._recovery_hints)),
    },
)
ScRenderError = type("ScRenderError", (ScComposeError,), {"__module__": "builtins"})
ScValidationError = type("ScValidationError", (ScComposeError,), {"__module__": "builtins"})
ScResolveError = type("ScResolveError", (ScComposeError,), {"__module__": "builtins"})
ScIncludeError = type("ScIncludeError", (ScComposeError,), {"__module__": "builtins"})
ScConfigError = type("ScConfigError", (ScComposeError,), {"__module__": "builtins"})
