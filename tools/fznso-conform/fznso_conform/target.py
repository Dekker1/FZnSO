"""Naming the solver to check, as a configuration or as a library.

Both commands take one argument, which is either a MiniZinc solver
configuration (`.msc`) or the solver library itself. A configuration says
everything: which library to load, and which MiniZinc library the solver ships.
A bare library says only itself, so one is written for it.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from tempfile import mkdtemp


@dataclass(frozen=True)
class Target:
    """One solver, however it was named."""

    #: The library the solver is in, which is what the ABI is read from.
    library: Path
    #: The configuration identifier, for MiniZinc.
    solver_id: str
    #: The directory the configuration is in, for `MZN_SOLVER_PATH`.
    solver_path: Path
    #: Whether the configuration was written here rather than given.
    synthesized: bool

    @property
    def name(self) -> str:
        """The solver's own name, which its prefixed identifiers start with.

        It comes from the library file, which is where the entry-point prefix
        comes from too. Everything from the first `.` on is the extension and
        any version, so `libgecode.6.4.dylib` is named `gecode`.
        """
        return self.library.name.removeprefix("lib").split(".", 1)[0]


def read(argument: str | Path) -> Target:
    """Read one command-line argument as a solver.

    A `.msc` is used as it stands. Anything else is taken for a library, and a
    configuration naming it is written to a temporary directory. That
    configuration carries no `mznlib`, because a library does not say whether
    the solver ships one: the solutions check then exercises the registry's own
    decompositions rather than the solver's overrides.
    """
    path = Path(argument).resolve()
    if path.suffix == ".msc":
        config = json.loads(path.read_text())
        return Target(
            library=Path(config["executable"]),
            solver_id=config["id"],
            solver_path=path.parent,
            synthesized=False,
        )

    directory = Path(mkdtemp(prefix="fznso-conform-"))
    name = path.name.removeprefix("lib").split(".", 1)[0]
    solver_id = f"org.minizinc.fznso.{name}"
    (directory / f"{name}.msc").write_text(
        json.dumps(
            {
                "id": solver_id,
                "name": f"{name} (FZnSO)",
                "version": "fznso",
                "inputType": "FZNSO",
                "executable": str(path),
            },
            indent=2,
        )
        + "\n"
    )
    return Target(library=path, solver_id=solver_id, solver_path=directory, synthesized=True)
