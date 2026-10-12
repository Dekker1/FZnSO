"""Hold a solver to the registry, in what it declares and in what it answers.

    python -m fznso_conform types <solver>
    python -m fznso_conform solutions <solver>

`<solver>` is a MiniZinc solver configuration (`.msc`) or the solver library
itself.

`types` reads the solver's capability lists through the ABI and holds every
registry name it declares to the registry's types. It needs no compiler and no
solve.

`solutions` states each registry constraint in Python and checks that the
solutions the solver permits are exactly the ones that meaning allows. It
drives the solver through MiniZinc, so it needs a MiniZinc build.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

from fznso_conform import target
from fznso_conform.harness import Harness
from fznso_conform.registry import Registry
from fznso_conform.signatures import check


def types(solver: target.Target, registry: Path) -> int:
    reports = check(solver.library, Registry.load(registry))
    print(f"{solver.name} — checked against the registry in {registry}")
    failed = False
    for report in reports:
        print()
        print(report.spell(f"{solver.name}_"))
        failed |= report.failed
    return 1 if failed else 0


def solutions(solver: target.Target, registry: Path) -> int:
    import pytest

    if solver.synthesized:
        print(
            f"note: no configuration was given for {solver.library}, so the solver's own "
            "MiniZinc library, if it has one, is not on the include path",
            file=sys.stderr,
        )
    os.environ["FZNSO_SOLVERS"] = solver.solver_id
    os.environ["FZNSO_SOLVER_PATH"] = str(solver.solver_path)
    os.environ["FZNSO_REGISTRY"] = str(registry)
    here = Path(__file__).resolve().parent.parent
    return pytest.main(["-q", str(here / "tests/test_registry.py")])


def main(argv: list[str]) -> int:
    if len(argv) < 3 or argv[1] not in ("types", "solutions"):
        print(__doc__, file=sys.stderr)
        return 2
    solver = target.read(argv[2])
    registry = Path(argv[3]) if len(argv) > 3 else Harness.registry_dir()
    return (types if argv[1] == "types" else solutions)(solver, registry)


def main_argv() -> int:
    """The `fznso-conform` console script."""
    return main(sys.argv)


if __name__ == "__main__":
    raise SystemExit(main_argv())
