"""Construct the stable-ABI equivalent of the native Beads exception."""

import inspect


class BeadComposeError(Exception):
    __module__ = "builtins"
    __slots__ = ("_code", "_message", "_stage", "_details")

    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'builtins.BeadComposeError' is not an acceptable base type")

    def __init__(self, *args, **kwargs):
        if kwargs:
            raise TypeError("builtins.BeadComposeError() takes no keyword arguments")
        if len(args) < 2:
            missing = "code" if not args else "message"
            count = 2 - len(args)
            raise TypeError(
                "BeadComposeError.__new__() missing "
                f"{count} required positional argument{'s' if count != 1 else ''}: "
                + ("'code' and 'message'" if not args else f"'{missing}'")
            )
        if len(args) > 4:
            raise TypeError(
                "BeadComposeError.__new__() takes from 2 to 4 positional arguments "
                f"but {len(args)} were given"
            )
        code, message = args[:2]
        stage = args[2] if len(args) > 2 else None
        details = args[3] if len(args) > 3 else None
        for index, value in enumerate((code, message, stage)):
            if index == 2 and value is None:
                continue
            if not isinstance(value, str):
                name = "None" if value is None else repr(type(value).__name__) + " object"
                raise TypeError(f"{name} is not an instance of 'str'")
        if details is not None:
            if not isinstance(details, dict):
                raise TypeError(f"'{type(details).__name__}' object is not an instance of 'dict'")
            for key, value in details.items():
                for item in (key, value):
                    if not isinstance(item, str):
                        name = "None" if item is None else repr(type(item).__name__) + " object"
                        raise TypeError(f"{name} is not an instance of 'str'")
            details = dict(sorted(details.items()))
        Exception.__init__(self, *args)
        self._code, self._message, self._stage, self._details = code, message, stage, details

    code = property(lambda self: self._code)
    message = property(lambda self: self._message)
    stage = property(lambda self: self._stage)
    details = property(lambda self: None if self._details is None else self._details.copy())

    def __str__(self):
        return self._message


BeadComposeError.__signature__ = inspect.Signature(
    [inspect.Parameter(name, inspect.Parameter.POSITIONAL_OR_KEYWORD, default=default)
     for name, default in (("code", inspect.Parameter.empty), ("message", inspect.Parameter.empty),
                           ("stage", None), ("details", None))]
)
