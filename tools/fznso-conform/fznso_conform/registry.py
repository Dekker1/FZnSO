"""Reading the registry TOMLs.

The registry is prose and tables meant for people, so this reads only the two
fields a conformance check needs, the identifier and the types, and ignores
everything the documentation generator cares about.
"""

from __future__ import annotations

import tomllib
from dataclasses import dataclass, field
from pathlib import Path


@dataclass(frozen=True)
class Type:
    """A type as the registry spells it."""

    base: str
    list_of: bool = False
    decision: bool = False
    set_of: bool = False
    opt: bool = False

    def __str__(self) -> str:
        """How the type reads in the registry's own notation."""
        out = ""
        if self.list_of:
            out += "list of "
        if self.decision:
            out += "var "
        if self.opt:
            out += "opt "
        if self.set_of:
            out += "set of "
        return out + self.base

    def narrows(self, expected: Type) -> bool:
        """Whether this is `expected` or a narrowing of it.

        Narrowing restricts what a name accepts without changing what it means:
        a solver that takes only a fixed set where the registry allows a
        variable one has declared the same constraint, restricted to what its
        propagator does. The base type and the shape may not move at all, and
        widening is never allowed, because an application reading the registry
        would have no way to know the extra cases existed.
        """
        return (
            self.base == expected.base
            and self.list_of == expected.list_of
            and self.set_of == expected.set_of
            and (self.decision == expected.decision or not self.decision)
            and (self.opt == expected.opt or not self.opt)
        )


#: The base types the registry uses. The set is closed, so an unrecognized
#: spelling is an error rather than a default: reading a new one as `int` would
#: make the check pass for the wrong reason.
BASES = ("int", "bool", "float", "string")


def parse_type(spelling: str) -> Type:
    """Read a type as the registry spells it.

    The qualifiers are stripped outermost first, in the order the registry
    writes them, so `list of var set of int` is a list of variable sets.
    """
    rest = spelling.strip()
    flags = {}
    for prefix, name in (
        ("list of ", "list_of"),
        ("var ", "decision"),
        ("opt ", "opt"),
        ("set of ", "set_of"),
    ):
        if rest.startswith(prefix):
            flags[name] = True
            rest = rest[len(prefix):]
    if rest not in BASES:
        raise ValueError(f"unknown base type `{rest}` in `{spelling}`")
    return Type(base=rest, **flags)


def _arg_type(arg: str) -> Type:
    """Split `"type: name"`, which is how the registry writes an argument."""
    # Categories other than constraints give a bare type with no name.
    return parse_type(arg.split(": ", 1)[0] if ": " in arg else arg)


@dataclass
class Entry:
    """One name the registry defines, with the types it defines it at."""

    source: str
    args: list[Type] = field(default_factory=list)


#: The Boolean a reified or half-reified form takes last. The registry leaves it
#: implicit, so it is appended rather than read.
REIFYING = Type(base="bool", decision=True)


def _read(path: Path, types_field: str, with_reified: bool) -> dict[str, Entry]:
    """Read one file's `[[entry]]` blocks, taking the types from `types_field`.

    The field differs by category: constraints give an array of `args`,
    objectives a single `arg`, options and statistics a `type`.
    """
    doc = tomllib.loads(path.read_text())
    out: dict[str, Entry] = {}
    for entry in doc.get("entry", ()):
        ident = entry.get("ident")
        if ident is None:
            continue
        raw = entry.get(types_field)
        if isinstance(raw, list):
            args = [_arg_type(a) for a in raw]
        elif isinstance(raw, str):
            args = [_arg_type(raw)]
        else:
            # A category that types nothing still contributes its name.
            args = []
        out[ident] = Entry(source=path.name, args=args)
        # Every constraint names a reified and a half-reified form, unless the
        # entry marks itself `functional`, one that computes a value rather
        # than checking a relation. The names follow from the base entry, so
        # they are derived rather than read.
        if with_reified and not entry.get("functional", False):
            for suffix in ("_reif", "_imp"):
                out[ident + suffix] = Entry(source=path.name, args=args + [REIFYING])
    return out


@dataclass
class Registry:
    """Every name the registry defines, across the four declared categories."""

    constraints: dict[str, Entry]
    objectives: dict[str, Entry]
    options: dict[str, Entry]
    statistics: dict[str, Entry]

    @classmethod
    def load(cls, root: Path) -> Registry:
        constraints: dict[str, Entry] = {}
        for path in sorted((root / "constraints").glob("*.toml")):
            constraints.update(_read(path, "args", True))
        # Message scopes are deliberately absent: they are the one registry
        # vocabulary a solver does not declare up front, so there is no
        # capability list to check them against.
        return cls(
            constraints=constraints,
            objectives=_read(root / "objectives.toml", "arg", False),
            options=_read(root / "options.toml", "type", False),
            statistics=_read(root / "statistics.toml", "type", False),
        )


def transposed(ident: str) -> tuple[str, ...]:
    """The words of `ident`, sorted, so a name in another order is findable.

    The registry names a constraint for the type it takes and puts that type
    first, so a name that drifted from an older scheme, `array_int_element` or
    `minimize_int`, is usually the right entry with its words transposed. That
    is worth saying out loud, because such a name is not a new constraint but a
    stale one, and nothing else distinguishes the two.
    """
    return tuple(sorted(ident.split("_")))
