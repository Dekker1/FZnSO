"""Integration tests for solvers behind FZnSO.

A test states a constraint, the decisions it ranges over, and what the
constraint means, in Python. The harness asks the solver for every solution and
compares that set with the one the Python meaning allows. A solver that
declares the constraint answers with its own propagator; one that does not gets
the registry's decomposition, so the same test covers both.
"""

from fznso_conform.case import Case, Decision
from fznso_conform.harness import Harness, Outcome, Unsupported

__all__ = ["Case", "Decision", "Harness", "Outcome", "Unsupported"]
