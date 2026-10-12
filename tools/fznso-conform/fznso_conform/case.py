"""What a test case is."""

from __future__ import annotations

import itertools
from collections.abc import Callable, Iterator, Sequence
from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class Decision:
    """A decision the solver assigns, and the values it may take.

    `name` is the MiniZinc identifier. `values` is the domain, which the
    harness both declares to the solver and enumerates itself. `kind` is the
    MiniZinc type: an `int`, a `bool`, or a `set of int` whose `values` are the
    sets the decision may take.
    """

    name: str
    values: Sequence[Any]
    kind: str = "int"

    def declaration(self) -> str:
        if self.kind == "bool":
            return f"var bool: {self.name};"
        if self.kind == "int":
            return f"var {{{', '.join(str(v) for v in sorted(self.values))}}}: {self.name};"
        if self.kind == "float":
            lo = min(self.values)
            hi = max(self.values)
            return f"var {lo}..{hi}: {self.name};"
        if self.kind == "set of int":
            universe = sorted({v for s in self.values for v in s})
            lo = universe[0] if universe else 0
            hi = universe[-1] if universe else 0
            return f"var set of {lo}..{hi}: {self.name};"
        raise ValueError(f"unsupported kind {self.kind!r}")


@dataclass(frozen=True)
class Case:
    """One test: a constraint, its decisions, and its meaning.

    `ident` is the registry identifier under test, which also names the file
    the predicate comes from (`fzn_<ident>.mzn`). `constraint` is the call, in
    MiniZinc. `holds` decides whether an assignment satisfies the constraint,
    in Python, and is the authority the solver is held against. `params`
    carries any parameter declarations the call needs.

    `why` says which edge case this test covers, so a failure reports it.
    """

    ident: str
    why: str
    decisions: Sequence[Decision]
    constraint: str
    holds: Callable[[dict[str, Any]], bool]
    params: str = ""
    extra_includes: Sequence[str] = field(default_factory=tuple)

    @property
    def name(self) -> str:
        return f"{self.ident}: {self.why}"

    def model(self) -> str:
        """The MiniZinc model the solver receives."""
        includes = "".join(f'include "{f}";\n' for f in (f"fzn_{self.ident}.mzn", *self.extra_includes))
        decls = "\n".join(d.declaration() for d in self.decisions)
        names = ", ".join(d.name for d in self.decisions)
        return (
            f"{includes}{self.params}\n{decls}\n"
            f"constraint {self.constraint};\n"
            f"solve satisfy;\n"
            f'output ["{names}"];\n'
        )

    def expected(self) -> set[tuple[Any, ...]]:
        """Every assignment the Python meaning allows."""
        return {
            tuple(self.key(a[d.name]) for d in self.decisions)
            for a in self.assignments()
            if self.holds(a)
        }

    def assignments(self) -> Iterator[dict[str, Any]]:
        for combination in itertools.product(*(d.values for d in self.decisions)):
            yield {d.name: v for d, v in zip(self.decisions, combination)}

    @staticmethod
    def key(value: Any) -> Any:
        """A hashable, order-independent form of a solver's answer."""
        if isinstance(value, (set, frozenset)):
            return frozenset(value)
        if isinstance(value, list):
            return tuple(Case.key(v) for v in value)
        return value
