"""Every registry constraint, against every solver behind FZnSO.

A test fails when the solver's set of solutions differs from the set the
constraint's meaning allows. The report says which solutions are missing or
wrong, and whether the solver posted the constraint itself.
"""

from __future__ import annotations

import pytest

from fznso_conform import Harness, Unsupported
from fznso_conform.cases import all_cases

CASES = list(all_cases())


@pytest.mark.parametrize("case", CASES, ids=lambda c: c.name)
def test_solutions(harness: Harness, case) -> None:
    try:
        outcome = harness.run(case)
    except Unsupported as reason:
        pytest.skip(str(reason))
    assert outcome.ok, outcome.report()
