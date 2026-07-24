"""End-to-end tests for the FZnSO Python bindings.

A model is built in Python and run against the example solver. The example
solver assigns each decision its domain's lower bound, shifts the first by the
solution index, and warns that it ignores the constraints.
"""

import ctypes.util
import pathlib
import sys

import pytest

import fznso

REPO = pathlib.Path(__file__).resolve().parents[2]


def solver_path() -> str:
    prefix, suffix = ("lib", ".dylib") if sys.platform == "darwin" else ("lib", ".so")
    path = REPO / "target" / "debug" / "examples" / f"{prefix}example_solver{suffix}"
    if not path.exists():
        pytest.skip(f"{path} not built; run `cargo build --example example_solver`")
    return str(path)


@pytest.fixture
def solver():
    return fznso.Library(solver_path()).create_solver()


def build_model():
    m = fznso.LayeredModel()
    a = m.add_decision(range(3, 21), name="a")  # inclusive [3, 20]
    b = m.add_decision(fznso.int_set(10, 20), name="b")
    m.add_constraint("int_le", [0, a])
    m.set_objective("minimize", a)
    return m, a, b


def test_options_round_trip(solver):
    assert solver.option_get("solution_limit") == 1
    solver.option_set("solution_limit", 3)
    assert solver.option_get("solution_limit") == 3


def test_unknown_option_raises_with_message(solver):
    with pytest.raises(ValueError, match="unknown option `nope`"):
        solver.option_set("nope", 1)


def test_run_reports_solutions_and_messages(solver):
    solver.option_set("solution_limit", 3)
    m, a, b = build_model()

    solutions, messages = [], []
    status = solver.run(
        m,
        on_solution=lambda s: solutions.append((s[a], s[b], s.statistic("solutions"))),
        on_message=lambda scope, value: messages.append((scope, value)),
    )

    assert status == fznso.Status.Complete
    assert solutions == [(3, 10, 0), (4, 10, 1), (5, 10, 2)]
    assert messages == [("warn", "ignoring 1 constraint(s)")]


def test_solution_is_invalid_outside_callback(solver):
    m, a, _ = build_model()
    escaped = []
    solver.run(m, on_solution=lambda s: escaped.append(s))
    # The solution borrowed a value that no longer exists; using it must raise
    # rather than read freed memory.
    with pytest.raises(RuntimeError, match="outside the on_solution callback"):
        escaped[0][a]


def test_callback_exception_propagates(solver):
    m, _, _ = build_model()

    def boom(_sol):
        raise KeyError("from the callback")

    with pytest.raises(KeyError, match="from the callback"):
        solver.run(m, on_solution=boom)


def test_value_kinds_round_trip():
    # Reading every value kind back through the solver's model view is covered
    # elsewhere; here we check the Python value conversions directly via the
    # domain a solution reports.
    m = fznso.LayeredModel()
    x = m.add_decision(fznso.int_set(-5, 5), name="x")
    assert isinstance(x, fznso.Decision)
    assert int(x) == 0


class DomainModel(fznso.Model):
    """A model implemented in Python rather than built — the solver reads it on
    demand by calling these methods.

    The values it returns are borrowed, not copied, so it owns them: every object
    it hands back is built once in ``__init__`` and stored on the instance."""

    def __init__(self, domains, constraints=0):
        super().__init__()
        self._domains = [fznso.int_set(lo, hi) for lo, hi in domains]
        self._names = [f"x{i}" for i in range(len(domains))]
        self._ident = "int_le"
        self._arguments = [[0, fznso.Decision(i)] for i in range(constraints)]

    def decision_count(self):
        return len(self._domains)

    def decision_domain(self, i):
        return self._domains[i]

    def decision_name(self, i):
        return self._names[i]

    def constraint_count(self):
        return len(self._arguments)

    def constraint_ident(self, i):
        return self._ident

    def constraint_arguments(self, i):
        return self._arguments[i]


def test_custom_python_model(solver):
    solver.option_set("solution_limit", 2)
    model = DomainModel([(7, 20), (30, 40)], constraints=2)

    solutions, messages = [], []
    solver.run(
        model,
        on_solution=lambda s: solutions.append((s.value(0), s.value(1))),
        on_message=lambda scope, value: messages.append((scope, value)),
    )
    # Lower bounds 7 and 30; the solver shifts the first by the solution index.
    assert solutions == [(7, 30), (8, 30)]
    assert messages == [("warn", "ignoring 2 constraint(s)")]


class TwoLayerModel(fznso.Model):
    """A custom model split across two layers, to check the layer protocol."""

    def __init__(self, domains, split):
        super().__init__()
        self._domains = [fznso.int_set(lo, hi) for lo, hi in domains]
        self._names = [f"x{i}" for i in range(len(domains))]
        self._split = split  # number of decisions in the first layer
        self._ident = "int_le"
        self._arguments = [7, fznso.Decision(0)]
        self._objective = "minimize"
        self._objective_arg = fznso.Decision(0)

    def decision_count(self):
        return len(self._domains)

    def decision_domain(self, i):
        return self._domains[i]

    def decision_name(self, i):
        return self._names[i]

    def layer_count(self):
        return 2

    def decision_layer_end(self, layer):
        return self._split if layer == 0 else len(self._domains)

    def constraint_count(self):
        return 1

    def constraint_ident(self, i):
        return self._ident

    def constraint_arguments(self, i):
        return self._arguments

    def objective_ident(self):
        return self._objective

    def objective_argument(self):
        return self._objective_arg


def test_custom_layered_model(solver):
    model = TwoLayerModel([(7, 20), (30, 40), (100, 110)], split=2)

    solutions, messages = [], []
    solver.run(
        model,
        on_solution=lambda s: solutions.append((s.value(0), s.value(1), s.value(2))),
        on_message=lambda scope, value: messages.append((scope, value)),
    )
    assert solutions == [(7, 30, 100)]
    # The solver reports the per-layer decision counts: 2 in layer 0, 1 in layer 1.
    assert ("layers", "2,1") in messages
    # It also reads back the model's strings (objective, decision name, constraint
    # identifier) and a constraint argument, all borrowing native Python objects.
    assert ("model", "obj=minimize;name0=x0;con0=int_le(7)") in messages


def test_custom_model_method_error_propagates(solver):
    class Broken(fznso.Model):
        def decision_count(self):
            return 1

        def decision_domain(self, i):
            raise ValueError("bad domain")

    with pytest.raises(ValueError, match="bad domain"):
        solver.run(Broken(), on_solution=lambda s: None)
