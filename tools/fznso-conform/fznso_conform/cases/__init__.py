"""The cases, one module per registry group.

A module is picked up automatically: put it in this package and expose
`CASES`, a sequence of `Case`.
"""

from __future__ import annotations

import importlib
import pkgutil
from collections.abc import Iterator

from fznso_conform.case import Case


def all_cases() -> Iterator[Case]:
    for info in sorted(pkgutil.iter_modules(__path__), key=lambda i: i.name):
        module = importlib.import_module(f"{__name__}.{info.name}")
        yield from getattr(module, "CASES", ())
