"""Holding a solver library's declarations to the registry's types.

The registry's one rule is that a name means what the registry says it means,
so that is the only thing checked here: **every declared name the registry
defines has to be declared at the registry's types, or at a narrowing of
them.** A name the registry does not define is the solver's own, and it is
reported rather than failed.

The exception worth the code is a name that is a registry name with its words
in another order. `minimize_int` where the registry says `int_minimize` is not
a solver-specific name, it is a stale one, and by the rule above nothing else
would notice.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

import fznso

from fznso_conform.registry import Entry, Registry, Type, transposed


def read_type(declared: fznso.Type) -> Type:
    """One declared type, in the registry's own terms."""
    return Type(
        base=declared.base,
        list_of=declared.list_of,
        decision=declared.decision,
        set_of=declared.set_of,
        opt=declared.opt,
    )


@dataclass
class Report:
    """What one capability list turned out to be."""

    what: str
    conforms: int = 0
    mismatches: list[tuple[str, list[str]]] = field(default_factory=list)
    stale: list[tuple[str, str]] = field(default_factory=list)
    own: list[str] = field(default_factory=list)

    @property
    def total(self) -> int:
        return self.conforms + len(self.mismatches) + len(self.stale) + len(self.own)

    @property
    def failed(self) -> bool:
        return bool(self.mismatches or self.stale)

    def unprefixed(self, prefix: str) -> list[str]:
        """The solver's own names that do not carry its prefix.

        A solver's own name is correct by the naming rule as long as it says
        whose it is, which is the one thing worth pointing out.
        """
        return [i for i in self.own if not i.startswith(prefix)]

    def spell(self, prefix: str) -> str:
        lines = [f"{self.what}: {self.total} declared, {self.conforms} conform"]
        for ident, problems in self.mismatches:
            lines.append(f"  FAIL {ident}")
            lines.extend(f"       {p}" for p in problems)
        for ident, should_be in self.stale:
            lines.append(f"  FAIL {ident} — the registry spells this `{should_be}`")
        loose = self.unprefixed(prefix)
        if loose:
            lines.append(
                f"  {len(loose)} name(s) are neither in the registry nor prefixed `{prefix}`:"
            )
            lines.extend(f"       {'  '.join(loose[i:i + 4])}" for i in range(0, len(loose), 4))
        return "\n".join(lines)


def _check(ident: str, declared: list[Type], defined: dict[str, Entry],
           by_words: dict[tuple[str, ...], str]) -> tuple[str, list[str]]:
    """Compare one declaration against the registry.

    Returns the verdict and, for a mismatch, what is wrong with it.
    """
    entry = defined.get(ident)
    if entry is None:
        name = by_words.get(transposed(ident))
        return ("stale", [name]) if name is not None and name != ident else ("own", [])
    if len(declared) != len(entry.args):
        return "mismatch", [
            f"takes {len(declared)} argument(s), the registry defines {len(entry.args)}"
        ]
    problems = []
    for i, (got, want) in enumerate(zip(declared, entry.args), start=1):
        if not got.narrows(want):
            widening = (
                " (widening; only narrowing is allowed)"
                if (got.base, got.list_of, got.set_of) == (want.base, want.list_of, want.set_of)
                else ""
            )
            problems.append(
                f"argument {i}: declared `{got}`, registry says `{want}`{widening}"
            )
    return ("mismatch", problems) if problems else ("conforms", [])


def _report(what: str, declarations: list[tuple[str, list[Type]]],
            defined: dict[str, Entry]) -> Report:
    by_words = {transposed(ident): ident for ident in defined}
    report = Report(what)
    for ident, types in declarations:
        verdict, detail = _check(ident, types, defined, by_words)
        if verdict == "conforms":
            report.conforms += 1
        elif verdict == "mismatch":
            report.mismatches.append((ident, detail))
        elif verdict == "stale":
            report.stale.append((ident, detail[0]))
        else:
            report.own.append(ident)
    return report


def check(library: Path, registry: Registry) -> list[Report]:
    """Hold every declaration in `library` to `registry`.

    One report per capability list. Message scopes have no list, so they have
    no report.
    """
    lib = fznso.Library(str(library))
    return [
        _report(
            "Constraints",
            [(c.ident, [read_type(t) for t in c.argument_types]) for c in lib.constraint_types()],
            registry.constraints,
        ),
        _report(
            "Objectives",
            [(o.ident, [read_type(o.argument_type)]) for o in lib.objectives()],
            registry.objectives,
        ),
        _report(
            "Options",
            [(o.ident, [read_type(o.argument_type)]) for o in lib.options()],
            registry.options,
        ),
        _report(
            "Statistics",
            [(s.ident, [read_type(s.value_type)]) for s in lib.statistics()],
            registry.statistics,
        ),
    ]
