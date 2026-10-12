"""Which solvers to test, and how each one is driven.

Both test modules take the `harness` fixture from here.
"""

from __future__ import annotations

import os
from pathlib import Path

import pytest

from fznso_conform import Harness

#: Where the solver configurations live. The default is the layout
#: `experiments/setup.sh` produces; override it to test other builds.
SOLVER_PATH = Path(
    os.environ.get(
        "FZNSO_SOLVER_PATH",
        Path.home() / "Documents/Papers/2027-fznso-tool-paper/experiments/deps/solvers",
    )
)

SOLVERS = os.environ.get(
    "FZNSO_SOLVERS",
    "org.minizinc.fznso.gecode,org.minizinc.fznso.huub,"
    "org.minizinc.fznso.highs,org.minizinc.fznso.gurobi,org.minizinc.fznso.z3",
).split(",")

#: Solvers to enumerate with nogoods even where they take `-a`, as a
#: comma-separated list of identifiers, or `all` for every solver.
NOGOODS = os.environ.get("FZNSO_NOGOODS", "").split(",")

@pytest.fixture(scope="session", params=SOLVERS)
def harness(request) -> Harness:
    return Harness(
        request.param,
        SOLVER_PATH,
        enumerate_with_nogoods="all" in NOGOODS or request.param in NOGOODS,
    )
